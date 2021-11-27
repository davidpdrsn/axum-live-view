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
impl PubSub for InProcess {
    async fn send_bytes(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        let _ = self.tx.send((topic.to_owned(), msg));
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> BoxStream<'static, Bytes> {
        let topic = topic.to_owned();

        BroadcastStream::new(self.tx.subscribe())
            .filter_map(|result| ready(result.ok()))
            .filter(move |(msg_topic, _)| ready(**msg_topic == topic))
            .map(|(_, msg)| msg)
            .boxed()
    }
}
