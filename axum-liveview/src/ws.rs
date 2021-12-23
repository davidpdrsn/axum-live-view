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
use std::time::Duration;
use tokio::time::Instant;
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

    const ONE_YEAR: Duration = Duration::from_secs(31_556_926);
    const HEARTBEAT_BOUNCE: Duration = Duration::from_secs(5);
    const HEARTBEAT_FREQUENCY: Duration = Duration::from_secs(5);
    const HEARTBEAT_MAX_FAILED_ATTEMPTS: usize = 5;

    let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_FREQUENCY);
    let mut failed_heartbeats = 0;
    let mut heartbeat_sent_at = Instant::now();

    let heartbeat_bounce = tokio::time::sleep(ONE_YEAR);
    tokio::pin!(heartbeat_bounce);

    loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                if failed_heartbeats >= HEARTBEAT_MAX_FAILED_ATTEMPTS {
                    tracing::debug!("failed too many heartbeats");
                    break;
                }

                if send_heartbeat(&mut socket).await.is_ok() {
                    heartbeat_sent_at = Instant::now();
                    heartbeat_bounce.as_mut().reset(Instant::now() + HEARTBEAT_BOUNCE);
                } else {
                    tracing::debug!("failed to send heartbeat");
                    failed_heartbeats += 1;
                }
            }

            _ = &mut heartbeat_bounce => {
                tracing::debug!("heartbeat didn't respond in the allocated time");
                heartbeat_bounce.as_mut().reset(Instant::now() + ONE_YEAR);
                failed_heartbeats += 1;
            }

            Some(Ok(msg)) = socket.recv() => {
                match handle_message_from_socket(msg, &pubsub, &mut state).await {
                    Some(HandledMessagedResult::Mounted(liveview_id, html)) => {
                        let _ = send_message_to_socket(
                            &mut socket,
                            liveview_id,
                            INITIAL_RENDER_TOPIC,
                            html,
                        )
                        .await;
                    },
                    Some(HandledMessagedResult::HeartbeatResponse) => {
                        tracing::trace!(
                            elapsed = ?heartbeat_sent_at.elapsed(),
                            "heartbeat came back",
                        );
                        heartbeat_bounce.as_mut().reset(Instant::now() + ONE_YEAR);
                        failed_heartbeats = 0;
                    }
                    None => {},
                }
            }

            Some((liveview_id, diff)) = state.diff_streams.next() => {
                let _ = send_message_to_socket(&mut socket, liveview_id, RENDERED_TOPIC, diff).await;
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

                let _ = send_message_to_socket(&mut socket, liveview_id, JS_COMMAND_TOPIC, msg).await;
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

async fn send_heartbeat(socket: &mut WebSocket) -> Result<(), axum::Error> {
    let msg = json!(["h"]);
    let msg = serde_json::to_string(&msg).unwrap();
    tracing::trace!("sending heartbeat");
    socket.send(ws::Message::Text(msg)).await?;

    Ok(())
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

enum HandledMessagedResult {
    Mounted(Uuid, html::Serialized),
    HeartbeatResponse,
}

async fn handle_message_from_socket<P>(
    msg: ws::Message,
    pubsub: &P,
    state: &mut SocketState,
) -> Option<HandledMessagedResult>
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
    let msg: RawMessageOrHeartbeat = tri!(serde_json::from_str(&text), Ok);

    let msg = match msg {
        RawMessageOrHeartbeat::HeartbeatResponse(heartbeat_response) => {
            if heartbeat_response.h != "ok" {
                tracing::debug!(?heartbeat_response, "invalid status in heartbeat response");
            }
            return Some(HandledMessagedResult::HeartbeatResponse);
        }
        RawMessageOrHeartbeat::RawMessage(msg) => msg,
    };

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

            return Some(HandledMessagedResult::Mounted(liveview_id, msg));
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
            let data = deserialize_data(data)?;
            tri!(
                pubsub.broadcast(&topic, FormEvent { value, data }).await,
                Ok
            );
        }
        EventBindingMessage::KeyEvent(KeyEventMessage {
            event_name,
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
            data,
        }) => {
            let topic = topics::liveview_local(liveview_id, &event_name);
            let data = deserialize_data(data)?;
            tri!(
                pubsub
                    .broadcast(
                        &topic,
                        KeyEvent {
                            key,
                            code,
                            alt,
                            ctrl,
                            shift,
                            meta,
                            data,
                        }
                    )
                    .await,
                Ok
            );
        }
    }

    None
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawMessageOrHeartbeat {
    HeartbeatResponse(HeartbeatResponse),
    RawMessage(RawMessage),
}

#[derive(Debug, Deserialize)]
struct HeartbeatResponse {
    h: String,
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

            "axum/axm-click" => Ok(EventBindingMessage::Click(from_value(data)?)),

            "axum/axm-input" | "axum/axm-change" | "axum/axm-focus" | "axum/axm-blur"
            | "axum/axm-submit" => Ok(EventBindingMessage::FormEvent(from_value(data)?)),

            "axum/axm-keydown"
            | "axum/axm-keyup"
            | "axum/axm-window-keyup"
            | "axum/axm-window-keydown" => Ok(EventBindingMessage::KeyEvent(from_value(data)?)),

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
    KeyEvent(KeyEventMessage),
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

#[derive(Debug, Deserialize)]
struct KeyEventMessage {
    #[serde(rename = "e")]
    event_name: String,
    #[serde(rename = "k")]
    key: String,
    #[serde(rename = "kc")]
    code: String,
    #[serde(rename = "a")]
    alt: bool,
    #[serde(rename = "c")]
    ctrl: bool,
    #[serde(rename = "s")]
    shift: bool,
    #[serde(rename = "m")]
    meta: bool,
    #[serde(rename = "d")]
    data: Option<Value>,
}

fn deserialize_data(data: Option<Value>) -> Option<Value> {
    if let Some(data) = data {
        match serde_json::from_value::<Value>(data.clone()) {
            Ok(data) => Some(data),
            Err(err) => {
                tracing::debug!(
                    "invalid data from `InputEventMessage`: {:?}. Error: {}",
                    data,
                    err
                );
                None
            }
        }
    } else {
        Some(Default::default())
    }
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

    pub fn into_value(self) -> V {
        self.value
    }

    pub fn data(&self) -> &D {
        &self.data
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

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEvent<D = ()> {
    key: String,
    code: String,
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
    data: D,
}

impl<D> KeyEvent<D> {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn code(&self) -> &str {
        &self.key
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn meta(&self) -> bool {
        self.meta
    }

    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn into_data(self) -> D {
        self.data
    }
}

impl<D> Encode for KeyEvent<D>
where
    D: Serialize,
{
    fn encode(&self) -> anyhow::Result<bytes::Bytes> {
        axum::Json(self).encode()
    }
}

impl<D> Decode for KeyEvent<D>
where
    D: DeserializeOwned,
{
    fn decode(msg: bytes::Bytes) -> anyhow::Result<Self> {
        Ok(axum::Json::<Self>::decode(msg)?.0)
    }
}
