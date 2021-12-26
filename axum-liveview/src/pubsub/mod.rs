use axum::async_trait;
use bytes::Bytes;
use futures_util::stream::{BoxStream, StreamExt};
use std::{
    future::{ready, Future},
    pin::Pin,
    sync::Arc,
};
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
pub trait PubSub: Send + Sync + 'static {
    async fn send_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()>;

    async fn subscribe_raw(&self, topic: &str) -> BoxStream<'static, Bytes>;

    async fn broadcast<T>(&self, topic: &T, msg: T::Message) -> anyhow::Result<()>
    where
        T: Topic,
        Self: Sized,
    {
        let bytes = msg.encode()?;
        self.send_raw(topic.topic(), bytes).await?;
        Ok(())
    }

    fn subscribe<'a, T>(
        &'a self,
        topic: &T,
    ) -> Pin<Box<dyn Future<Output = BoxStream<'static, T::Message>> + Send + 'a>>
    where
        T: Topic,
        Self: Sized,
    {
        let topic = topic.topic().to_owned();

        Box::pin(async move {
            let mut stream = self.subscribe_raw(&topic).await;

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

            Box::pin(decoded_stream) as BoxStream<'static, T::Message>
        })
    }
}

#[async_trait]
impl PubSub for Arc<dyn PubSub> {
    async fn send_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        PubSub::send_raw(&**self, topic, msg).await
    }

    async fn subscribe_raw(&self, topic: &str) -> BoxStream<'static, Bytes> {
        PubSub::subscribe_raw(&**self, topic).await
    }
}

// ---- Logging ----

pub(crate) struct Logging<P> {
    inner: P,
}

impl<P> Logging<P> {
    pub(crate) fn new(inner: P) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<P> PubSub for Logging<P>
where
    P: PubSub,
{
    async fn send_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        {
            let msg = String::from_utf8_lossy(&msg);
            tracing::trace!(?topic, %msg, "send_raw");
        }
        self.inner.send_raw(topic, msg).await
    }

    async fn subscribe_raw(&self, topic: &str) -> BoxStream<'static, Bytes> {
        tracing::trace!(?topic, "subscribing");
        self.inner.subscribe_raw(topic).await
    }
}
