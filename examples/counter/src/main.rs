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

    let app = Router::new().route("/", get(root)).route(
        "/app.js",
        get_service(ServeFile::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist/bundle.js"),
        ))
        .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
    );

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
            Msg::IncrBy(n) => self.count += n,
            Msg::Decr => {
                if self.count > 0 {
                    self.count -= 1;
                }
            }
        }

        let set_title = axum_live_view::js_command::set_title(format!("Count: {}", self.count));
        Updated::new(self).with(set_title)
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <div>
                <button axm-click={ Msg::Incr }>"+"</button>
                <button axm-click={ Msg::Decr }>"-"</button>
            </div>

            if self.count >= 3 {
                <div>
                    <button axm-click={ Msg::IncrBy(10) }>"+10"</button>
                </div>
            }

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
    IncrBy(u64),
}
