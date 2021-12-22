use axum::{response::IntoResponse, routing::get, Router};
use axum_liveview::{html, messages::KeyEvent, Html, LiveView, LiveViewManager, Setup};
use std::net::SocketAddr;
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_span_events(FmtSpan::ENTER)
        .init();

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
struct View {}

impl LiveView for View {
    fn setup(&self, setup: &mut Setup<Self>) {
        setup.on("keydown", Self::keydown);
    }

    fn render(&self) -> Html {
        html! {
            <input type="text" axm-keydown="keydown" />
        }
    }
}

impl View {
    #[tracing::instrument(skip(self))]
    async fn keydown(self, event: KeyEvent) -> Self {
        self
    }
}
