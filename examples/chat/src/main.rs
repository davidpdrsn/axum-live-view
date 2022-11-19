use axum::{
    extract::Extension,
    http::{HeaderMap, Uri},
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_live_view::{
    event_data::EventData,
    html, js_command,
    live_view::{self, Updated, ViewHandle},
    Html, LiveView, LiveViewUpgrade,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let messages: Messages = Default::default();

    let (tx, _) = broadcast::channel::<NewMessagePing>(1024);

    let app = Router::new()
        .route("/", get(root))
        .route("/bundle.js", axum_live_view::precompiled_js())
        .layer(
            ServiceBuilder::new()
                .layer(Extension(messages))
                .layer(Extension(tx)),
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

impl LiveView for MessagesList {
    type Message = ();

    fn mount(&mut self, _: Uri, _: &HeaderMap, handle: ViewHandle<Self::Message>) {
        let mut rx = self.tx.subscribe();
        tokio::spawn(async move {
            while let Ok(NewMessagePing) = rx.recv().await {
                if handle.send(()).await.is_err() {
                    break;
                }
            }
        });
    }

    fn update(mut self, _msg: (), _data: Option<EventData>) -> Updated<Self> {
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

struct SendMessageForm {
    message: String,
    name: String,
    messages: Messages,
    tx: broadcast::Sender<NewMessagePing>,
}

impl LiveView for SendMessageForm {
    type Message = FormMsg;

    fn update(mut self, msg: FormMsg, data: Option<EventData>) -> Updated<Self> {
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
