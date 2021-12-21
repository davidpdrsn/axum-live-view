use crate::{
    html::Html,
    pubsub::{Decode, PubSub},
};
use async_stream::stream;
use axum::http::Uri;
use bytes::Bytes;
use futures_util::{
    future::{BoxFuture, FutureExt},
    stream::StreamExt,
    Stream,
};
use std::sync::Arc;
use std::{
    any::TypeId,
    future::{ready, Future},
    hash::Hash,
};
use tokio_stream::StreamMap;
use uuid::Uuid;

// ---- LiveView ----

pub trait LiveView: Sized + Send + Sync + 'static {
    fn setup(&self, sub: &mut Setup<Self>);

    fn render(&self) -> Html;
}

pub struct RenderResult<T> {
    kind: Kind<T>,
}

pub(crate) enum Kind<T> {
    Render(T),
    DontRender(T),
    NavigateTo(Uri),
}

impl<T> RenderResult<T> {
    pub fn render(value: T) -> Self {
        Self {
            kind: Kind::Render(value),
        }
    }

    pub fn dont_render(value: T) -> Self {
        Self {
            kind: Kind::DontRender(value),
        }
    }

    pub fn navigate_to(uri: Uri) -> Self {
        Self {
            kind: Kind::NavigateTo(uri),
        }
    }
}

impl<T> From<T> for RenderResult<T> {
    fn from(value: T) -> Self {
        Self::render(value)
    }
}

pub(crate) async fn run_to_stream<T, P>(
    mut liveview: T,
    pubsub: P,
    liveview_id: Uuid,
) -> impl Stream<Item = LiveViewStreamItem> + Send
where
    T: LiveView,
    P: PubSub,
{
    let mut setup = Setup::new();
    liveview.setup(&mut setup);

    let mut stream_map = StreamMap::new();
    for (topic, callback) in setup.subscriptions {
        let stream = match topic {
            SubscriptionKind::Local(topic) => {
                pubsub
                    .subscribe_raw(&topics::liveview_local(liveview_id, &topic))
                    .await
            }
            SubscriptionKind::Broadcast(topic) => pubsub.subscribe_raw(&topic).await,
        };
        stream_map.insert(callback, stream);
    }

    stream! {
        while let Some((callback, msg)) = stream_map.next().await {
            let result = (callback.callback)(liveview, msg).await;
            liveview = match result.kind {
                Kind::Render(liveview) => {
                    let markup = liveview.render();
                    yield LiveViewStreamItem::Html(markup);
                    liveview
                }
                Kind::DontRender(liveview) => liveview,
                Kind::NavigateTo(uri) => {
                    yield LiveViewStreamItem::NavigateTo(uri);
                    break;
                }
            };
        }
    }
}

pub(crate) enum LiveViewStreamItem {
    Html(Html),
    NavigateTo(Uri),
}

pub(crate) mod topics {
    use uuid::Uuid;

    pub(crate) fn mounted(liveview_id: Uuid) -> String {
        liveview_local(liveview_id, "mounted")
    }

    pub(crate) fn initial_render(liveview_id: Uuid) -> String {
        liveview_local(liveview_id, "initial-render")
    }

    pub(crate) fn rendered(liveview_id: Uuid) -> String {
        liveview_local(liveview_id, "rendered")
    }

    pub(crate) fn js_command(liveview_id: Uuid) -> String {
        liveview_local(liveview_id, "js-command")
    }

    pub(crate) fn socket_disconnected(liveview_id: Uuid) -> String {
        liveview_local(liveview_id, "socket-disconnected")
    }

    pub(crate) fn liveview_local(liveview_id: Uuid, topic: &str) -> String {
        format!("liveview/{}/{}", liveview_id, topic)
    }
}

pub struct Setup<T> {
    subscriptions: Vec<(SubscriptionKind, AsyncCallback<T>)>,
}

#[derive(Clone)]
enum SubscriptionKind {
    Local(String),
    Broadcast(String),
}

impl<T> Setup<T> {
    fn new() -> Self {
        Self {
            subscriptions: Default::default(),
        }
    }

    pub fn on<F, Msg>(&mut self, topic: &str, callback: F)
    where
        F: SubscriptionCallback<T, Msg>,
        T: Send + 'static,
        Msg: Decode,
    {
        self.on_kind(SubscriptionKind::Local(topic.to_owned()), callback)
    }

    pub fn on_broadcast<F, Msg>(&mut self, topic: &str, callback: F)
    where
        F: SubscriptionCallback<T, Msg>,
        T: Send + 'static,
        Msg: Decode,
    {
        self.on_kind(SubscriptionKind::Broadcast(topic.to_owned()), callback)
    }

    fn on_kind<F, Msg>(&mut self, kind: SubscriptionKind, callback: F)
    where
        F: SubscriptionCallback<T, Msg>,
        T: Send + 'static,
        Msg: Decode,
    {
        let callback = Arc::new(
            move |receiver: T, raw_msg: Bytes| match Msg::decode(raw_msg) {
                Ok(msg) => Box::pin(callback.call(receiver, msg).map(|value| value.into())) as _,
                Err(err) => {
                    tracing::warn!(?err, "failed to decode message for subscriber");
                    Box::pin(ready(RenderResult::dont_render(receiver))) as _
                }
            },
        );
        let topic: Arc<str> = match kind.clone() {
            SubscriptionKind::Local(topic) => topic.into(),
            SubscriptionKind::Broadcast(topic) => topic.into(),
        };
        self.subscriptions.push((
            kind,
            AsyncCallback {
                type_id: TypeId::of::<F>(),
                topic,
                callback,
            },
        ));
    }
}

pub trait SubscriptionCallback<T, Msg>: Copy + Send + Sync + 'static {
    type Output: Into<RenderResult<T>>;
    type Future: Future<Output = Self::Output> + Send + 'static;

    fn call(self, receiver: T, input: Msg) -> Self::Future;
}

impl<T, F, Fut, K> SubscriptionCallback<T, ()> for F
where
    F: Fn(T) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = K> + Send + 'static,
    K: Into<RenderResult<T>>,
{
    type Output = K;
    type Future = Fut;

    fn call(self, receiver: T, _: ()) -> Self::Future {
        self(receiver)
    }
}

impl<T, Msg, F, Fut, K> SubscriptionCallback<T, (Msg,)> for F
where
    F: Fn(T, Msg) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = K> + Send + 'static,
    K: Into<RenderResult<T>>,
    Msg: Decode,
{
    type Output = K;
    type Future = Fut;

    fn call(self, receiver: T, (input,): (Msg,)) -> Self::Future {
        self(receiver, input)
    }
}

struct AsyncCallback<T> {
    type_id: TypeId,
    topic: Arc<str>,
    callback: Arc<dyn Fn(T, Bytes) -> BoxFuture<'static, RenderResult<T>> + Send + Sync + 'static>,
}

impl<T> Clone for AsyncCallback<T> {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            topic: self.topic.clone(),
            callback: self.callback.clone(),
        }
    }
}

impl<T> Hash for AsyncCallback<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.topic.hash(state);
    }
}

impl<T> PartialEq for AsyncCallback<T> {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.topic == other.topic
    }
}

impl<T> Eq for AsyncCallback<T> {}
