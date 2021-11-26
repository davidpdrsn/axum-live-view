use axum::{
    async_trait,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_liveview::{LiveView, LiveViewManager, Subscriptions};
use maud::{html, Markup};
use std::net::SocketAddr;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let pubsub = axum_liveview::pubsub::InProcess::new();

    let app = Router::new()
        .route("/", get(root))
        .merge(axum_liveview::routes())
        .layer(axum_liveview::layer(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr, _>())
        .await
        .unwrap();
}

async fn root(live: LiveViewManager) -> impl IntoResponse {
    let counter = Counter::default();

    Html(
        html! {
            (maud::DOCTYPE)
            html {
                head {
                    (axum_liveview::assets())
                }
                body {
                    (live.embed(counter))
                }
            }
        }
        .into_string(),
    )
}

#[derive(Default)]
struct Counter {
    count: u64,
}

#[async_trait]
impl LiveView for Counter {
    fn setup(sub: &mut Subscriptions<Self>) {
        sub.on("increment", |mut this, ()| async move {
            this.count += 1;
            this
        })
        .on("decrement", |mut this, ()| async move {
            this.count -= 1;
            this
        });
    }

    fn render(&self) -> Markup {
        html! {
            div {
                (self.count)
            }
            div {
                button live-click="increment" { "+" }
                button live-click="decrement" { "-" }
            }
        }
    }
}
