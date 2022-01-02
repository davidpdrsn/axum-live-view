use crate::{
    live_view::{EventData, LiveView, LiveViewId, Updated},
    pubsub::{Decode, PubSub, Topic},
    topics::{self, FixedTopic},
    ws::WithAssociatedData,
};
use axum::Json;
use bytes::Bytes;
use futures_util::{future::BoxFuture, Stream};
use std::sync::Arc;
use std::{
    any::{type_name, TypeId},
    fmt,
    future::{ready, Future},
    hash::Hash,
};
use tokio_stream::StreamMap;

pub struct Subscriptions<T>
where
    T: LiveView,
{
    update_topic: FixedTopic<Json<WithAssociatedData<T::Message>>>,
    update_callback: AsyncCallback<T>,
    subscriptions: Vec<(String, AsyncCallback<T>)>,
}

impl<T> fmt::Debug for Subscriptions<T>
where
    T: LiveView,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            update_topic,
            update_callback,
            subscriptions,
        } = self;

        f.debug_struct("Subscriptions")
            .field("update_topic", &update_topic)
            .field("update_callback", &update_callback)
            .field("subscriptions", &subscriptions)
            .finish()
    }
}

impl<T> Subscriptions<T>
where
    T: LiveView,
{
    pub(crate) fn new(liveview_id: LiveViewId) -> Self {
        let callback = |this: T, Json(msg): Json<WithAssociatedData<T::Message>>| {
            let WithAssociatedData { msg, data } = msg;
            let data = EventData::new(data);
            this.update(msg, data)
        };
        let update_topic = topics::update::<T::Message>(liveview_id);
        let callback = make_callback(&update_topic, callback);

        Self {
            update_topic,
            update_callback: callback,
            subscriptions: Default::default(),
        }
    }

    pub fn on<F, K, Fut>(&mut self, topic: &K, callback: F)
    where
        K: Topic + Send + 'static,
        F: Fn(T, K::Message) -> Fut + Copy + Send + Sync + 'static,
        Fut: Future<Output = Updated<T>> + Send + 'static,
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

fn make_callback<L, T, F, M, Fut>(topic: &T, callback: F) -> AsyncCallback<L>
where
    L: LiveView,
    T: Topic,
    F: Fn(L, M) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = Updated<L>> + Send + 'static,
    M: Decode,
{
    let topic = topic.topic().to_owned().into();

    let callback = Arc::new(
        move |receiver: L, raw_msg: Bytes| match M::decode(raw_msg.clone()) {
            Ok(msg) => Box::pin(callback(receiver, msg)) as _,
            Err(err) => {
                tracing::warn!(
                    ?err,
                    t_type_name = %type_name::<L>(),
                    msg_type_name = %type_name::<T::Message>(),
                    raw_msg = ?std::str::from_utf8(&raw_msg),
                    "failed to decode message for subscriber",
                );
                Box::pin(ready(Updated::new(receiver))) as _
            }
        },
    );

    AsyncCallback {
        type_id: TypeId::of::<F>(),
        topic,
        callback,
    }
}

pub(crate) struct AsyncCallback<T> {
    type_id: TypeId,
    topic: Arc<str>,
    callback: Arc<dyn Fn(T, Bytes) -> BoxFuture<'static, Updated<T>> + Send + Sync + 'static>,
}

impl<T> fmt::Debug for AsyncCallback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            type_id,
            topic,
            callback: _,
        } = self;

        f.debug_struct("AsyncCallback")
            .field("type_id", &type_id)
            .field("topic", &topic)
            .field("callback", &"...")
            .finish()
    }
}

impl<T> AsyncCallback<T> {
    pub(crate) async fn call(self, t: T, msg: Bytes) -> Updated<T> {
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
