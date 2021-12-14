use axum::{response::IntoResponse, routing::get, Router};
use axum_liveview::{html, Html, LiveView, LiveViewManager, Subscriptions, PubSubExt};
use std::{time::{Instant, Duration}, net::SocketAddr};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let pubsub = axum_liveview::pubsub::InProcess::new();

    {
        let pubsub = pubsub.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1) / 30);
            loop {
                interval.tick().await;
                let _ = pubsub.broadcast(topics::ping, ()).await;
            }
        });
    }

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
    let counter = Counter::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                { axum_liveview::assets() }
            </head>
            <body>
                { live.embed(counter) }
                <script>
                    r#"
                        const liveView = new LiveView('localhost', 4000)
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
    previous_click: Option<Instant>,
}

impl LiveView for Counter {
    fn setup(&self, subscriptions: &mut Subscriptions<Self>) {
        subscriptions.on(topics::incr, Self::increment);
        subscriptions.on(topics::decr, Self::decrement);
        subscriptions.on_broadcast(topics::ping, Self::re_render);
    }

    fn render(&self) -> Html {
        html! {
            <div>
                <button live-click={ topics::incr }>"+"</button>
                <button live-click={ topics::decr }>"-"</button>
            </div>

            <div>
                if self.count == 0 {
                    "zero..."
                } else {
                    { self.count }
                }
            </div>

            if self.count % 10 == 0 {
                <div>"Divisble by 10!"</div>
            }

            if let Some(previous_click) = &self.previous_click {
                <div>{ format!("Your previous click as {:?} ago", previous_click.elapsed()) }</div>
            }
        }
    }
}

impl Counter {
    async fn increment(mut self) -> Self {
        self.count += 1;
        let _ = self.previous_click.insert(Instant::now());
        self
    }

    async fn decrement(mut self) -> Self {
        let _ = self.previous_click.insert(Instant::now());
        if self.count > 0 {
            self.count -= 1;
        }
        self
    }

    async fn re_render(self) -> Self {
        self
    }
}

mod topics {
    macro_rules! declare_topic {
        ($name:ident) => {
            #[allow(non_upper_case_globals)]
            pub const $name: &str = stringify!($name);
        };
    }

    declare_topic!(incr);
    declare_topic!(decr);
    declare_topic!(ping);
}
