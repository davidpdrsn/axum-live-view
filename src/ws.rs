use crate::{liveview::liveview_local_topic, pubsub::PubSub, LiveViewManager, PubSubExt};
use axum::{
    extract::ws::{self, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use futures_util::{stream::BoxStream, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio_stream::StreamMap;
use uuid::Uuid;

pub(crate) fn routes<B>() -> Router<B>
where
    B: Send + 'static,
{
    Router::new().route("/live", get(ws))
}

async fn ws(upgrade: WebSocketUpgrade, live: LiveViewManager) -> impl IntoResponse {
    let pubsub = live.pubsub();
    upgrade.on_upgrade(move |socket| handle_socket(socket, pubsub))
}

#[derive(Default)]
struct SocketState {
    markup_streams: StreamMap<Uuid, BoxStream<'static, Bytes>>,
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
                        handle_message_from_socket(msg, &pubsub, &mut state).await;
                    }
                    Err(err) => {
                        tracing::trace!(%err, "error from socket");
                        break;
                    }
                }
            }

            Some((liveview_id, bytes)) = state.markup_streams.next() => {
                let html = if let Ok(str) = String::from_utf8(bytes.to_vec()) {
                    str
                } else {
                    tracing::error!("rendered liveview gave invalid UTF-8");
                    continue;
                };
                let msg = serde_json::json!({
                    "topic": "rendered",
                    "data": {
                        "liveview_id": liveview_id,
                        "html": html,
                    },
                });
                let msg = serde_json::to_string(&msg).unwrap();
                tracing::trace!(?msg, "sending message to websocket");
                if socket.send(ws::Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        }
    }

    let liveview_ids = state
        .markup_streams
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();

    for liveview_id in liveview_ids {
        let _ = pubsub
            .send(
                &liveview_local_topic(liveview_id, "socket-disconnected"),
                (),
            )
            .await;
    }
}

async fn handle_message_from_socket<P>(msg: ws::Message, pubsub: &P, state: &mut SocketState)
where
    P: PubSub,
{
    macro_rules! or_try {
        ($level:ident, $expr:expr, $pattern:path) => {
            match $expr {
                $pattern(inner) => inner,
                other => {
                    tracing::error!(?other);
                    return;
                }
            }
        };
    }

    let text = or_try!(error, msg, ws::Message::Text);
    let msg: RawMessage = or_try!(error, serde_json::from_str(&text), Ok);
    let liveview_id = msg.liveview_id;
    let msg = or_try!(error, Message::try_from(msg), Ok);

    tracing::trace!(?msg, "received message from websocket");

    match msg {
        Message::Mount => {
            let markup_stream = pubsub
                .subscribe(&liveview_local_topic(liveview_id, "rendered"))
                .await;
            state
                .markup_streams
                .insert(liveview_id, Box::pin(markup_stream));
        }
        Message::LiveViewEvent(event) => {
            let topic = liveview_local_topic(liveview_id, &event.event_name);
            or_try!(error, pubsub.send(&topic, ()).await, Ok);
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    liveview_id: Uuid,
    topic: String,
    data: Value,
}

impl TryFrom<RawMessage> for Message {
    type Error = anyhow::Error;

    fn try_from(value: RawMessage) -> Result<Self, Self::Error> {
        match &*value.topic {
            "axum/mount-liveview" => Ok(Message::Mount),
            "axum/liveview-event" => {
                Ok(Message::LiveViewEvent(serde_json::from_value(value.data)?))
            }
            other => {
                anyhow::bail!("unknown message topic: {:?}", other)
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Mount,
    LiveViewEvent(LiveViewEvent),
}

#[derive(Debug, Deserialize)]
struct LiveViewEvent {
    event_name: String,
}
