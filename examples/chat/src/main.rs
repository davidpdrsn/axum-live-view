use axum::{
    async_trait, extract::Extension, response::IntoResponse, routing::get, AddExtensionLayer, Json,
    Router,
};
use axum_liveview::{
    associated_data::FormEventValue,
    html,
    liveview::Updated,
    pubsub::{InProcess, Topic},
    AssociatedData, EmbedLiveView, Html, LiveView, PubSub, Subscriptions,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower::ServiceBuilder;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = InProcess::new();

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
        .merge(axum_liveview::routes())
        .layer(
            ServiceBuilder::new()
                .layer(axum_liveview::layer(pubsub.clone()))
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
    embed_liveview: EmbedLiveView,
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
                { axum_liveview::assets() }
            </head>
            <body>
                { embed_liveview.embed(list).unit() }
                { embed_liveview.embed(form).unit() }
                <script>
                    r#"
                        const liveView = new LiveView({ host: 'localhost', port: 4000 })
                        liveView.connect()
                    "#
                </script>
            </body>
        </html>
    }
}

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

    async fn update(mut self, _msg: (), _data: AssociatedData) -> Updated<Self> {
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

    async fn update(mut self, msg: FormMsg, data: AssociatedData) -> Updated<Self> {
        match msg {
            FormMsg::Submit => {
                let new_msg = serde_json::json!(data.as_form().unwrap());
                let new_msg = serde_json::from_value::<Message>(new_msg).unwrap();

                let _ = self.pubsub.broadcast(&NewMessageTopic, Json(new_msg)).await;

                // TODO(david): clear the input as well
                self.message.clear();
            }
            FormMsg::MessageChange => match data.as_form().unwrap() {
                FormEventValue::String(value) => {
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
                FormEventValue::String(value) => {
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

        Updated::new(self)
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
