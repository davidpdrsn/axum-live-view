use axum::{response::IntoResponse, routing::get, Router};
use axum_live_view::{
    event_data::EventData, html, live_view::Updated, Html, LiveView, LiveViewUpgrade,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/bundle.js", axum_live_view::precompiled_js());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(live: LiveViewUpgrade) -> impl IntoResponse {
    let view = Counter::default();

    live.response(move |embed| {
        html! {
            <!DOCTYPE html>
            <html>
                <head>
                </head>
                <body>
                    { embed.embed(view) }
                    <script src="/bundle.js"></script>
                </body>
            </html>
        }
    })
}

#[derive(Default, Clone)]
struct Counter {
    count: u64,
}

impl LiveView for Counter {
    type Message = Msg;

    fn update(mut self, msg: Msg, _data: Option<EventData>) -> Updated<Self> {
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
