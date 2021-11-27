use axum::{
    async_trait,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_liveview::{LiveView, LiveViewManager, Subscriptions};
use maud::{html, Markup};
use std::net::SocketAddr;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let pubsub =
        axum_liveview::pubsub::Postgres::new("host=localhost dbname=foobar user=davidpdrsn")
            .await
            .unwrap();

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
        sub.on("increment", Self::increment)
            .on("decrement", Self::decrement);
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
