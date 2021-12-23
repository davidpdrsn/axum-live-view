use axum::{response::IntoResponse, routing::get, Router};
use axum_liveview::{bindings::axm, html, Html, LiveView, LiveViewManager, Setup};
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
    let counter = Counter::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                { axum_liveview::assets() }
            </head>
            <body>
                { live.embed(counter) }
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
struct Counter {
    count: u64,
}

impl LiveView for Counter {
    fn setup(&self, subscriptions: &mut Setup<Self>) {
        subscriptions.on("incr", Self::increment);
        subscriptions.on("decr", Self::decrement);
    }

    fn render(&self) -> Html {
        html! {
            <div>
                <button { axm::click() }={ "incr" }>"+"</button>
                <button { axm::click() }={ "decr" }>"-"</button>
            </div>

            <div>
                "Counter value: "
                { self.count }
            </div>
        }
    }
}

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
