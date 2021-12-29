use crate::{
    liveview::LiveView,
    pubsub::{Decode, PubSub, Topic},
    topics::{self, FixedTopic},
    ws::WithEventContext,
};
use axum::Json;
use bytes::Bytes;
use futures_util::{
    future::{BoxFuture, FutureExt},
    Stream,
};
use std::sync::Arc;
use std::{
    any::{type_name, TypeId},
    future::{ready, Future},
    hash::Hash,
};
use tokio_stream::StreamMap;
use uuid::Uuid;

pub struct Subscriptions<T>
where
    T: LiveView,
{
    update_topic: FixedTopic<Json<WithEventContext<T::Message>>>,
    update_callback: AsyncCallback<T>,
    subscriptions: Vec<(String, AsyncCallback<T>)>,
}

impl<T> Subscriptions<T>
where
    T: LiveView,
{
    pub(crate) fn new(liveview_id: Uuid) -> Self {
        let callback = |this: T, Json(msg): Json<WithEventContext<T::Message>>| {
            let WithEventContext { msg, ctx } = msg;
            this.update(msg, ctx)
        };
        let update_topic = topics::update::<T::Message>(liveview_id);
        let callback = make_callback(&update_topic, callback);

        Self {
            update_topic,
            update_callback: callback,
            subscriptions: Default::default(),
        }
    }

    pub fn on<F, K>(&mut self, topic: &K, callback: F)
    where
        K: Topic + Send + 'static,
        T: Send + 'static,
        F: SubscriptionCallback<T, K::Message>,
    {
        let callback = make_callback(topic, callback);
        let topic = topic.topic().to_owned();
        self.subscriptions.push((topic, callback));
    }

    pub(crate) async fn into_stream<P>(
        self,
        pubsub: P,
    ) -> anyhow::Result<impl Stream<Item = (AsyncCallback<T>, Bytes)>>
    where
        P: PubSub,
    {
        let Subscriptions {
            update_topic,
            update_callback,
            subscriptions,
        } = self;

        let mut stream_map = StreamMap::with_capacity(subscriptions.len() + 1);

        let stream = pubsub.subscribe_raw(update_topic.topic()).await?;
        stream_map.insert(update_callback, stream);

        for (topic, callback) in subscriptions {
            let stream = pubsub.subscribe_raw(&topic).await?;
            stream_map.insert(callback, stream);
        }

        Ok(stream_map)
    }
}

fn make_callback<L, T, F, M>(topic: &T, callback: F) -> AsyncCallback<L>
where
    L: LiveView,
    T: Topic,
    F: SubscriptionCallback<L, M>,
    M: Decode,
{
    let topic = topic.topic().to_owned().into();

    let callback = Arc::new(
        move |receiver: L, raw_msg: Bytes| match M::decode(raw_msg.clone()) {
            Ok(msg) => Box::pin(callback.call(receiver, msg).map(|value| value.into())) as _,
            Err(err) => {
                tracing::warn!(
                    ?err,
                    t_type_name = %type_name::<L>(),
                    msg_type_name = %type_name::<T::Message>(),
                    raw_msg = ?std::str::from_utf8(&raw_msg),
                    "failed to decode message for subscriber",
                );
                Box::pin(ready(receiver)) as _
            }
        },
    );

    AsyncCallback {
        type_id: TypeId::of::<F>(),
        topic,
        callback,
    }
}

pub trait SubscriptionCallback<T, Msg>: Copy + Send + Sync + 'static {
    type Output: Into<T>;
    type Future: Future<Output = Self::Output> + Send + 'static;

    fn call(self, receiver: T, input: Msg) -> Self::Future;
}

impl<T, F, Fut, K> SubscriptionCallback<T, ()> for F
where
    F: Fn(T) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = K> + Send + 'static,
    K: Into<T>,
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
    K: Into<T>,
    Msg: Decode,
{
    type Output = K;
    type Future = Fut;

    fn call(self, receiver: T, (input,): (Msg,)) -> Self::Future {
        self(receiver, input)
    }
}

pub(crate) struct AsyncCallback<T> {
    type_id: TypeId,
    topic: Arc<str>,
    callback: Arc<dyn Fn(T, Bytes) -> BoxFuture<'static, T> + Send + Sync + 'static>,
}

impl<T> AsyncCallback<T> {
    pub(crate) async fn call(self, t: T, msg: Bytes) -> T {
        (self.callback)(t, msg).await
    }
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
