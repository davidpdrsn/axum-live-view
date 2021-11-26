#![allow(clippy::new_without_default)]

use axum::{
    response::{Headers, IntoResponse},
    routing::get,
    AddExtensionLayer, Router,
};
use bytes::Bytes;
use maud::Markup;

pub mod pubsub;

mod liveview;
mod manager;
mod ws;

pub use self::{
    liveview::{LiveView, ShouldRender, Subscriptions},
    manager::LiveViewManager,
    pubsub::PubSubExt,
};

pub fn routes<B>() -> Router<B>
where
    B: Send + 'static,
{
    Router::new()
        .merge(ws::routes())
        .route("/live/app.js", get(js))
}

pub fn assets() -> Markup {
    maud::html! {
        script src="/live/app.js" {}
    }
}

async fn js() -> impl IntoResponse {
    const JS: &str = concat!(include_str!("morphdom.js"), include_str!("liveview.js"));
    (Headers([("content-type", "application/javascript")]), JS)
}

pub fn layer<P>(pubsub: P) -> AddExtensionLayer<LiveViewManager>
where
    P: pubsub::PubSub,
{
    AddExtensionLayer::new(LiveViewManager::new(pubsub::Logging::new(pubsub)))
}

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
