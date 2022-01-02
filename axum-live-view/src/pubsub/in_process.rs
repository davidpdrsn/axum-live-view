use std::convert::Infallible;

use super::*;

#[derive(Debug, Clone)]
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

#[async_trait]
impl PubSubBackend for InProcess {
    type Error = Infallible;

    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> Result<(), Self::Error> {
        let _ = self.tx.send((topic.to_owned(), msg));
        Ok(())
    }

    async fn subscribe_raw(&self, topic: &str) -> Result<BoxStream<'static, Bytes>, Self::Error> {
        let topic = topic.to_owned();

        let stream = BroadcastStream::new(self.tx.subscribe())
            .filter_map(|result| ready(result.ok()))
            .filter(move |(msg_topic, _)| ready(**msg_topic == topic))
            .map(|(_, msg)| msg)
            .boxed();

        Ok(stream)
    }
}
