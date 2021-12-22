use crate::{
    html::{self, Diff},
    liveview::topics,
    pubsub::{Decode, Encode, PubSub},
    LiveViewManager,
};
use axum::{
    extract::ws::{self, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures_util::{stream::BoxStream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use tokio_stream::StreamMap;
use uuid::Uuid;

pub(crate) fn routes<B>() -> Router<B>
where
    B: Send + 'static,
{
    Router::new().route("/live", get(ws))
}

async fn ws(upgrade: WebSocketUpgrade, live: LiveViewManager) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| handle_socket(socket, live.pubsub))
}

#[derive(Default)]
struct SocketState {
    diff_streams: StreamMap<Uuid, BoxStream<'static, Diff>>,
    js_command_streams: StreamMap<Uuid, BoxStream<'static, JsCommand>>,
}

async fn handle_socket<P>(mut socket: WebSocket, pubsub: P)
where
    P: PubSub,
{
    let mut state = SocketState::default();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(msg) => {
                        if let Some((liveview_id, html)) = handle_message_from_socket(msg, &pubsub, &mut state).await {
                            if send_message_to_socket(&mut socket, liveview_id, INITIAL_RENDER_TOPIC, html).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        tracing::trace!(%err, "error from socket");
                        break;
                    }
                }
            }

            Some((liveview_id, diff)) = state.diff_streams.next() => {
                if send_message_to_socket(&mut socket, liveview_id, RENDERED_TOPIC, diff).await.is_err() {
                    break;
                }
            }

            Some((liveview_id, js_command)) = state.js_command_streams.next() => {
                let msg = match js_command {
                    JsCommand::NavigateTo { uri } => json!({
                        "type": "navigate_to",
                        "data": {
                            "uri": uri,
                        }
                    }),
                };

                if send_message_to_socket(&mut socket, liveview_id, JS_COMMAND_TOPIC, msg).await.is_err() {
                    break;
                }
            }
        }
    }

    let liveview_ids = state
        .diff_streams
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();

    for liveview_id in liveview_ids {
        let _ = pubsub
            .broadcast(&topics::socket_disconnected(liveview_id), ())
            .await;
    }
}

const INITIAL_RENDER_TOPIC: &str = "i";
const RENDERED_TOPIC: &str = "r";
const JS_COMMAND_TOPIC: &str = "j";

async fn send_message_to_socket<T>(
    socket: &mut WebSocket,
    liveview_id: Uuid,
    topic: &'static str,
    msg: T,
) -> Result<(), axum::Error>
where
    T: serde::Serialize,
{
    let msg = json!([liveview_id, topic, msg,]);
    let msg = serde_json::to_string(&msg).unwrap();
    tracing::trace!(%msg, "sending message to websocket");

    socket.send(ws::Message::Text(msg)).await
}

async fn handle_message_from_socket<P>(
    msg: ws::Message,
    pubsub: &P,
    state: &mut SocketState,
) -> Option<(Uuid, html::Serialized)>
where
    P: PubSub,
{
    macro_rules! tri {
        ($expr:expr, $pattern:path $(,)?) => {
            match $expr {
                $pattern(inner) => inner,
                other => {
                    tracing::error!(line = line!(), ?other);
                    return None;
                }
            }
        };
    }

    let text = tri!(msg, ws::Message::Text);
    let msg: RawMessage = tri!(serde_json::from_str(&text), Ok);
    let liveview_id = msg.liveview_id;
    let msg = tri!(EventBindingMessage::try_from(msg), Ok);

    tracing::trace!(?msg, "received message from websocket");

    match msg {
        EventBindingMessage::Mount => {
            let mut initial_render_stream = pubsub
                .subscribe::<Json<html::Serialized>>(&topics::initial_render(liveview_id))
                .await;

            tri!(
                pubsub.broadcast(&topics::mounted(liveview_id), ()).await,
                Ok,
            );

            let Json(msg) = tri!(initial_render_stream.next().await, Some);

            let diff_stream = pubsub
                .subscribe::<Json<Diff>>(&topics::rendered(liveview_id))
                .await
                .map(|Json(diff)| diff);
            state
                .diff_streams
                .insert(liveview_id, Box::pin(diff_stream));

            let js_command_stream = pubsub
                .subscribe::<Json<JsCommand>>(&topics::js_command(liveview_id))
                .await
                .map(|Json(js_command)| js_command);
            state
                .js_command_streams
                .insert(liveview_id, Box::pin(js_command_stream));

            return Some((liveview_id, msg));
        }
        EventBindingMessage::Click(Click { event_name, data }) => {
            let topic = topics::liveview_local(liveview_id, &event_name);
            if let Some(data) = data {
                tri!(pubsub.broadcast(&topic, axum::Json(data)).await, Ok);
            } else {
                tri!(pubsub.broadcast(&topic, ()).await, Ok);
            }
        }
        EventBindingMessage::FormEvent(FormEventMessage {
            event_name,
            data,
            value,
        }) => {
            let topic = topics::liveview_local(liveview_id, &event_name);
            let data = if let Some(data) = data {
                match serde_json::from_value::<Value>(data.clone()) {
                    Ok(data) => data,
                    Err(err) => {
                        tracing::warn!(
                            "invalid data from `InputEventMessage`: {:?}. Error: {}",
                            data,
                            err
                        );
                        return None;
                    }
                }
            } else {
                Default::default()
            };
            tri!(
                pubsub.broadcast(&topic, FormEvent { value, data }).await,
                Ok
            );
        }
    }

    None
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FormEvent<V = String, D = ()> {
    value: V,
    data: D,
}

impl<V, D> FormEvent<V, D> {
    pub fn value(&self) -> &V {
        &self.value
    }

    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn into_value(self) -> V {
        self.value
    }

    pub fn into_data(self) -> D {
        self.data
    }

    pub fn into_parts(self) -> (V, D) {
        (self.value, self.data)
    }
}

impl<V, D> Encode for FormEvent<V, D>
where
    V: Serialize,
    D: Serialize,
{
    fn encode(&self) -> anyhow::Result<bytes::Bytes> {
        axum::Json(self).encode()
    }
}

impl<V, D> Decode for FormEvent<V, D>
where
    V: DeserializeOwned,
    D: DeserializeOwned,
{
    fn decode(msg: bytes::Bytes) -> anyhow::Result<Self> {
        Ok(axum::Json::<Self>::decode(msg)?.0)
    }
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    liveview_id: Uuid,
    topic: String,
    data: Value,
}

impl TryFrom<RawMessage> for EventBindingMessage {
    type Error = anyhow::Error;

    fn try_from(value: RawMessage) -> Result<Self, Self::Error> {
        let RawMessage {
            topic,
            data,
            liveview_id: _,
        } = value;

        match &*topic {
            "axum/mount-liveview" => Ok(EventBindingMessage::Mount),
            "axum/live-click" => Ok(EventBindingMessage::Click(from_value(data)?)),
            "axum/live-input" | "axum/live-change" | "axum/live-focus" | "axum/live-blur"
            | "axum/live-submit" => Ok(EventBindingMessage::FormEvent(from_value(data)?)),
            other => {
                anyhow::bail!("unknown message topic: {:?}", other)
            }
        }
    }
}

#[derive(Debug)]
enum EventBindingMessage {
    Mount,
    Click(Click),
    FormEvent(FormEventMessage),
}

#[derive(Debug, Deserialize)]
struct Click {
    #[serde(rename = "e")]
    event_name: String,
    #[serde(rename = "d")]
    data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct FormEventMessage {
    #[serde(rename = "e")]
    event_name: String,
    #[serde(rename = "v")]
    value: Value,
    #[serde(rename = "d")]
    data: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum JsCommand {
    NavigateTo { uri: String },
}
