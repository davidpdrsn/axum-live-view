use axum::async_trait;
use bytes::Bytes;
use futures_util::stream::{BoxStream, StreamExt};
use std::{future::ready, sync::Arc};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

mod in_process;
mod message;

pub use self::{
    in_process::InProcess,
    message::{Bincode, Message},
};

#[async_trait]
pub trait PubSub: Send + Sync + 'static {
    async fn send_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()>;

    async fn subscribe_raw(&self, topic: &str) -> BoxStream<'static, Bytes>;

    async fn broadcast<T>(&self, topic: &str, msg: T) -> anyhow::Result<()>
    where
        Self: Sized,
        T: Message + Send + Sync + 'static,
    {
        match msg.encode() {
            Ok(bytes) => self.send_raw(topic, bytes).await,
            Err(err) => Err(err),
        }
    }

    async fn subscribe<T>(&self, topic: &str) -> BoxStream<'static, T>
    where
        Self: Sized,
        T: Message + Send + Sync + 'static,
    {
        let mut stream = self.subscribe_raw(topic).await;
        let topic = topic.to_owned();
        let decoded_stream = async_stream::stream! {
            while let Some(bytes) = stream.next().await {
                match T::decode(bytes) {
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

        Box::pin(decoded_stream)
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
