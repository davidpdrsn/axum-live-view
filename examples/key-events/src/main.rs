use axum::{response::IntoResponse, routing::get, Router};
use axum_liveview::{bindings::KeyEvent, html, Html, LiveView, LiveViewManager, Setup};
use serde::Deserialize;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = axum_liveview::pubsub::InProcess::new();

    let app = Router::new()
        .route("/", get(root))
        .merge(axum_liveview::routes())
        .layer(axum_liveview::layer(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr, _>())
        .await
        .unwrap();
}

async fn root(live: LiveViewManager) -> impl IntoResponse {
    let form = View::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                { axum_liveview::assets() }
            </head>
            <body>
                { live.embed(form) }
                <script>
                    r#"
                        const liveView = new LiveView({ host: 'localhost', port: 4000 })
                        liveView.connect()
                    "#
                </script>
            </body>
        </html>
    }
}

#[derive(Default)]
struct View {
    count: u64,
    prev: Option<KeyEvent<Data>>,
}

impl LiveView for View {
    fn setup(&self, setup: &mut Setup<Self>) {
        setup.on("key", Self::key);
    }

    fn render(&self) -> Html {
        html! {
            <div axm-window-keyup="key" axm-key="escape" axm-data-id="window-keyup">
                <div>
                    "Keydown"
                    <br />
                    <input type="text" axm-keydown="key" axm-data-id="keydown" />
                </div>

                <div>
                    "Keydown (w debounce)"
                    <br />
                    <input type="text" axm-keydown="key" axm-debounce="500" axm-data-id="keydown-w-debounce" />
                </div>

                <div>
                    "Keyup"
                    <br />
                    <input type="text" axm-keyup="key" axm-data-id="keyup" />
                </div>

                <hr />

                if let Some(event) = &self.prev {
                    <div>"Event count: " { self.count }</div>
                    <pre>
                        <code>
                            { format!("{:#?}", event) }
                        </code>
                    </pre>
                } else {
                    <div>
                        "No keys pressed yet"
                    </div>
                }
            </div>
        }
    }
}

impl View {
    async fn key(mut self, event: KeyEvent<Data>) -> Self {
        self.prev = Some(event);
        self.count += 1;
        self
    }
}

#[derive(Deserialize, Debug)]
struct Data {
    id: String,
}
