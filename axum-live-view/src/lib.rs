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

use axum::{http::header, response::Headers, routing::get, Router};

#[macro_use]
mod macros;

pub mod event_data;
pub mod extract;
pub mod js_command;
pub mod live_view;
pub mod test;

mod html;
mod life_cycle;
mod util;

#[doc(inline)]
pub use self::{extract::LiveViewUpgrade, html::Html, live_view::LiveView};
pub use axum_live_view_macros::html;

#[cfg(feature = "precompiled-js")]
#[cfg_attr(docsrs, doc(cfg(feature = "precompiled-js")))]
pub const PRECOMPILED_JS: &str = include_str!("../../assets/axum_live_view.min.js");

#[cfg(feature = "precompiled-js")]
#[cfg_attr(docsrs, doc(cfg(feature = "precompiled-js")))]
pub fn precompiled_js_route<B>(path: &str) -> Router<B>
where
    B: Send + 'static,
{
    Router::new().route(
        path,
        get(|| async {
            (
                Headers([(header::CONTENT_TYPE, "application/json")]),
                PRECOMPILED_JS,
            )
        }),
    )
}

#[doc(hidden)]
pub mod __private {
    //! Private API. Do _not_ use anything from this module!

    pub use crate::html::private::*;
}
