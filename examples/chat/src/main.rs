use axum::{
    async_trait,
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    AddExtensionLayer, Json, Router,
};
use axum_live_view::{
    html, js_command,
    live_view::{
        EmbedLiveView, EventData, FormEventData, LiveView, Shared, Subscriptions, Updated,
    },
    pubsub::{InProcess, PubSub, Topic},
    Html,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower::ServiceBuilder;
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = InProcess::new();
    let (live_view_routes, live_view_layer) = axum_live_view::router_parts(pubsub.clone());

    let messages: Messages = Default::default();

    {
        let pubsub = pubsub.clone();
        let messages = messages.clone();
        let mut new_messages = pubsub.subscribe(&NewMessageTopic).await.unwrap();
        tokio::spawn(async move {
            while let Some(Json(msg)) = new_messages.next().await {
                messages.lock().unwrap().push(msg);
                let _ = pubsub.broadcast(&ReRenderMessageList, ()).await;
            }
        });
    }

    let app = Router::new()
        .route("/", get(root))
        .route(
            "/bundle.js",
            get_service(ServeFile::new(
                PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("dist/bundle.js"),
            ))
            .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .merge(live_view_routes)
        .layer(
            ServiceBuilder::new()
                .layer(live_view_layer)
                .layer(AddExtensionLayer::new(pubsub))
                .layer(AddExtensionLayer::new(messages)),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type Messages = Arc<Mutex<Vec<Message>>>;

async fn root(
    embed_live_view: EmbedLiveView<InProcess>,
    Extension(pubsub): Extension<InProcess>,
    Extension(messages): Extension<Messages>,
) -> impl IntoResponse {
    let list = MessagesList { messages };

    let form = SendMessageForm {
        pubsub,
        message: Default::default(),
        name: Default::default(),
    };

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                <script src="/bundle.js"></script>
            </head>
            <body>
                { embed_live_view.embed(list).unit() }
                { embed_live_view.embed(form).unit() }
            </body>
        </html>
    }
}

#[derive(Clone)]
struct MessagesList {
    messages: Arc<Mutex<Vec<Message>>>,
}

#[async_trait]
impl LiveView for MessagesList {
    type Message = ();

    fn init(&self, subscriptions: &mut Subscriptions<Self>) {
        subscriptions.on(&ReRenderMessageList, |this, ()| async move {
            Updated::new(this)
        });
    }

    async fn update(mut self, _msg: (), _data: EventData) -> Updated<Self> {
        Updated::new(self)
    }

    fn render(&self) -> Html<Self::Message> {
        let messages = self.messages.lock().unwrap().clone();
        html! {
            if messages.is_empty() {
                <p>"Its quiet, too quiet..."</p>
            } else {
                <ul>
                    for msg in messages {
                        <li>
                            { &msg.name } ": "
                            <div>
                                { &msg.message }
                            </div>
                        </li>
                    }
                </ul>
            }
        }
    }
}

#[derive(Clone)]
struct SendMessageForm<P> {
    pubsub: P,
    message: String,
    name: String,
}

#[async_trait]
impl<P> LiveView for SendMessageForm<P>
where
    P: PubSub,
{
    type Message = FormMsg;

    fn init(&self, _subscriptions: &mut Subscriptions<Self>) {}

    async fn update(mut self, msg: FormMsg, data: EventData) -> Updated<Self> {
        let mut js_commands = Vec::new();

        match msg {
            FormMsg::Submit => {
                let new_msg = serde_json::json!(data.as_form().unwrap());
                let new_msg = serde_json::from_value::<Message>(new_msg).unwrap();

                let _ = self.pubsub.broadcast(&NewMessageTopic, Json(new_msg)).await;

                self.message.clear();
                js_commands.push(js_command::clear_value("#text-input"));
            }
            FormMsg::MessageChange => match data.as_form().unwrap() {
                FormEventData::String(value) => {
                    self.message = value.to_owned();
                }
                other => {
                    tracing::error!(
                        ?other,
                        "unexpected value type for `FormMsg::MessageChange` event"
                    )
                }
            },
            FormMsg::NameChange => match data.as_form().unwrap() {
                FormEventData::String(value) => {
                    self.name = value.to_owned();
                }
                other => {
                    tracing::error!(
                        ?other,
                        "unexpected value type for `FormMsg::NameChange` event"
                    )
                }
            },
        }

        Updated::new(self).with_all(js_commands)
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <form axm-submit={ FormMsg::Submit }>
                <input
                    type="text"
                    name="name"
                    placeholder="Your name"
                    axm-input={ FormMsg::NameChange }
                />

                <div>
                    <input
                        id="text-input"
                        type="text"
                        name="message"
                        placeholder="Message..."
                        axm-input={ FormMsg::MessageChange }
                    />

                    <input
                        type="submit"
                        value="Send!"
                        disabled=if self.message.is_empty() || self.name.is_empty() { Some(()) } else { None }
                    />
                </div>
            </form>
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum FormMsg {
    Submit,
    MessageChange,
    NameChange,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Message {
    name: String,
    message: String,
}

struct ReRenderMessageList;

impl Topic for ReRenderMessageList {
    type Message = ();

    fn topic(&self) -> &str {
        "re-render-message-list"
    }
}

struct NewMessageTopic;

impl Topic for NewMessageTopic {
    type Message = Json<Message>;

    fn topic(&self) -> &str {
        "new-message"
    }
}
