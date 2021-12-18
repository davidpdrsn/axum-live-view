use crate::{
    html::Html,
    pubsub::{Message, PubSub},
};
use async_stream::stream;
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
    fn setup(&self, sub: &mut Subscriptions<Self>);

    fn render(&self) -> Html;
}

pub enum ShouldRender<T> {
    Yes(T),
    No(T),
}

impl<T> From<T> for ShouldRender<T> {
    fn from(value: T) -> Self {
        Self::Yes(value)
    }
}

pub(crate) async fn run_to_stream<T, P>(
    mut liveview: T,
    pubsub: P,
    liveview_id: Uuid,
) -> impl Stream<Item = Html> + Send
where
    T: LiveView,
    P: PubSub,
{
    let mut subscriptions = Subscriptions::new();
    liveview.setup(&mut subscriptions);

    let mut stream_map = StreamMap::new();
    for (topic, callback) in subscriptions.handlers {
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
            liveview = match (callback.callback)(liveview, msg).await {
                ShouldRender::Yes(liveview) => {
                    let markup = liveview.render();
                    yield markup;
                    liveview
                }
                ShouldRender::No(liveview) => liveview,
            };
        }
    }
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

    pub(crate) fn socket_disconnected(liveview_id: Uuid) -> String {
        liveview_local(liveview_id, "socket-disconnected")
    }

    pub(crate) fn liveview_local(liveview_id: Uuid, topic: &str) -> String {
        format!("liveview/{}/{}", liveview_id, topic)
    }
}

// ---- Subscriptions ----

pub struct Subscriptions<T> {
    handlers: Vec<(SubscriptionKind, AsyncCallback<T>)>,
}

#[derive(Clone)]
enum SubscriptionKind {
    Local(String),
    Broadcast(String),
}

impl<T> Subscriptions<T> {
    fn new() -> Self {
        Self {
            handlers: Default::default(),
        }
    }

    pub fn on<F, Msg>(&mut self, topic: &str, callback: F) -> &mut Self
    where
        F: SubscriptionCallback<T, Msg>,
        T: Send + 'static,
        Msg: Message,
    {
        self.on_kind(SubscriptionKind::Local(topic.to_owned()), callback)
    }

    pub fn on_broadcast<F, Msg>(&mut self, topic: &str, callback: F) -> &mut Self
    where
        F: SubscriptionCallback<T, Msg>,
        T: Send + 'static,
        Msg: Message,
    {
        self.on_kind(SubscriptionKind::Broadcast(topic.to_owned()), callback)
    }

    fn on_kind<F, Msg>(&mut self, kind: SubscriptionKind, callback: F) -> &mut Self
    where
        F: SubscriptionCallback<T, Msg>,
        T: Send + 'static,
        Msg: Message,
    {
        let callback = Arc::new(
            move |receiver: T, raw_msg: Bytes| match Msg::decode(raw_msg) {
                Ok(msg) => Box::pin(callback.call(receiver, msg).map(|value| value.into())) as _,
                Err(err) => {
                    tracing::warn!(?err, "failed to decode message for subscriber");
                    Box::pin(ready(ShouldRender::No(receiver))) as _
                }
            },
        );
        let topic: Arc<str> = match kind.clone() {
            SubscriptionKind::Local(topic) => topic.into(),
            SubscriptionKind::Broadcast(topic) => topic.into(),
        };
        self.handlers.push((
            kind,
            AsyncCallback {
                type_id: TypeId::of::<F>(),
                topic,
                callback,
            },
        ));
        self
    }
}

pub trait SubscriptionCallback<T, Msg>: Copy + Send + Sync + 'static {
    type Output: Into<ShouldRender<T>>;
    type Future: Future<Output = Self::Output> + Send + 'static;

    fn call(self, receiver: T, input: Msg) -> Self::Future;
}

impl<T, F, Fut, K> SubscriptionCallback<T, ()> for F
where
    F: Fn(T) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = K> + Send + 'static,
    K: Into<ShouldRender<T>>,
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
    K: Into<ShouldRender<T>>,
    Msg: Message,
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
    callback: Arc<dyn Fn(T, Bytes) -> BoxFuture<'static, ShouldRender<T>> + Send + Sync + 'static>,
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
