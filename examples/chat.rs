use axum::{
    async_trait,
    extract::Extension,
    response::{Html, IntoResponse},
    routing::get,
    AddExtensionLayer, Json, Router,
};
use axum_liveview::{
    pubsub::{PubSub, PubSubExt},
    LiveView, LiveViewManager, ShouldRender, Subscriptions,
};
use maud::{html, Markup};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
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
        .layer(
            ServiceBuilder::new()
                .layer(axum_liveview::layer(pubsub))
                .layer(AddExtensionLayer::new(AppState::default())),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Default, Clone)]
struct AppState {
    messages: Arc<Mutex<Vec<Message>>>,
}

async fn root(live: LiveViewManager, Extension(state): Extension<AppState>) -> impl IntoResponse {
    let messages = state.messages.lock().to_vec();

    let chat = Chat {
        messages,
        persisted_messages: state.messages,
        working_message: Default::default(),
        user_id: Uuid::new_v4(),
        pubsub: live.pubsub(),
    };

    Html(
        html! {
            (maud::DOCTYPE)
            html {
                head {
                    (axum_liveview::assets())
                }
                body {
                    (live.embed(chat))
                }
            }
        }
        .into_string(),
    )
}

struct Chat<P> {
    messages: Vec<Message>,
    persisted_messages: Arc<Mutex<Vec<Message>>>,
    working_message: String,
    user_id: Uuid,
    pubsub: P,
}

#[async_trait]
impl<P> LiveView for Chat<P>
where
    P: PubSub,
{
    fn setup(subscriptions: &mut Subscriptions<Self>) {
        subscriptions
            .on("typed", Self::typed)
            .on("submit", Self::submit)
            .on_global("new_message", Self::new_message);
    }

    #[allow(clippy::branches_sharing_code)]
    fn render(&self) -> Markup {
        html! {
            form {
                input type="text" live-input="typed" value=(self.working_message) {}

                button live-click="submit" {
                    "Send"
                }
            }

            @if self.messages.is_empty() {
                p {
                    "Its quiet, too quiet..."
                }
            } @else {
                @for message in &self.messages {
                    @if message.sent_by == self.user_id {
                        div style="text-align: right" {
                            (message.text)
                        }
                    } @else {
                        div style="text-align: left" {
                            (message.text)
                        }
                    }
                }
            }
        }
    }
}

impl<P> Chat<P>
where
    P: PubSub,
{
    async fn typed(mut self, value: String) -> ShouldRender<Self> {
        self.working_message = value;
        ShouldRender::No(self)
    }

    async fn submit(mut self) -> Self {
        let text = std::mem::take(&mut self.working_message);
        if !text.is_empty() {
            let message = Message {
                text,
                sent_by: self.user_id,
            };
            self.persisted_messages.lock().push(message.clone());
            let _ = self.pubsub.broadcast("new_message", Json(message)).await;
        }
        self
    }

    async fn new_message(mut self, Json(message): Json<Message>) -> Self {
        self.messages.push(message);
        self
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    text: String,
    sent_by: Uuid,
}
