#![allow(unused_imports)]

use axum::{
    async_trait,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use axum_liveview::{
    pubsub::PubSub, LiveView, LiveViewManager, PubSubExt, ShouldRender, Subscriptions,
};
use maud::{html, Markup};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use tokio::time::interval;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;

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
    let pubsub = live.pubsub();
    let counter = Counter {
        count: 0,
        pubsub,
    };

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

struct Counter<P> {
    count: u64,
    pubsub: P,
}

#[async_trait]
impl<P> LiveView for Counter<P>
where
    P: PubSub,
{
    fn setup(sub: &mut Subscriptions<Self>) {
        sub.on("increment", Self::increment)
            .on("decrement", Self::decrement)
            .on_global("incremented", Self::incremented)
            .on_global("decremented", Self::decremented);
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

impl<P> Counter<P>
where
    P: PubSub,
{
    async fn incremented(mut self) -> Self {
        self.count += 1;
        self
    }

    async fn decremented(mut self) -> Self {
        if self.count > 0 {
            self.count -= 1;
        }
        self
    }

    async fn increment(self) -> ShouldRender<Self> {
        let _ = self.pubsub.send("incremented", ()).await;
        ShouldRender::No(self)
    }

    async fn decrement(self) -> ShouldRender<Self> {
        let _ = self.pubsub.send("decremented", ()).await;
        ShouldRender::No(self)
    }
}
