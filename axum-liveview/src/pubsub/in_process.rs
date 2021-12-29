use super::*;

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

#[async_trait]
impl PubSubBackend for InProcess {
    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        let _ = self.tx.send((topic.to_owned(), msg));
        Ok(())
    }

    async fn subscribe_raw(&self, topic: &str) -> anyhow::Result<BoxStream<'static, Bytes>> {
        let topic = topic.to_owned();

        let stream = BroadcastStream::new(self.tx.subscribe())
            .filter_map(|result| ready(result.ok()))
            .filter(move |(msg_topic, _)| ready(**msg_topic == topic))
            .map(|(_, msg)| msg)
            .boxed();

        Ok(stream)
    }
}
