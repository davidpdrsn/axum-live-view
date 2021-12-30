#![allow(unused_imports)]

use axum::{async_trait, response::IntoResponse, routing::get, Router};
use axum_liveview::{
    html, liveview::Updated, AssociatedData, EmbedLiveView, Html, LiveView, Subscriptions,
};
use serde::{Deserialize, Serialize};
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

async fn root(embed_liveview: EmbedLiveView) -> impl IntoResponse {
    let counter = Counter::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                { axum_liveview::assets() }
                <style>
                    r#"
                        .hide {
                            display: none;
                        }
                    "#
                </style>
            </head>
            <body>
                { embed_liveview.embed(counter) }

                <script>
                    r#"
                        const liveView = new LiveView({
                            host: 'localhost',
                            port: 4000,
                        })
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
