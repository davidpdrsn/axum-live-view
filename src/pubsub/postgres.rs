use super::*;

pub struct Postgres {
    client: tokio_postgres::Client,
    subscriptions: Arc<RwLock<HashMap<String, broadcast::Sender<Bytes>>>>,
}

impl Postgres {
    pub async fn new(config: &str) -> anyhow::Result<Self> {
        let (client, mut conn) = tokio_postgres::connect(config, tokio_postgres::NoTls).await?;

        let subscriptions: Arc<RwLock<HashMap<_, broadcast::Sender<_>>>> = Default::default();

        {
            let subscriptions = subscriptions.clone();
            tokio::spawn(async move {
                let mut stream = futures_util::stream::poll_fn(|cx| conn.poll_message(cx));
                while let Some(msg) = stream.next().await {
                    match msg {
                        Ok(msg) => match msg {
                            tokio_postgres::AsyncMessage::Notice(notice) => {
                                tracing::info!(%notice, "connection notice");
                            }
                            tokio_postgres::AsyncMessage::Notification(notification) => {
                                let topic = notification.channel();
                                let msg = Bytes::copy_from_slice(notification.payload().as_bytes());

                                if let Some(tx) = subscriptions.read().get(topic) {
                                    let _ = tx.send(msg);
                                }
                            }
                            _ => {}
                        },
                        Err(err) => {
                            tracing::error!(%err, "connection error");
                        }
                    }
                }
            });
        }

        Ok(Self {
            client,
            subscriptions,
        })
    }
}

#[async_trait]
impl PubSub for Postgres {
    async fn send_bytes(&self, topic: &str, msg: Bytes) -> anyhow::Result<()> {
        let topic = pg_sanitize_topic(topic);

        let msg = String::from_utf8(msg.to_vec())?;
        self.client
            .execute("select pg_notify($1, $2)", &[&topic, &msg])
            .await?;

        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> BoxStream<'static, Bytes> {
        let topic = pg_sanitize_topic(topic);

        let rx = match self.subscriptions.write().entry(topic.clone()) {
            Entry::Occupied(entry) => entry.get().subscribe(),
            Entry::Vacant(entry) => {
                let (tx, rx) = broadcast::channel(1024);
                entry.insert(tx);
                rx
            }
        };

        let query = format!("listen {}", topic);
        self.client
            // TODO(david): can this contain user input?
            .execute(&*query, &[])
            .await
            .unwrap();

        tokio_stream::wrappers::BroadcastStream::new(rx)
            .filter_map(|msg| async move { msg.ok() })
            .boxed()
    }
}

// TODO(david): find proper way to sanitize the names, maybe some a-zA-Z encoding?
fn pg_sanitize_topic(topic: &str) -> String {
    topic.to_owned().replace('/', "_").replace('-', "_")
}
