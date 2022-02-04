//! TODO

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
    missing_docs
)]
#![deny(unreachable_pub, private_in_public)]
#![allow(elided_lifetimes_in_paths, clippy::type_complexity)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]

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

/// TODO
pub use axum_live_view_macros::html;

/// A precompiled build of axum-live-view's JavaScript.
///
/// This enables using axum-live-view without a bundler like [webpack].
///
/// What's compiled is
///
/// ```javascript
/// import { LiveView } from "axum-live-view"
///
/// window.liveView = new LiveView()
/// ```
///
/// Which will automatically detect and connect any live views on the page. The live view instance
/// is stored on the `window` object so its accessible to other scripts and from the developer
/// console.
///
/// This constant contains just the JavaScript itself allowing you to serve it however you want.
/// For a good default route that serves just the JavaScript use [`precompiled_js`].
///
/// [webpack]: https://webpack.js.org
#[cfg(feature = "precompiled-js")]
#[cfg_attr(docsrs, doc(cfg(feature = "precompiled-js")))]
pub const PRECOMPILED_JS: &str = include_str!("../../assets-precompiled/axum_live_view.min.js");

/// A route that returns a precompiled build of axum-live-view's JavaScript.
///
/// This enables using axum-live-view without a bundler like [webpack].
///
/// What's compiled is
///
/// ```javascript
/// import { LiveView } from "axum-live-view"
///
/// window.liveView = new LiveView()
/// ```
///
/// Which will automatically detect and connect any live views on the page. The live view instance
/// is stored on the `window` object so its accessible to other scripts and from the developer
/// console.
///
/// # Example
///
/// ```
/// use axum::Router;
/// use axum_live_view::precompiled_js;
///
/// let app = Router::new().route("/assets/live_view.js", precompiled_js());
/// # let _: Router<axum::body::Body> = app;
/// ```
///
/// [webpack]: https://webpack.js.org
#[cfg(feature = "precompiled-js")]
#[cfg_attr(docsrs, doc(cfg(feature = "precompiled-js")))]
#[allow(clippy::declare_interior_mutable_const)]
pub fn precompiled_js<B>() -> axum::routing::MethodRouter<B>
where
    B: Send + 'static,
{
    use axum::{
        http::{header, HeaderMap, HeaderValue, StatusCode},
        response::IntoResponse,
        routing::get,
    };

    const HASH: &str = include_str!("../../assets-precompiled/axum_live_view.hash.txt");
    const PRECOMPILED_JS_GZ: &[u8] =
        include_bytes!("../../assets-precompiled/axum_live_view.min.js.gz");

    const APPLICATION_JAVASCRIPT: HeaderValue =
        HeaderValue::from_static("application/javascript; charset=utf-8");
    const GZIP: HeaderValue = HeaderValue::from_static("gzip");

    get(|request_headers: HeaderMap| async move {
        let etag = format!("\"{HASH}\"").parse::<HeaderValue>().unwrap();

        if request_headers
            .get(header::IF_NONE_MATCH)
            .filter(|&value| value == etag)
            .is_some()
        {
            StatusCode::NOT_MODIFIED.into_response()
        } else {
            let mut response_headers = HeaderMap::new();

            response_headers.insert(header::CONTENT_TYPE, APPLICATION_JAVASCRIPT);
            response_headers.insert(header::ETAG, etag);

            if request_headers
                .get(header::ACCEPT_ENCODING)
                .and_then(|value| value.to_str().ok())
                .filter(|value| value.contains("gzip"))
                .is_some()
            {
                response_headers.insert(header::CONTENT_ENCODING, GZIP);
                (response_headers, PRECOMPILED_JS_GZ).into_response()
            } else {
                (response_headers, PRECOMPILED_JS).into_response()
            }
        }
    })
}

#[doc(hidden)]
pub mod __private {
    //! Private API. Do _not_ use anything from this module!

    pub use crate::html::private::*;
}
