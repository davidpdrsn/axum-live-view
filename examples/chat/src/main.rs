use axum::{
    async_trait,
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    AddExtensionLayer, Router,
};
use axum_live_view::{html, live_view, EventData, Html, LiveView, LiveViewUpgrade, Updated};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let messages: Messages = Default::default();

    // {
    //     let pubsub = pubsub.clone();
    //     let messages = messages.clone();
    //     let mut new_messages = pubsub.subscribe(&NewMessageTopic).await.unwrap();
    //     tokio::spawn(async move {
    //         while let Some(Json(msg)) = new_messages.next().await {
    //             messages.lock().unwrap().push(msg);
    //             let _ = pubsub.broadcast(&ReRenderMessageList, ()).await;
    //         }
    //     });
    // }

    let app = Router::new()
        .route("/", get(root))
        .route(
            "/bundle.js",
            get_service(ServeFile::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist/bundle.js"),
            ))
            .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .layer(AddExtensionLayer::new(messages));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type Messages = Arc<Mutex<Vec<Message>>>;

async fn root(
    live: LiveViewUpgrade,
    Extension(messages): Extension<Messages>,
) -> impl IntoResponse {
    let list = MessagesList { messages };

    let form = SendMessageForm {
        message: Default::default(),
        name: Default::default(),
    };

    let combined = live_view::combine((list, form), |list, form| {
        html! {
            { list }
            { form }
        }
    });

    live.response(move |embed| {
        html! {
            <!DOCTYPE html>
            <html>
                <head>
                </head>
                <body>
                    { embed.embed(combined) }
                    <script src="/bundle.js"></script>
                </body>
            </html>
        }
    })
}

#[derive(Clone)]
struct MessagesList {
    messages: Arc<Mutex<Vec<Message>>>,
}

#[async_trait]
impl LiveView for MessagesList {
    type Message = ();
    type Error = Infallible;

    async fn update(
        mut self,
        _msg: (),
        _data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        Ok(Updated::new(self))
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

struct SendMessageForm {
    message: String,
    name: String,
}

#[async_trait]
impl LiveView for SendMessageForm {
    type Message = FormMsg;
    type Error = Infallible;

    async fn update(
        mut self,
        msg: FormMsg,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        dbg!((msg, data));

        let mut js_commands = Vec::new();

        // match msg {
        //     FormMsg::Submit => {
        //         let new_msg = serde_json::json!(data.as_form().unwrap());
        //         let new_msg = serde_json::from_value::<Message>(new_msg).unwrap();

        //         let _ = self.pubsub.broadcast(&NewMessageTopic, Json(new_msg)).await;

        //         self.message.clear();
        //         js_commands.push(js_command::clear_value("#text-input"));
        //     }
        //     FormMsg::MessageChange => match data.as_form().unwrap() {
        //         FormEventData::String(value) => {
        //             self.message = value.to_owned();
        //         }
        //         other => {
        //             tracing::error!(
        //                 ?other,
        //                 "unexpected value type for `FormMsg::MessageChange` event"
        //             )
        //         }
        //     },
        //     FormMsg::NameChange => match data.as_form().unwrap() {
        //         FormEventData::String(value) => {
        //             self.name = value.to_owned();
        //         }
        //         other => {
        //             tracing::error!(
        //                 ?other,
        //                 "unexpected value type for `FormMsg::NameChange` event"
        //             )
        //         }
        //     },
        // }

        Ok(Updated::new(self).with_all(js_commands))
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
