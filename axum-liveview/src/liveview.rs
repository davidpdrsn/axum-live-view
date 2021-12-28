use crate::{
    html::Html,
    pubsub::{Decode, PubSub, Topic},
    topics::{self, FixedTopic},
    ws::{FormEventValue, KeyEventValue},
};
use async_stream::stream;
use axum::{async_trait, Json};
use bytes::Bytes;
use futures_util::{
    future::{BoxFuture, FutureExt},
    stream::StreamExt,
    Stream,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;
use std::{
    any::{type_name, TypeId},
    future::{ready, Future},
    hash::Hash,
};
use tokio_stream::StreamMap;
use uuid::Uuid;

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;

    fn setup(&self, sub: &mut Setup<Self>);

    async fn update(self, msg: Self::Message, ctx: EventContext) -> Self;

    fn render(&self) -> Html<Self::Message>;
}

pub(crate) async fn run_to_stream<T, P>(
    mut liveview: T,
    pubsub: P,
    liveview_id: Uuid,
) -> impl Stream<Item = LiveViewStreamItem<T::Message>> + Send
where
    T: LiveView,
    P: PubSub,
{
    let mut setup = Setup::new(liveview_id);
    liveview.setup(&mut setup);

    let Setup {
        update_topic,
        update_callback,
        subscriptions,
    } = setup;

    let mut stream_map = StreamMap::new();

    let stream = pubsub.subscribe_raw(update_topic.topic()).await;
    stream_map.insert(update_callback, stream);

    for (topic, callback) in subscriptions {
        let stream = pubsub.subscribe_raw(&topic).await;
        stream_map.insert(callback, stream);
    }

    stream! {
        while let Some((callback, msg)) = stream_map.next().await {
            liveview = (callback.callback)(liveview, msg).await;
            let markup = liveview.render();
            yield LiveViewStreamItem::Html(markup);
        }
    }
}

pub(crate) enum LiveViewStreamItem<T> {
    Html(Html<T>),
}

pub struct Setup<T>
where
    T: LiveView,
{
    update_topic: FixedTopic<Json<WithEventContext<T::Message>>>,
    update_callback: AsyncCallback<T>,
    subscriptions: Vec<(String, AsyncCallback<T>)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct WithEventContext<T> {
    pub(crate) msg: T,
    pub(crate) ctx: EventContext,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EventContext {
    inner: EventContextInner,
}

impl EventContext {
    pub(crate) fn click() -> Self {
        Self {
            inner: EventContextInner::Click,
        }
    }

    pub(crate) fn form(value: FormEventValue) -> Self {
        Self {
            inner: EventContextInner::Form(value),
        }
    }

    pub(crate) fn key(value: KeyEventValue) -> Self {
        Self {
            inner: EventContextInner::Key(value),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum EventContextInner {
    Click,
    Form(FormEventValue),
    Key(KeyEventValue),
}

impl<T> Setup<T>
where
    T: LiveView,
{
    fn new(liveview_id: Uuid) -> Self {
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

struct AsyncCallback<T> {
    type_id: TypeId,
    topic: Arc<str>,
    callback: Arc<dyn Fn(T, Bytes) -> BoxFuture<'static, T> + Send + Sync + 'static>,
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
