# axum-liveview

axum-liveview allows you to build rich, real-time experiences with
server-rendered HTML. This is done entirely in Rust - no JavaScript or WASM
needed.

Basically [Phoenix LiveView][phx] but for [axum].

# 🚨 BIG SCARY WARNING 🚨

This project is still very much work in progress. Everything is subject to
change and you shouldn't use this for anything serious.

Contributions are welcome, but little work has currently been done to make the
repo approachable to contributors. That will improve as we do more work and get
closer to an eventual 0.1 release.

# Example usage

This is what using axum-liveview looks like.

```rust
use axum::{routing::get, Router};
use axum_liveview::{html, Html, LiveView, LiveViewManager, bindings::Axm, Setup};

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
    // `LiveViewManager` is an extractor that is hooked up to the liveview setup
    // and enables you to embed liveviews into HTML templates.
    live: LiveViewManager,
) -> Html {
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
                { live.embed(counter) }

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
impl LiveView for Counter {
    // Setup our liveview by specifying which pubsub topics we want to subscribe to
    // and which callbacks to associated with each topic.
    fn setup(&self, setup: &mut Setup<Self>) {
        // `on` is for subscribing to a topic local to this liveview instance.
        // This is how you subscribe to events from the browser.
        //
        // There is also `on_broadcast` for subscribing to events from other
        // parts of the application.
        setup.on("increment", Self::increment);
        setup.on("decrement", Self::decrement);
    }

    // Render the liveview into an HTML template. This function is called during
    // the initial render in `LiveViewManager::embed` and for each subsequent
    // update.
    //
    // The HTML is diff'ed on the server and only minimal deltas are sent over
    // the wire. The browser then builds the full HTML template and efficiently
    // updates the DOM.
    fn render(&self) -> Html {
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
                <button { Amx::Click }="increment">"+"</button>
                <button { Amx::Click }="decrement">"-"</button>
            </div>
        }
    }
}

// The callbacks that will be called when there are new messages on the pubsub
// topics we subscribed to in `LiveView::setup`
impl Counter {
    async fn increment(mut self) -> Self {
        self.count += 1;
        self
    }

    async fn decrement(mut self) -> Self {
        if self.count > 0 {
            self.count -= 1;
        }
        self
    }
}
```

[phx]: https://github.com/phoenixframework/phoenix_live_view
[axum]: https://github.com/tokio-rs/axum
