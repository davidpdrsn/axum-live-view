use crate::{
    html::{self, Diff},
    js::JsCommand,
    liveview::{embed::EmbedLiveView, LiveViewId},
    pubsub::PubSub,
    topics,
};
use anyhow::Context;
use axum::{
    extract::ws::{self, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures_util::{stream::BoxStream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use std::{collections::HashMap, time::Duration};
use tokio::time::{timeout, Instant};
use tokio_stream::StreamMap;

pub(crate) fn routes<B>() -> Router<B>
where
    B: Send + 'static,
{
    Router::new().route("/live", get(ws))
}

async fn ws(upgrade: WebSocketUpgrade, embed_liveview: EmbedLiveView) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| handle_socket(socket, embed_liveview.pubsub))
}

#[derive(Default)]
struct SocketState {
    diff_streams: StreamMap<LiveViewId, BoxStream<'static, (Option<Diff>, Vec<JsCommand>)>>,
}

async fn handle_socket<P>(mut socket: WebSocket, pubsub: P)
where
    P: PubSub,
{
    let mut state = SocketState::default();

    const A_VERY_LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24 * 365 * 10);
    const HEARTBEAT_BOUNCE: Duration = Duration::from_secs(5);
    const HEARTBEAT_FREQUENCY: Duration = Duration::from_secs(5);
    const HEARTBEAT_MAX_FAILED_ATTEMPTS: usize = 5;

    let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_FREQUENCY);
    let mut failed_heartbeats = 0;
    let mut heartbeat_sent_at = Instant::now();
    let mut heartbeat_inflight = false;

    let heartbeat_bounce = tokio::time::sleep(A_VERY_LONG_TIME);
    tokio::pin!(heartbeat_bounce);

    loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                // TODO(david): extract to function
                if failed_heartbeats >= HEARTBEAT_MAX_FAILED_ATTEMPTS {
                    tracing::debug!("failed too many heartbeats");
                    break;
                }

                if heartbeat_inflight { continue }

                if send_heartbeat(&mut socket).await.is_ok() {
                    heartbeat_inflight = true;
                    heartbeat_sent_at = Instant::now();
                    heartbeat_bounce.as_mut().reset(Instant::now() + HEARTBEAT_BOUNCE);
                } else {
                    tracing::debug!("failed to send heartbeat");
                    failed_heartbeats += 1;
                }
            }

            _ = &mut heartbeat_bounce => {
                heartbeat_inflight = false;
                tracing::debug!("heartbeat didn't respond in the allocated time");
                heartbeat_bounce.as_mut().reset(Instant::now() + A_VERY_LONG_TIME);
                failed_heartbeats += 1;
            }

            Some(Ok(msg)) = socket.recv() => {
                // TODO(david): extract to function
                match handle_message_from_socket(msg, &pubsub, &mut state).await {
                    Ok(Some(HandledMessagedResult::Mounted(liveview_id, initial_render_html))) => {
                        let _ = send_message_to_socket(
                            &mut socket,
                            liveview_id,
                            INITIAL_RENDER_TOPIC,
                            initial_render_html,
                        )
                        .await;
                    },
                    Ok(Some(HandledMessagedResult::HeartbeatResponse)) => {
                        tracing::trace!(
                            elapsed = ?heartbeat_sent_at.elapsed(),
                            "heartbeat came back",
                        );
                        heartbeat_inflight = false;
                        heartbeat_bounce.as_mut().reset(Instant::now() + A_VERY_LONG_TIME);
                        failed_heartbeats = 0;
                    }
                    Ok(Some(HandledMessagedResult::InitialRenderError(liveview_id))) => {
                        tracing::warn!("no response from `initial-render` message sent to liveview");
                        let _ = send_message_to_socket(
                            &mut socket,
                            liveview_id,
                            LIVEVIEW_GONE_TOPIC,
                            json!(null),
                        )
                        .await;
                        state.diff_streams.remove(&liveview_id);
                    }
                    Ok(None) => {},
                    Err(err) => {
                        tracing::error!(%err, "error handling message from socket");
                    },
                }
            }

            Some((liveview_id, (diff, js_commands))) = state.diff_streams.next() => {
                // TODO(david): extract to function
                if let Some(diff) = diff {
                    let _ = send_message_to_socket(
                        &mut socket,
                        liveview_id,
                        RENDERED_TOPIC,
                        diff,
                    ).await;
                }

                if !js_commands.is_empty() {
                    let _ = send_message_to_socket(
                        &mut socket,
                        liveview_id,
                        JS_COMMANDS_TOPIC,
                        js_commands,
                    ).await;
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

    tracing::trace!("WebSocket task ending");
}

async fn send_heartbeat(socket: &mut WebSocket) -> Result<(), axum::Error> {
    let msg = json!(["h"]);
    let msg = serde_json::to_string(&msg).unwrap();
    tracing::trace!("sending heartbeat");
    socket.send(ws::Message::Text(msg)).await?;
    Ok(())
}

const INITIAL_RENDER_TOPIC: &str = "i";
const LIVEVIEW_GONE_TOPIC: &str = "liveview-gone";
const RENDERED_TOPIC: &str = "r";
const JS_COMMANDS_TOPIC: &str = "j";

async fn send_message_to_socket<T>(
    socket: &mut WebSocket,
    liveview_id: LiveViewId,
    topic: &'static str,
    msg: T,
) -> Result<(), axum::Error>
where
    T: serde::Serialize,
{
    let msg = json!([liveview_id, topic, msg]);
    let msg = serde_json::to_string(&msg).unwrap();
    tracing::trace!(%msg, "sending message to websocket");

    socket.send(ws::Message::Text(msg)).await
}

enum HandledMessagedResult {
    Mounted(LiveViewId, html::Serialized),
    HeartbeatResponse,
    InitialRenderError(LiveViewId),
}

async fn handle_message_from_socket<P>(
    msg: ws::Message,
    pubsub: &P,
    state: &mut SocketState,
) -> anyhow::Result<Option<HandledMessagedResult>>
where
    P: PubSub,
{
    let text = if let ws::Message::Text(text) = msg {
        text
    } else {
        return Ok(None);
    };

    let msg = serde_json::from_str::<RawMessageOrHeartbeat>(&text)
        .with_context(|| format!("parsing into `RawMessageOrHeartbeat`. text = {:?}", text))?;

    let msg = match msg {
        RawMessageOrHeartbeat::HeartbeatResponse(heartbeat_response) => {
            if heartbeat_response.h != "ok" {
                tracing::debug!(?heartbeat_response, "invalid status in heartbeat response");
            }
            return Ok(Some(HandledMessagedResult::HeartbeatResponse));
        }
        RawMessageOrHeartbeat::RawMessage(msg) => msg,
    };

    let liveview_id = msg.liveview_id;
    let msg = EventFromBrowser::try_from(msg).context("Parsing into `EventFromBrowser`")?;

    tracing::trace!(?msg, "received message from websocket");

    match msg {
        EventFromBrowser::Mount => {
            let mut initial_render_stream = pubsub
                .subscribe(&topics::initial_render(liveview_id))
                .await
                .context("creating initial-render stream")?;

            pubsub
                .broadcast(&topics::mounted(liveview_id), ())
                .await
                .context("broadcasting mounted")?;

            let Json(initial_render_html) =
                match timeout(Duration::from_secs(5), initial_render_stream.next()).await {
                    Ok(Some(initial_render_html)) => initial_render_html,
                    Ok(None) => {
                        return Ok(Some(HandledMessagedResult::InitialRenderError(liveview_id)));
                    }
                    Err(err) => {
                        tracing::warn!(?err, "error from initial render stream");
                        return Ok(Some(HandledMessagedResult::InitialRenderError(liveview_id)));
                    }
                };

            let diff_stream = pubsub
                .subscribe(&topics::rendered(liveview_id))
                .await
                .context("creating rendered stream")?
                .map(|Json(diff)| diff);
            state
                .diff_streams
                .insert(liveview_id, Box::pin(diff_stream));

            return Ok(Some(HandledMessagedResult::Mounted(
                liveview_id,
                initial_render_html,
            )));
        }

        EventFromBrowser::Click(ClickEvent { msg }) => {
            let topic = topics::update(liveview_id);
            let msg = WithAssociatedData {
                msg,
                data: AssociatedDataKind::Click,
            };
            pubsub
                .broadcast(&topic, Json(msg))
                .await
                .context("broadcasting click event")?;
        }

        EventFromBrowser::FormEvent(FormEvent { msg, value }) => {
            let topic = topics::update(liveview_id);
            let msg = WithAssociatedData {
                msg,
                data: AssociatedDataKind::Form(value),
            };
            pubsub
                .broadcast(&topic, Json(msg))
                .await
                .context("broadcasting form event")?;
        }

        EventFromBrowser::KeyEvent(KeyEvent {
            msg,
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
        }) => {
            let topic = topics::update(liveview_id);
            let msg = WithAssociatedData {
                msg,
                data: AssociatedDataKind::Key(KeyEventValue {
                    key,
                    code,
                    alt,
                    ctrl,
                    shift,
                    meta,
                }),
            };
            pubsub
                .broadcast(&topic, Json(msg))
                .await
                .context("broadcasting key event")?;
        }
    }

    Ok(None)
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
    liveview_id: LiveViewId,
    topic: String,
    data: Value,
}

impl TryFrom<RawMessage> for EventFromBrowser {
    type Error = anyhow::Error;

    fn try_from(raw_message: RawMessage) -> Result<Self, Self::Error> {
        let RawMessage {
            topic,
            data,
            liveview_id: _,
        } = raw_message;

        let topic = topic
            .strip_prefix("axum/")
            .with_context(|| format!("unknown message topic: {:?}", topic))?;

        match &*topic {
            "mount-liveview" => Ok(EventFromBrowser::Mount),

            other => match Axm::from_str(other)? {
                Axm::Click => Ok(EventFromBrowser::Click(from_value(data)?)),

                Axm::Input | Axm::Change | Axm::Focus | Axm::Blur | Axm::Submit => {
                    Ok(EventFromBrowser::FormEvent(from_value(data)?))
                }

                Axm::Keydown | Axm::Keyup | Axm::Key | Axm::WindowKeydown | Axm::WindowKeyup => {
                    Ok(EventFromBrowser::KeyEvent(from_value(data)?))
                }

                Axm::Throttle | Axm::Debounce => {
                    anyhow::bail!("{:?} is not a valid event type", topic)
                }
            },
        }
    }
}

#[derive(Debug)]
enum EventFromBrowser {
    Mount,
    Click(ClickEvent),
    FormEvent(FormEvent),
    KeyEvent(KeyEvent),
}

#[derive(Debug, Deserialize)]
struct ClickEvent {
    #[serde(rename = "m")]
    msg: Value,
}

#[derive(Debug, Deserialize)]
struct FormEvent {
    #[serde(rename = "m")]
    msg: Value,
    #[serde(rename = "v")]
    value: FormEventValue,
}

#[derive(Debug, Deserialize, Serialize)]
struct KeyEvent {
    #[serde(rename = "m")]
    msg: Value,
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
    #[serde(rename = "me")]
    meta: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct WithAssociatedData<T> {
    pub(crate) msg: T,
    pub(crate) data: AssociatedDataKind,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum AssociatedDataKind {
    Click,
    Form(FormEventValue),
    Key(KeyEventValue),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum FormEventValue {
    String(String),
    Strings(Vec<String>),
    Bool(bool),
    Map(HashMap<String, FormEventValue>),
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct KeyEventValue {
    #[serde(rename = "k")]
    pub(crate) key: String,
    #[serde(rename = "kc")]
    pub(crate) code: String,
    #[serde(rename = "a")]
    pub(crate) alt: bool,
    #[serde(rename = "c")]
    pub(crate) ctrl: bool,
    #[serde(rename = "s")]
    pub(crate) shift: bool,
    #[serde(rename = "m")]
    pub(crate) meta: bool,
}

axm! {
    #[derive(Debug)]
    pub(crate) enum Axm {
        #[attr = "blur"]
        Blur,
        #[attr = "change"]
        Change,
        #[attr = "click"]
        Click,
        #[attr = "focus"]
        Focus,
        #[attr = "input"]
        Input,
        #[attr = "keydown"]
        Keydown,
        #[attr = "keyup"]
        Keyup,
        #[attr = "submit"]
        Submit,
        #[attr = "throttle"]
        Throttle,
        #[attr = "debounce"]
        Debounce,
        #[attr = "key"]
        Key,
        #[attr = "window-keydown"]
        WindowKeydown,
        #[attr = "window-keyup"]
        WindowKeyup,
    }
}
