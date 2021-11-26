use crate::pubsub::PubSub;
use crate::Codec;
use async_stream::stream;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{
    future::{BoxFuture, FutureExt},
    stream::{FuturesUnordered, StreamExt},
    Stream,
};
use maud::Markup;
use std::sync::Arc;
use std::{
    any::TypeId,
    future::{ready, Future},
    hash::Hash,
};
use tokio_stream::StreamMap;

// ---- LiveView ----

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    fn setup(sub: &mut Subscriptions<Self>);

    fn render(&self) -> Markup;
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
) -> impl Stream<Item = Markup> + Send
where
    T: LiveView,
    P: PubSub,
{
    let mut subscriptions = Subscriptions::new();
    T::setup(&mut subscriptions);

    let mut stream_map = subscriptions
        .handlers
        .into_iter()
        .map(|(topic, callback)| pubsub.subscribe(&topic).map(|stream| (callback, stream)))
        .collect::<FuturesUnordered<_>>()
        .collect::<StreamMap<_, _>>()
        .await;

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

// ---- Subscriptions ----

pub struct Subscriptions<T> {
    handlers: Vec<(String, AsyncCallback<T>)>,
}

impl<T> Subscriptions<T> {
    fn new() -> Self {
        Self {
            handlers: Default::default(),
        }
    }

    pub fn on<F, Fut, Msg, K>(&mut self, topic: &'static str, callback: F) -> &mut Self
    where
        T: Send + 'static,
        F: Fn(T, Msg) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = K> + Send + 'static,
        K: Into<ShouldRender<T>>,
        Msg: Codec,
    {
        let callback = Arc::new(
            move |receiver: T, raw_msg: Bytes| match Msg::decode(raw_msg) {
                Ok(msg) => Box::pin(callback(receiver, msg).map(|value| value.into())) as _,
                // TODO(david): handle error someshow
                Err(_err) => Box::pin(ready(ShouldRender::No(receiver))) as _,
            },
        );
        self.handlers.push((
            topic.to_owned(),
            AsyncCallback {
                type_id: TypeId::of::<F>(),
                callback,
            },
        ));
        self
    }
}

struct AsyncCallback<T> {
    type_id: TypeId,
    callback: Arc<dyn Fn(T, Bytes) -> BoxFuture<'static, ShouldRender<T>> + Send + Sync + 'static>,
}

impl<T> Clone for AsyncCallback<T> {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            callback: self.callback.clone(),
        }
    }
}

impl<T> Hash for AsyncCallback<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

impl<T> PartialEq for AsyncCallback<T> {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl<T> Eq for AsyncCallback<T> {}
