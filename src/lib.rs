#![allow(dead_code)]

use bytes::Bytes;

mod liveview;
mod pubsub;

pub use self::liveview::{LiveView, ShouldRender, Subscriptions};

pub trait Codec: Sized {
    fn encode(&self) -> anyhow::Result<Bytes>;

    fn decode(msg: Bytes) -> anyhow::Result<Self>;
}

impl Codec for Bytes {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(self.clone())
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(msg)
    }
}

impl Codec for String {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::copy_from_slice(self.as_bytes()))
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(std::str::from_utf8(&*msg)?.to_owned())
    }
}

impl Codec for () {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::new())
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        anyhow::ensure!(msg.is_empty());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{*, pubsub::PubSub};
    use async_trait::async_trait;
    use maud::Markup;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn counter() {
        #[derive(Default)]
        struct Counter {
            n: i32,
        }

        #[async_trait]
        impl LiveView for Counter {
            fn setup(subscriptions: &mut Subscriptions<Self>) {
                subscriptions
                    .on("counter/increment", Self::on_increment)
                    .on("counter/decrement", Self::on_decrement);
            }

            async fn render(&self) -> Markup {
                maud::html! { (self.n) }
            }
        }

        impl Counter {
            async fn on_increment(mut self, _msg: ()) -> ShouldRender<Self> {
                self.n += 1;
                ShouldRender::Yes(self)
            }

            async fn on_decrement(mut self, _msg: ()) -> ShouldRender<Self> {
                self.n -= 1;
                ShouldRender::Yes(self)
            }
        }

        let pubsub = pubsub::InProcess::new();
        let counter = Counter::default();
        let stream = liveview::run_to_stream(counter, pubsub.clone()).await;
        futures_util::pin_mut!(stream);

        pubsub.send("counter/increment", Bytes::new()).await.unwrap();
        assert_eq!(stream.next().await.unwrap().into_string(), "1");

        pubsub.send("counter/increment", Bytes::new()).await.unwrap();
        assert_eq!(stream.next().await.unwrap().into_string(), "2");

        pubsub.send("counter/decrement", Bytes::new()).await.unwrap();
        assert_eq!(stream.next().await.unwrap().into_string(), "1");
    }
}
