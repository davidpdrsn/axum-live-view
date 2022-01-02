use axum::{
    async_trait,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_liveview::{
    html, liveview::Updated, AssociatedData, EmbedLiveView, Html, LiveView, Subscriptions,
};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf};
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = axum_liveview::pubsub::InProcess::new();

    let app = Router::new()
        .route("/", get(root))
        .route(
            "/bundle.js",
            get_service(ServeFile::new(
                PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("dist/bundle.js"),
            ))
            .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .merge(axum_liveview::routes())
        .layer(axum_liveview::LiveViewLayer::new(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(embed_liveview: EmbedLiveView) -> impl IntoResponse {
    let counter = Counter::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                <script src="/bundle.js"></script>
            </head>
            <body>
                { embed_liveview.embed(counter) }
            </body>
        </html>
    }
}

#[derive(Default)]
struct Counter {
    count: u64,
}

#[async_trait]
impl LiveView for Counter {
    type Message = Msg;

    fn init(&self, _subscriptions: &mut Subscriptions<Self>) {}

    async fn update(mut self, msg: Msg, _data: AssociatedData) -> Updated<Self> {
        match msg {
            Msg::Incr => self.count += 1,
            Msg::Decr => {
                if self.count > 0 {
                    self.count -= 1;
                }
            }
        }

        Updated::new(self)
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <div>
                <button axm-click={ Msg::Incr }>"+"</button>
                <button axm-click={ Msg::Decr }>"-"</button>
            </div>

            <div>
                "Counter value: "
                { self.count }
            </div>
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Msg {
    Incr,
    Decr,
}
