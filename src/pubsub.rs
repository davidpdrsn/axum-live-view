use bytes::Bytes;
use futures_util::stream::{BoxStream, StreamExt};
use std::future::{ready, Future, Ready};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

pub trait PubSub: Clone + Send + Sync + 'static {
    type SendFuture: Future<Output = anyhow::Result<()>>;
    type SubscribeFuture: Future<Output = BoxStream<'static, Bytes>>;

    fn send(&self, topic: &str, message: Bytes) -> Self::SendFuture;

    fn subscribe(&self, topic: &str) -> Self::SubscribeFuture;
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

impl PubSub for InProcess {
    type SendFuture = Ready<anyhow::Result<()>>;
    type SubscribeFuture = Ready<BoxStream<'static, Bytes>>;

    fn send(&self, topic: &str, message: Bytes) -> Self::SendFuture {
        let _ = self.tx.send((topic.to_owned(), message));
        ready(Ok(()))
    }

    fn subscribe(&self, topic: &str) -> Self::SubscribeFuture {
        let topic = topic.to_owned();

        let stream = BroadcastStream::new(self.tx.subscribe())
            .filter_map(|result| ready(result.ok()))
            .filter(move |(msg_topic, _)| ready(**msg_topic == topic))
            .map(|(_, msg)| msg)
            .boxed();

        ready(stream)
    }
}
