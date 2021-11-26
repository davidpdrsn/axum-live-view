use crate::Codec;
use bytes::Bytes;
use futures_util::{
    future::BoxFuture,
    stream::{BoxStream, StreamExt},
};
use std::{future::ready, sync::Arc};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

pub trait PubSub: Send + Sync + 'static {
    fn send_bytes(&self, topic: &str, message: Bytes) -> BoxFuture<'static, anyhow::Result<()>>;

    fn subscribe(&self, topic: &str) -> BoxFuture<'static, BoxStream<'static, Bytes>>;
}

pub trait PubSubExt: PubSub {
    fn send<T>(&self, topic: &str, message: T) -> BoxFuture<'static, anyhow::Result<()>>
    where
        T: Codec,
    {
        match message.encode() {
            Ok(bytes) => self.send_bytes(topic, bytes),
            Err(err) => Box::pin(ready(Err(err))),
        }
    }
}

impl<P> PubSubExt for P where P: PubSub {}

impl PubSub for Arc<dyn PubSub> {
    fn send_bytes(&self, topic: &str, message: Bytes) -> BoxFuture<'static, anyhow::Result<()>> {
        PubSub::send_bytes(&**self, topic, message)
    }

    fn subscribe(&self, topic: &str) -> BoxFuture<'static, BoxStream<'static, Bytes>> {
        PubSub::subscribe(&**self, topic)
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

impl<P> PubSub for Logging<P>
where
    P: PubSub,
{
    fn send_bytes(&self, topic: &str, message: Bytes) -> BoxFuture<'static, anyhow::Result<()>> {
        {
            let message = String::from_utf8_lossy(&message);
            tracing::trace!(?topic, %message, "send_bytes");
        }
        self.inner.send_bytes(topic, message)
    }

    fn subscribe(&self, topic: &str) -> BoxFuture<'static, BoxStream<'static, Bytes>> {
        tracing::trace!(?topic, "subscribing");
        self.inner.subscribe(topic)
    }
}

#[derive(Clone)]
pub struct InProcess {
    tx: broadcast::Sender<(String, Bytes)>,
}

impl InProcess {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }
}

impl Default for InProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl PubSub for InProcess {
    fn send_bytes(&self, topic: &str, message: Bytes) -> BoxFuture<'static, anyhow::Result<()>> {
        let _ = self.tx.send((topic.to_owned(), message));
        Box::pin(ready(Ok(())))
    }

    fn subscribe(&self, topic: &str) -> BoxFuture<'static, BoxStream<'static, Bytes>> {
        let topic = topic.to_owned();

        let stream = BroadcastStream::new(self.tx.subscribe())
            .filter_map(|result| ready(result.ok()))
            .filter(move |(msg_topic, _)| ready(**msg_topic == topic))
            .map(|(_, msg)| msg)
            .boxed();

        Box::pin(ready(stream))
    }
}
