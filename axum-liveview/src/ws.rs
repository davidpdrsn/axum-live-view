use std::collections::HashMap;

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
use serde::{Deserialize, Serialize};
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
    macro_rules! try_ {
        ($expr:expr, $pattern:path $(,)?) => {
            match $expr {
                $pattern(inner) => inner,
                other => {
                    tracing::error!(?other);
                    return None;
                }
            }
        };
    }

    let text = try_!(msg, ws::Message::Text);
    let msg: RawMessage = try_!(serde_json::from_str(&text), Ok);
    let liveview_id = msg.liveview_id;
    let msg = try_!(EventBindingMessage::try_from(msg), Ok);

    tracing::trace!(?msg, "received message from websocket");

    match msg {
        EventBindingMessage::Mount => {
            let mut initial_render_stream = pubsub
                .subscribe::<Json<html::Serialized>>(&topics::initial_render(liveview_id))
                .await;

            try_!(
                pubsub.broadcast(&topics::mounted(liveview_id), ()).await,
                Ok,
            );

            let Json(msg) = try_!(initial_render_stream.next().await, Some);

            let diff_stream = pubsub
                .subscribe::<Json<Diff>>(&topics::rendered(liveview_id))
                .await
                .map(|Json(diff)| diff);

            state
                .diff_streams
                .insert(liveview_id, Box::pin(diff_stream));

            return Some((liveview_id, msg));
        }
        EventBindingMessage::Click(Click { event_name, data }) => {
            let topic = topics::liveview_local(liveview_id, &event_name);
            if let Some(data) = data {
                try_!(pubsub.broadcast(&topic, axum::Json(data)).await, Ok);
            } else {
                try_!(pubsub.broadcast(&topic, ()).await, Ok);
            }
        }
        EventBindingMessage::InputEvent(InputEventMessage {
            event_name,
            data,
            value,
        }) => {
            let topic = topics::liveview_local(liveview_id, &event_name);
            let data = if let Some(data) = data {
                serde_json::from_value(data.clone())
                    .unwrap_or_else(|_| panic!("invalid data from `InputEventMessage`: {:?}", data))
            } else {
                Default::default()
            };
            try_!(
                pubsub.broadcast(&topic, InputEvent { value, data }).await,
                Ok
            );
        }
    }

    None
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InputEvent {
    value: String,
    data: HashMap<String, String>,
}

impl InputEvent {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }
}

impl Encode for InputEvent {
    fn encode(&self) -> anyhow::Result<bytes::Bytes> {
        axum::Json(self).encode()
    }
}

impl Decode for InputEvent {
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
            "axum/live-input" | "axum/live-change" | "axum/live-focus" | "axum/live-blur" => {
                Ok(EventBindingMessage::InputEvent(from_value(data)?))
            }
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
    InputEvent(InputEventMessage),
}

#[derive(Debug, Deserialize)]
struct Click {
    #[serde(rename = "e")]
    event_name: String,
    #[serde(rename = "d")]
    data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct InputEventMessage {
    #[serde(rename = "e")]
    event_name: String,
    #[serde(rename = "v")]
    value: String,
    #[serde(rename = "d")]
    data: Option<Value>,
}
