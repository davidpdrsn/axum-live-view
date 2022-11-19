# axum-live-view

axum-live-view allows you to build rich, real-time experiences with
server-rendered HTML. This is done entirely in Rust - no JavaScript or WASM
needed.

Basically [Phoenix LiveView][phx] but for [axum].

# 🚨 BIG SCARY WARNING 🚨

This project is still very much work in progress. Everything is subject to
change and you shouldn't use this for anything serious.

# Example usage

This is what using axum-live-view looks like.

```rust
use axum::{response::IntoResponse, routing::get, Router};
use axum_live_view::{
    event_data::EventData, html, live_view::Updated, Html, LiveView, LiveViewUpgrade,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

#[tokio::main]
async fn main() {
    // A normal axum router...
    let app = Router::new()
        .route("/", get(root))
        // Use a precompiled and minified build of axum-live-view's JavaScript.
        // This is the easiest way to get started. Integration with bundlers
        // is of course also possible.
        .route("/assets/live-view.js", axum_live_view::precompiled_js());

    // ...that we run like any other axum app
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Our handler function for `GET /`
async fn root(
    // `LiveViewUpgrade` is an extractor that accepts both regular requests and
    // WebSocket upgrade requests. If it receives a regular request it will
    // render your live view's HTML and return a regular static response. This
    // leads to good SEO and fast first paint.
    //
    // axum-live-view's JavaScript client will then call this endpoint a second
    // time to establish a WebSocket connection at which point your view will be
    // spawned in an async task. Events from the browser and HTML diffs from
    // your view will then be sent over the WebSocket connection.
    //
    // If the WebSocket connection breaks (or your view crashes) the JavaScript
    // client will call this endpoint again to establish a new connection and
    // a new instance of your view is created.
    //
    // The task running the old view automatically stops when the WebSocket is
    // closed.
    live: LiveViewUpgrade,
) -> impl IntoResponse {
    // `Counter` is our live view and we initialize it with the default values.
    let counter = Counter::default();

    live.response(|embed_live_view| {
        html! {
            <!DOCTYPE html>
            <html>
                <head>
                </head>
                <body>
                    // Embed our live view into the HTML template. This will render the
                    // view and include the HTML in the response, leading to good SEO
                    // and fast first paint.
                    { embed_live_view.embed(counter) }

                    // Load the JavaScript. This will automatically initialize live view
                    // connections.
                    <script src="/assets/live-view.js"></script>
                </body>
            </html>
        }
    })
}

// Our live view is just a regular Rust struct...
#[derive(Default)]
struct Counter {
    count: u64,
}

// ...that implements the `LiveView` trait.
impl LiveView for Counter {
    // This is the type of update messages our HTML contains. They will be sent
    // to the view in the `update` method
    type Message = Msg;

    // Update the view based on which message it receives.
    //
    // `EventData` contains data from the event that happened in the
    // browser. This might be values of input fields or which key was pressed in
    // a keyboard event.
    fn update(
        mut self,
        msg: Msg,
        data: Option<EventData>,
    ) -> Updated<Self> {
        match msg {
            Msg::Increment => {
                self.count += 1;
            }
            Msg::Decrement => {
                if self.count > 0 {
                    self.count -= 1;
                }
            }
        }

        Updated::new(self)
    }

    // Render the live view into an HTML template. This function is called during
    // the initial render in `LiveViewManager::embed` and for each subsequent
    // update.
    //
    // The HTML is diff'ed on the server and only minimal deltas are sent over
    // the wire. The browser then builds the full HTML template and efficiently
    // updates the DOM.
    fn render(&self) -> Html<Self::Message> {
        html! {
            <div>
                "Counter value: "
                // Embed dynamic Rust values into the HTML.
                //
                // `if`, `for`, and `match` are also supported.
                { self.count }
            </div>

            <div>
                // Elements with the `axm-click` attribute will send an update message
                // to the view which calls `update` after which the view is
                // re-rendered.
                <button axm-click={ Msg::Increment }>"+"</button>
                <button axm-click={ Msg::Decrement }>"-"</button>
            </div>
        }
    }

    // The `LiveView` trait also has a `mount` method that is called when a new
    // WebSocket connects. This can be used to perform auth, load data that
    // isn't needed for the first response, and spawn a task that can send
    // messages to the view itself from other parts of the application.
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Msg {
    Increment,
    Decrement,
}
```

[phx]: https://github.com/phoenixframework/phoenix_live_view
[axum]: https://github.com/tokio-rs/axum
