use axum::{
    async_trait,
    extract::Extension,
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
    routing::{get, get_service},
    AddExtensionLayer, Router,
};
use axum_live_view::{
    event_data::EventData,
    html, js_command,
    life_cycle::SelfHandle,
    live_view::{self, Updated},
    Html, LiveView, LiveViewUpgrade,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let messages: Messages = Default::default();

    let (tx, _) = broadcast::channel::<NewMessagePing>(1024);

    let app = Router::new()
        .route("/", get(root))
        .route(
            "/bundle.js",
            get_service(ServeFile::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist/bundle.js"),
            ))
            .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .layer(
            ServiceBuilder::new()
                .layer(AddExtensionLayer::new(messages))
                .layer(AddExtensionLayer::new(tx)),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type Messages = Arc<Mutex<Vec<Message>>>;

#[derive(Clone, Copy)]
struct NewMessagePing;

async fn root(
    live: LiveViewUpgrade,
    Extension(messages): Extension<Messages>,
    Extension(tx): Extension<broadcast::Sender<NewMessagePing>>,
) -> impl IntoResponse {
    let list = MessagesList {
        messages: messages.clone(),
        tx: tx.clone(),
    };

    let form = SendMessageForm {
        message: Default::default(),
        name: Default::default(),
        messages,
        tx,
    };

    let combined = live_view::combine((list, form), |list, form| {
        html! {
            { list }
            <hr />
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

struct MessagesList {
    messages: Messages,
    tx: broadcast::Sender<NewMessagePing>,
}

#[async_trait]
impl LiveView for MessagesList {
    type Message = ();
    type Error = Infallible;

    async fn mount(
        &mut self,
        _: Uri,
        _: &HeaderMap,
        handle: SelfHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        let mut rx = self.tx.subscribe();
        tokio::spawn(async move {
            while let Ok(NewMessagePing) = rx.recv().await {
                if handle.send(()).await.is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

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
    messages: Messages,
    tx: broadcast::Sender<NewMessagePing>,
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
        let mut js_commands = Vec::new();

        match msg {
            FormMsg::Submit => {
                let new_msg = data
                    .unwrap()
                    .as_form()
                    .unwrap()
                    .deserialize::<Message>()
                    .unwrap();

                self.messages.lock().unwrap().push(new_msg);
                let _ = self.tx.send(NewMessagePing);

                self.message.clear();
                js_commands.push(js_command::clear_value("#text-input"));
            }
            FormMsg::MessageChange => {
                self.message = data
                    .unwrap()
                    .as_input()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
            }
            FormMsg::NameChange => {
                self.name = data
                    .unwrap()
                    .as_input()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
            }
        }

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
