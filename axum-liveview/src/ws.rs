use crate::{liveview::liveview_local_topic, pubsub::PubSub, LiveViewManager, PubSubExt};
use axum::{
    extract::ws::{self, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{stream::BoxStream, StreamExt};
use serde::Deserialize;
use serde_json::{from_value, Value};
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
    markup_streams: StreamMap<Uuid, BoxStream<'static, String>>,
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

            Some((liveview_id, html)) = state.markup_streams.next() => {
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
            .broadcast(
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
    macro_rules! try_ {
        ($expr:expr, $pattern:path) => {
            match $expr {
                $pattern(inner) => inner,
                other => {
                    tracing::error!(?other);
                    return;
                }
            }
        };
    }

    let text = try_!(msg, ws::Message::Text);
    let msg: RawMessage = try_!(serde_json::from_str(&text), Ok);
    let liveview_id = msg.liveview_id;
    let msg = try_!(Message::try_from(msg), Ok);

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
        Message::LiveClick(LiveClick { event_name, additional_data }) => {
            let topic = liveview_local_topic(liveview_id, &event_name);
            if let Some(additional_data) = additional_data {
                try_!(pubsub.broadcast(&topic, axum::Json(additional_data)).await, Ok);
            } else {
                try_!(pubsub.broadcast(&topic, ()).await, Ok);
            }
        }
        Message::LiveInput(LiveInput { event_name, value }) => {
            let topic = liveview_local_topic(liveview_id, &event_name);
            try_!(pubsub.broadcast(&topic, value).await, Ok);
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
        let RawMessage {
            topic,
            data,
            liveview_id: _,
        } = value;

        match &*topic {
            "axum/mount-liveview" => Ok(Message::Mount),
            "axum/live-click" => Ok(Message::LiveClick(from_value(data)?)),
            "axum/live-input" => Ok(Message::LiveInput(from_value(data)?)),
            other => {
                anyhow::bail!("unknown message topic: {:?}", other)
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Mount,
    LiveClick(LiveClick),
    LiveInput(LiveInput),
}

#[derive(Debug, Deserialize)]
struct LiveClick {
    event_name: String,
    additional_data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct LiveInput {
    event_name: String,
    value: String,
}
