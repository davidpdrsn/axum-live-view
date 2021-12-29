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
    Router,
};

#[macro_use]
mod macros;

pub mod associated_data;
pub mod html;
pub mod js;
pub mod liveview;
pub mod middleware;
pub mod pubsub;

mod subscriptions;
mod topics;
mod ws;

pub use self::{
    associated_data::AssociatedData,
    html::Html,
    liveview::{embed::EmbedLiveView, LiveView},
    middleware::layer,
    pubsub::PubSub,
    subscriptions::Subscriptions,
};
pub use axum_liveview_macros::html;

const APP_JS_PATH: &str = "/live/app.js";

pub fn routes<B>() -> Router<B>
where
    B: Send + 'static,
{
    Router::new()
        .merge(ws::routes())
        .route(APP_JS_PATH, get(js))
}

pub fn assets<T>() -> html::Html<T> {
    use crate as axum_liveview;
    html! {
        <script src={ APP_JS_PATH }></script>
    }
}

async fn js() -> impl IntoResponse {
    const JS: &str = concat!(
        include_str!("js/morphdom.js"),
        include_str!("js/liveview.js")
    );
    (Headers([("content-type", "application/javascript")]), JS)
}
