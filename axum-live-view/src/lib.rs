//! Real-time user experiences with server-rendered HTML.
//!
//! axum-live-view allows you to build rich, real-time experiences with server-rendered HTML. This
//! is done entirely in Rust - you don't need to write any JavaScript or compile to WASM.
//!
//! Basically [Phoenix LiveView][phx] for [axum].
//!
//! # Example
//!
//! ```rust
//! use axum::{response::IntoResponse, routing::get, Router};
//! use axum_live_view::{
//!     event_data::EventData, html, live_view::Updated, Html, LiveView, LiveViewUpgrade,
//! };
//! use serde::{Deserialize, Serialize};
//! use std::convert::Infallible;
//!
//! #[tokio::main]
//! async fn main() {
//!     // A normal axum router...
//!     let app = Router::new()
//!         .route("/", get(root))
//!         // Use a precompiled and minified build of axum-live-view's JavaScript.
//!         // This is the easiest way to get started. Integration with bundlers
//!         // is of course also possible.
//!         .route("/assets/live-view.js", axum_live_view::precompiled_js());
//!
//!     # async {
//!     // ...that we run like any other axum app
//!     axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
//!         .serve(app.into_make_service())
//!         .await
//!         .unwrap();
//!     # };
//! }
//!
//! // Our handler function for `GET /`
//! async fn root(
//!     // `LiveViewUpgrade` is an extractor that accepts both regular requests and
//!     // WebSocket upgrade requests. If it receives a regular request it will
//!     // render your live view's HTML and return a regular static response. This
//!     // leads to good SEO and fast first paint.
//!     //
//!     // axum-live-view's JavaScript client will then call this endpoint a second
//!     // time to establish a WebSocket connection at which point your view will be
//!     // spawned in an async task. Events from the browser and HTML diffs from
//!     // your view will then be sent over the WebSocket connection.
//!     //
//!     // If the WebSocket connection breaks (or your view crashes) the JavaScript
//!     // client will call this endpoint again to establish a new connection and
//!     // a new instance of your view is created.
//!     //
//!     // The task running the old view automatically stops when the WebSocket is
//!     // closed.
//!     live: LiveViewUpgrade,
//! ) -> impl IntoResponse {
//!     // `Counter` is our live view and we initialize it with the default values.
//!     let counter = Counter::default();
//!
//!     live.response(|embed_live_view| {
//!         html! {
//!             <!DOCTYPE html>
//!             <html>
//!                 <head>
//!                 </head>
//!                 <body>
//!                     // Embed our live view into the HTML template. This will render the
//!                     // view and include the HTML in the response, leading to good SEO
//!                     // and fast first paint.
//!                     { embed_live_view.embed(counter) }
//!
//!                     // Load the JavaScript. This will automatically initialize live view
//!                     // connections.
//!                     <script src="/assets/live-view.js"></script>
//!                 </body>
//!             </html>
//!         }
//!     })
//! }
//!
//! // Our live view is just a regular Rust struct...
//! #[derive(Default)]
//! struct Counter {
//!     count: u64,
//! }
//!
//! // ...that implements the `LiveView` trait.
//! impl LiveView for Counter {
//!     // This is the type of update messages our HTML contains. They will be sent
//!     // to the view in the `update` method
//!     type Message = Msg;
//!
//!     // Update the view based on which message it receives.
//!     //
//!     // `EventData` contains data from the event that happened in the
//!     // browser. This might be values of input fields or which key was pressed in
//!     // a keyboard event.
//!     fn update(
//!         mut self,
//!         msg: Msg,
//!         data: Option<EventData>,
//!     ) -> Updated<Self> {
//!         match msg {
//!             Msg::Increment => {
//!                 self.count += 1;
//!             }
//!             Msg::Decrement => {
//!                 if self.count > 0 {
//!                     self.count -= 1;
//!                 }
//!             }
//!         }
//!
//!         Updated::new(self)
//!     }
//!
//!     // Render the live view into an HTML template. This function is called during
//!     // the initial render in `LiveViewManager::embed` and for each subsequent
//!     // update.
//!     //
//!     // The HTML is diff'ed on the server and only minimal deltas are sent over
//!     // the wire. The browser then builds the full HTML template and efficiently
//!     // updates the DOM.
//!     fn render(&self) -> Html<Self::Message> {
//!         html! {
//!             <div>
//!                 "Counter value: "
//!                 // Embed dynamic Rust values into the HTML.
//!                 //
//!                 // `if`, `for`, and `match` are also supported.
//!                 { self.count }
//!             </div>
//!
//!             <div>
//!                 // Elements with the `axm-click` attribute will send an update message
//!                 // to the view which calls `update` after which the view is
//!                 // re-rendered.
//!                 <button axm-click={ Msg::Increment }>"+"</button>
//!                 <button axm-click={ Msg::Decrement }>"-"</button>
//!             </div>
//!         }
//!     }
//!
//!     // The `LiveView` trait also has a `mount` method that is called when a new
//!     // WebSocket connects. This can be used to perform auth, load data that
//!     // isn't needed for the first response, and spawn a task that can send
//!     // messages to the view itself from other parts of the application.
//! }
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! enum Msg {
//!     Increment,
//!     Decrement,
//! }
//! ```
//!
//! # Life cycle
//!
//! [`LiveViewUpgrade::response`] is used to embed live view's in templates and connect them to
//! axum-live-view's JavaScript client. The life cycle for a view created with [`LiveViewUpgrade::response`] is:
//!
//! 1. The browser makes a regular `GET` request to an endpoint that has the [`LiveViewUpgrade`]
//!    extractor.
//! 2. [`LiveViewUpgrade::response`] is called to embed a live view and return a regular static HTML response.
//! 3. The JavaScript client notices that the page contains a live view and issues a WebSocket
//!    upgrade request to the same endpoint.
//! 4. [`LiveViewUpgrade`] notices that its responding to a WebSocket request, and instead of
//!    returning HTML, it will run your live view in an async task and send state updates over the
//!    socket connection.
//! 5. When the client leaves the page, and closes the WebSocket connection, the async task is
//!    automatically shutdown.
//! 6. If the connection terminates, perhaps due to an error, the JavaScript client will
//!    automatically open a new connection by calling your endpoint again. Thus
//!    [`LiveViewUpgrade::response`] will be called and a new fresh instance of your view is
//!    created.
//!
//! Take note that your endpoint is called twice. Once with a regular `GET` request and again to
//! upgrade to a stateful WebSocket connection. You can use [`EmbedLiveView::connected`] to check
//! whether the handler is responding to the initial `GET` request or the WebSocket upgrade
//! request.
//!
//! # Bindings
//!
//! axum-live-view supports bindings to react to client side events and update the state of your
//! view.
//!
//! For example to react to clicks on a button use `axm-click`:
//!
//! ```rust
//! # use axum_live_view::html;
//! # #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
//! # enum Msg { Increment }
//! #
//! html! {
//!     <button axm-click={ Msg::Increment }>"+"</button>
//! };
//! ```
//!
//! This will send `Msg::Increment` to your view's `update` method.
//!
//! See [`html`](macro.html.html) for details on all the support bindings.
//!
//! # Pros and cons
//!
//! Some pros and cons to keep in mind when deciding whether axum-live-view is right for your use
//! case:
//!
//! ## Pros
//!
//! - Simple programming model. You just write Rust and don't have to worry about all the
//! complexities associated with client-side development.
//! - You don't need to build and maintain a separate API.
//! - Use code that isn't otherwise compatible with WASM, since your views run entirely on the
//! server.
//!
//! ## Cons
//!
//! - Increased latency. If your servers are far away from your users you might get more latency
//! since updating the view requires a roundtrip to the server.
//!
//! [phx]: https://github.com/phoenixframework/phoenix_live_view
//! [axum]: https://github.com/tokio-rs/axum
//! [`EmbedLiveView::connected`]: extract::EmbedLiveView::connected

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

#[doc = include_str!("docs/html.md")]
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
