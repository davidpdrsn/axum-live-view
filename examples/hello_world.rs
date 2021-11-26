use axum::{
    async_trait,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_liveview::{LiveView, LiveViewManager, PubSubExt, ShouldRender, Subscriptions};
use maud::{html, Markup};
use std::{net::SocketAddr, time::Duration};

#[tokio::main]
async fn main() {
    let pubsub = axum_liveview::pubsub::InProcess::new();

    {
        let pubsub = pubsub.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                pubsub.send("ping", ()).await.unwrap();
            }
        });
    }

    let app = Router::new()
        .route("/", get(root))
        .merge(axum_liveview::routes())
        .layer(axum_liveview::layer(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(live: LiveViewManager) -> impl IntoResponse {
    let counter = Counter::default();

    Html(
        html! {
            (live.embed(counter))
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
        sub.on("increment", Counter::increment)
            .on("decrement", Counter::decrement)
            .on("ping", Counter::ping);
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
    async fn ping(self, _: ()) -> ShouldRender<Self> {
        println!("received ping!");
        ShouldRender::No(self)
    }

    async fn increment(self, _: ()) -> Self {
        self
    }

    async fn decrement(self, _: ()) -> Self {
        self
    }
}
