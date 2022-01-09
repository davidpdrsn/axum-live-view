use crate::{event_data::EventData, html::Html, js_command::JsCommand};
use axum::{
    async_trait,
    http::{HeaderMap, Uri},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, time::Instant};
use tokio::sync::mpsc;

mod combine;

pub use self::combine::combine;

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;
    type Error: fmt::Display + Send + Sync + 'static;

    async fn mount(
        &mut self,
        _uri: Uri,
        _request_headers: &HeaderMap,
        _handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn update(
        self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error>;

    fn render(&self) -> Html<Self::Message>;

    fn map_err<F, E2>(self, f: F) -> MapErr<Self, F>
    where
        F: Fn(Self::Error) -> E2 + Send + Sync + 'static,
        E2: fmt::Display + Send + Sync + 'static,
    {
        assert_live_view::<_, Self::Message, E2>(MapErr { view: self, f })
    }
}

#[derive(Debug, Clone)]
pub struct Updated<T> {
    live_view: T,
    js_commands: Vec<JsCommand>,
}

impl<T> Updated<T> {
    pub fn new(live_view: T) -> Self {
        Self {
            live_view,
            js_commands: Default::default(),
        }
    }

    pub fn with(mut self, js_command: JsCommand) -> Self {
        self.js_commands.push(js_command);
        self
    }

    pub fn with_all<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.extend(commands);
        self
    }

    pub fn map<F, K>(self, f: F) -> Updated<K>
    where
        F: FnOnce(T) -> K,
    {
        Updated {
            live_view: f(self.live_view),
            js_commands: self.js_commands,
        }
    }

    pub(crate) fn into_parts(self) -> (T, Vec<JsCommand>) {
        (self.live_view, self.js_commands)
    }
}

impl<T> Extend<JsCommand> for Updated<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.js_commands.extend(iter);
    }
}

#[inline]
fn assert_live_view<V, M, E>(v: V) -> V
where
    V: LiveView<Message = M, Error = E>,
{
    v
}

pub struct MapErr<V, F> {
    view: V,
    f: F,
}

impl<V, F> fmt::Debug for MapErr<V, F>
where
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapErr")
            .field("view", &self.view)
            .field("f", &format_args!("{}", std::any::type_name::<F>()))
            .finish()
    }
}

#[async_trait]
impl<V, F, E2> LiveView for MapErr<V, F>
where
    V: LiveView,
    F: Fn(V::Error) -> E2 + Send + Sync + 'static,
    E2: fmt::Display + Send + Sync + 'static,
{
    type Message = V::Message;
    type Error = E2;

    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        self.view
            .mount(uri, request_headers, handle)
            .await
            .map_err(&self.f)
    }

    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        let (view, cmds) = self
            .view
            .update(msg, data)
            .await
            .map_err(&self.f)?
            .into_parts();
        self.view = view;
        Ok(Updated::new(self).with_all(cmds))
    }

    fn render(&self) -> Html<Self::Message> {
        self.view.render()
    }
}

pub struct ViewHandle<M> {
    tx: mpsc::Sender<M>,
}

impl<M> ViewHandle<M> {
    pub(crate) fn new() -> (Self, mpsc::Receiver<M>) {
        let (tx, rx) = mpsc::channel(1024);
        (Self { tx }, rx)
    }

    pub async fn send(&self, msg: M) -> Result<(), ViewHandleSendError> {
        self.tx.send(msg).await.map_err(|_| ViewHandleSendError)?;
        Ok(())
    }

    pub(crate) fn with<F, M2>(self, f: F) -> ViewHandle<M2>
    where
        F: Fn(M2) -> M + Send + Sync + 'static,
        M2: Send + 'static,
        M: Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<M2>(1024);
        let old_tx = self.tx;

        // probably not the most effecient thing to spawn here
        // might be worth moving to using a `Sink` and using `SinkExt::with`
        // will probably require boxing since `ViewHandle` should only
        // be generic over the message
        crate::util::spawn_unit(async move {
            while let Some(msg) = rx.recv().await {
                if old_tx.send(f(msg)).await.is_err() {
                    break;
                }
            }
        });

        ViewHandle { tx }
    }
}

impl<M> Clone for ViewHandle<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M> fmt::Debug for ViewHandle<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewHandle").finish()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ViewHandleSendError;

impl fmt::Display for ViewHandleSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to send message to view")
    }
}

impl std::error::Error for ViewHandleSendError {}

// useful for debugging how long time mount, update, and render takes
#[allow(dead_code)]
pub(crate) struct Logging<V>(pub(crate) V);

#[async_trait]
impl<V> LiveView for Logging<V>
where
    V: LiveView,
{
    type Message = V::Message;
    type Error = V::Error;

    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let start = Instant::now();
        let result = self.0.mount(uri, request_headers, handle).await;
        tracing::trace!("mount finished in {:?}", start.elapsed());
        result
    }

    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        let start = Instant::now();
        let result = self
            .0
            .update(msg, data)
            .await
            .map(|updated| updated.map(Self));
        tracing::trace!("update finished in {:?}", start.elapsed());
        result
    }

    fn render(&self) -> Html<Self::Message> {
        let start = Instant::now();
        let result = self.0.render();
        tracing::trace!("render finished in {:?}", start.elapsed());
        result
    }
}
