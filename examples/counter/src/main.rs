use axum::{
    async_trait,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_live_view::{
    html, pubsub::InProcess, test, EmbedLiveView, EventData, Html, LiveView, Updated,
};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf};
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // let pubsub = InProcess::new();
    // let (live_view_routes, live_view_layer) = axum_live_view::router_parts(pubsub);

    let app = Router::new().route("/", get(root)).route(
        "/app.js",
        get_service(ServeFile::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist/bundle.js"),
        ))
        .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
    );
    // .merge(live_view_routes)
    // .layer(live_view_layer);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(live: test::LiveViewUpgrade) -> impl IntoResponse {
    let view = Counter::default();

    live.response(move |embed| {
        html! {
            <!DOCTYPE html>
            <html>
                <head>
                </head>
                <body>
                    { embed.embed(view) }
                    <script src="/app.js"></script>
                </body>
            </html>
        }
    })
}

#[derive(Default, Clone)]
struct Counter {
    count: u64,
}

#[async_trait]
impl LiveView for Counter {
    type Message = Msg;

    async fn update(mut self, msg: Msg, _data: EventData) -> Updated<Self> {
        match msg {
            Msg::Incr => self.count += 1,
            Msg::Decr => {
                if self.count > 0 {
                    self.count -= 1;
                }
            }
        }

        if self.count >= 10 {
            panic!();
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
