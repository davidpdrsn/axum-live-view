# axum-liveview

axum-liveview allows you to build rich, real-time experiences with
server-rendered HTML. This is done entirely in Rust - no JavaScript or WASM
needed.

Basically [Phoenix LiveView][phx] but for [axum].

# 🚨 BIG SCARY WARNING 🚨

This project is still very much work in progress. Everything is subject to
change and you shouldn't use this for anything serious.

# Example usage

This is what using axum-liveview looks like.

```rust
use axum::{async_trait, response::IntoResponse, routing::get, Router};
use axum_liveview::{html, AssociatedData, EmbedLiveView, Html, LiveView};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // liveviews must send and receive messages both from the browser and from
    // other parts of your application. `axum_liveview::pubsub` is how that is
    // done.
    //
    // It allows you publish messages to topics and subscribe to a stream
    // of messages in a particular topic.
    //
    // `InProcess` is a pubsub implementation that uses `tokio::sync::broadcast`.
    let pubsub = axum_liveview::pubsub::InProcess::new();

    // A normal axum router.
    let app = Router::new()
        .route("/", get(root))
        // liveview has a few routes of its own that you have to include.
        .merge(axum_liveview::routes())
        // liveview also has a middleware that you must include.
        .layer(axum_liveview::layer(pubsub));

    // We run the app just like any other axum app
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Our handler function for `GET /`
async fn root(
    // `EmbedLiveView` is an extractor that is hooked up to the liveview setup
    // and enables you to embed liveviews into HTML templates.
    embed_liveview: EmbedLiveView,
) -> impl IntoResponse {
    // `Counter` is our liveview and we initialize it with the default values.
    let counter = Counter::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                // axum-liveview comes with some assets that you must load.
                { axum_liveview::assets() }
            </head>
            <body>
                // Embed our liveview into the HTML template. This will render the
                // view and include the HTML in the response, leading to good SEO
                // and fast first paint.
                //
                // It will also start a stateful async task for updating the view
                // and sending the changes down to the client via a WebSocket
                // connection.
                { embed_liveview.embed(counter) }

                // This is all the JavaScript you need to write to initialize
                // the liveview connection and handle updates.
                <script>
                    r#"
                        const liveView = new LiveView({ host: 'localhost', port: 3000 })
                        liveView.connect()
                    "#
                </script>
            </body>
        </html>
    }
}

// Our liveview is just a regular Rust struct...
#[derive(Default)]
struct Counter {
    count: u64,
}

// ...that implements the `LiveView` trait.
#[async_trait]
impl LiveView for Counter {
    // This is the type of update messages our HTML contains. They will be sent
    // to the view in the `update` method
    type Message = Msg;

    // Update the view based on which message it receives.
    //
    // `AssociatedData` contains data from the event that happened in the
    // browser. This might be values of input fields or which key was pressed in
    // a keyboard event.
    async fn update(mut self, msg: Msg, data: AssociatedData) -> Self {
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

        self
    }

    // Render the liveview into an HTML template. This function is called during
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
                // Elements with the `axm-click` attribute will send a message
                // on the corresponding pubsub topic which will call a callback,
                // update the liveview state, and call `render` again.
                <button axm-click={ Msg::Increment }>"+"</button>
                <button axm-click={ Msg::Decrement }>"-"</button>
            </div>
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Msg {
    Increment,
    Decrement,
}
```

[phx]: https://github.com/phoenixframework/phoenix_live_view
[axum]: https://github.com/tokio-rs/axum
