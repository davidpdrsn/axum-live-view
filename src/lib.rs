#![warn(
    clippy::all,
    clippy::dbg_macro,
    clippy::todo,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::mem_forget,
    clippy::unused_self,
    clippy::filter_map_next,
    clippy::needless_continue,
    clippy::needless_borrow,
    clippy::match_wildcard_for_single_variants,
    clippy::if_let_mutex,
    clippy::mismatched_target_os,
    clippy::await_holding_lock,
    clippy::match_on_vec_items,
    clippy::imprecise_flops,
    clippy::suboptimal_flops,
    clippy::lossy_float_literal,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::fn_params_excessive_bools,
    clippy::exit,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::macro_use_imports,
    clippy::option_option,
    clippy::verbose_file_reads,
    clippy::unnested_or_patterns,
    clippy::str_to_string,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    // missing_debug_implementations,
    // missing_docs
)]
#![deny(unreachable_pub, private_in_public)]
#![allow(elided_lifetimes_in_paths, clippy::type_complexity)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]

use axum::{
    response::{Headers, IntoResponse},
    routing::get,
    AddExtensionLayer, Router,
};
use bytes::Bytes;
use maud::Markup;
use serde::{de::DeserializeOwned, Serialize};

pub mod liveview;
pub mod pubsub;

mod manager;
mod ws;

#[doc(inline)]
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

impl<T> Codec for (T,)
where
    T: Codec,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        self.0.encode()
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        let t = T::decode(msg)?;
        Ok((t,))
    }
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

impl<T> Codec for axum::Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        let bytes = serde_json::to_vec(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(Self(serde_json::from_slice(&msg)?))
    }
}
