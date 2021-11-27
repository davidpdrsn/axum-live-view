use crate::codec::Codec;
use axum::async_trait;
use bytes::Bytes;
use futures_util::stream::{BoxStream, StreamExt};
use parking_lot::RwLock;
use std::{
    collections::{hash_map::Entry, HashMap},
    future::ready,
    sync::Arc,
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[cfg(feature = "tokio-postgres")]
mod postgres;
#[cfg(feature = "tokio-postgres")]
pub use postgres::Postgres;

mod in_process;
pub use in_process::InProcess;

// TODO(david): redis pubsub

#[async_trait]
pub trait PubSub: Send + Sync + 'static {
    async fn send_bytes(&self, topic: &str, msg: Bytes) -> anyhow::Result<()>;

    async fn subscribe(&self, topic: &str) -> BoxStream<'static, Bytes>;
}

#[async_trait]
pub trait PubSubExt: PubSub {
    async fn send<T>(&self, topic: &str, msg: T) -> anyhow::Result<()>
    where
        T: Codec + Send + Sync + 'static,
    {
        match msg.encode() {
            Ok(bytes) => self.send_bytes(topic, bytes).await,
            Err(err) => Err(err),
        }
    }
}

impl<P> PubSubExt for P where P: PubSub {}

#[async_trait]
impl PubSub for Arc<dyn PubSub> {
    async fn send_bytes(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        PubSub::send_bytes(&**self, topic, msg).await
    }

    async fn subscribe(&self, topic: &str) -> BoxStream<'static, Bytes> {
        PubSub::subscribe(&**self, topic).await
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
    async fn send_bytes(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        {
            let msg = String::from_utf8_lossy(&msg);
            tracing::trace!(?topic, %msg, "send_bytes");
        }
        self.inner.send_bytes(topic, msg).await
    }

    async fn subscribe(&self, topic: &str) -> BoxStream<'static, Bytes> {
        tracing::trace!(?topic, "subscribing");
        self.inner.subscribe(topic).await
    }
}
