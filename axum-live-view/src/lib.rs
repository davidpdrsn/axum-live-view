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
    missing_debug_implementations,
    // missing_docs
)]
#![deny(unreachable_pub, private_in_public)]
#![allow(elided_lifetimes_in_paths, clippy::type_complexity)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]

#[macro_use]
mod macros;

#[doc(hidden)]
pub mod html;

pub mod js_command;
pub mod live_view;
pub mod middleware;
pub mod pubsub;

mod topics;
mod util;
mod ws;

pub use self::{
    html::Html,
    live_view::{EmbedLiveView, EventData, LiveView, Updated},
    middleware::LiveViewLayer,
};
pub use axum_live_view_macros::html;

pub fn router_parts<P, B>(pubsub: P) -> (axum::Router<B>, LiveViewLayer<P>)
where
    P: pubsub::PubSub + Clone,
    B: Send + 'static,
{
    let routes = ws::routes::<P, B>();
    let layer = LiveViewLayer::new(pubsub);
    (routes, layer)
}

fn spawn_unit<F>(future: F) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future)
}
