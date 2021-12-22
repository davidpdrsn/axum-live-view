use axum::{response::IntoResponse, routing::get, Router};
use axum_liveview::{html, messages::KeyEvent, Html, LiveView, LiveViewManager, Setup};
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
    prev: Option<KeyEvent>,
}

impl LiveView for View {
    fn setup(&self, setup: &mut Setup<Self>) {
        setup.on("keydown", Self::key);
        setup.on("keyup", Self::key);
    }

    fn render(&self) -> Html {
        html! {
            <div>
                "Keydown"
                <br />
                <input type="text" axm-keydown="keydown" />
            </div>

            <div>
                "Keydown (w debounce)"
                <br />
                <input type="text" axm-keydown="keydown" axm-debounce="500" />
            </div>

            <div>
                "Keyup"
                <br />
                <input type="text" axm-keyup="keyup" />
            </div>

            <div>
                "Keyup (only escape)"
                <br />
                <input type="text" axm-keyup="keyup" axm-key="escape" />
            </div>

            if let Some(event) = &self.prev {
                <hr />
                <div>"Event count: " { self.count }</div>
                <pre>
                    <code>
                        { format!("{:#?}", event) }
                    </code>
                </pre>
            }
        }
    }
}

impl View {
    async fn key(mut self, event: KeyEvent) -> Self {
        self.prev = Some(event);
        self.count += 1;
        self
    }
}
