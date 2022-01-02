use axum::async_trait;
use bytes::Bytes;
use futures_util::{
    future::BoxFuture,
    stream::{BoxStream, StreamExt},
};
use std::{future::ready, sync::Arc};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

mod encode_decode;
mod in_process;

pub use self::{
    encode_decode::{Bincode, Decode, Encode},
    in_process::InProcess,
};

pub trait Topic: Send + Sync + 'static {
    type Message: Encode + Decode + Send;

    fn topic(&self) -> &str;
}

#[async_trait]
pub trait PubSubBackend: Send + Sync + 'static {
    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()>;

    async fn subscribe_raw(&self, topic: &str) -> anyhow::Result<BoxStream<'static, Bytes>>;
}

pub trait PubSub: PubSubBackend {
    fn broadcast<'a, T>(&'a self, topic: &T, msg: T::Message) -> BoxFuture<'a, anyhow::Result<()>>
    where
        T: Topic,
        Self: Sized,
    {
        let topic = topic.topic().to_owned();
        Box::pin(async move {
            let bytes = msg.encode()?;
            self.broadcast_raw(&topic, bytes).await?;
            Ok(())
        })
    }

    fn subscribe<'a, T>(
        &'a self,
        topic: &T,
    ) -> BoxFuture<'a, anyhow::Result<BoxStream<'static, T::Message>>>
    where
        T: Topic,
        Self: Sized,
    {
        let topic = topic.topic().to_owned();

        Box::pin(async move {
            let mut stream = self.subscribe_raw(&topic).await?;

            let decoded_stream = async_stream::stream! {
                while let Some(bytes) = stream.next().await {
                    match T::Message::decode(bytes) {
                        Ok(msg) => yield msg,
                        Err(err) => {
                            tracing::warn!(
                                ?topic,
                                ?err,
                                expected_type = tracing::field::display(std::any::type_name::<T>()),
                                "failed to decode message for topic stream",
                            );
                        }
                    }
                }
            };

            Ok(Box::pin(decoded_stream) as BoxStream<'static, T::Message>)
        })
    }
}

impl<T> PubSub for T where T: PubSubBackend {}

#[async_trait]
impl PubSubBackend for Arc<dyn PubSubBackend> {
    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        PubSubBackend::broadcast_raw(&**self, topic, msg).await
    }

    async fn subscribe_raw(&self, topic: &str) -> anyhow::Result<BoxStream<'static, Bytes>> {
        PubSubBackend::subscribe_raw(&**self, topic).await
    }
}

pub(crate) struct Logging<P> {
    inner: P,
}

impl<P> Logging<P> {
    pub(crate) fn new(inner: P) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<P> PubSubBackend for Logging<P>
where
    P: PubSubBackend,
{
    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        {
            let msg = String::from_utf8_lossy(&msg);
            tracing::trace!(?topic, %msg, "broadcast_raw");
        }
        self.inner.broadcast_raw(topic, msg).await
    }

    async fn subscribe_raw(&self, topic: &str) -> anyhow::Result<BoxStream<'static, Bytes>> {
        tracing::trace!(?topic, "subscribing");
        self.inner.subscribe_raw(topic).await
    }
}
