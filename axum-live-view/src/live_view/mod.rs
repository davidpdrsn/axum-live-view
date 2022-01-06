use crate::{event_data::EventData, html::Html, js_command::JsCommand};
use axum::{
    async_trait,
    http::{HeaderMap, Uri},
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

mod combine;

pub use self::combine::combine;

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;
    type Error: fmt::Display;

    async fn mount(&mut self, _uri: Uri, _request_headers: &HeaderMap) -> Result<(), Self::Error> {
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
        E2: std::fmt::Display + Send + Sync + 'static,
    {
        assert_live_view::<_, Self::Message, E2>(MapErr { view: self, f })
    }
}

#[derive(Debug, Clone)]
pub struct Updated<T> {
    pub(crate) live_view: T,
    pub(crate) js_commands: Vec<JsCommand>,
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

    pub fn into_parts(self) -> (T, Vec<JsCommand>) {
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
    E2: std::fmt::Display + Send + Sync + 'static,
{
    type Message = V::Message;
    type Error = E2;

    async fn mount(&mut self, uri: Uri, request_headers: &HeaderMap) -> Result<(), Self::Error> {
        self.view.mount(uri, request_headers).await.map_err(&self.f)
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
