use crate::{
    live_view::{EmbedLiveView, LiveViewId},
    pubsub::{PubSub, PubSubError},
    topics::{self, RenderedMessage},
};
use anyhow::Context;
use axum::{
    extract::ws::{self, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures_util::{sink::SinkExt, stream::BoxStream, Sink, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use std::{collections::HashMap, convert::TryFrom};
use tokio_stream::StreamMap;

pub(crate) fn routes<P, B>() -> Router<B>
where
    P: PubSub + Clone,
    B: Send + 'static,
{
    Router::new().route("/live", get(ws::<P>))
}

async fn ws<P>(upgrade: WebSocketUpgrade, embed_live_view: EmbedLiveView<P>) -> impl IntoResponse
where
    P: PubSub + Clone,
{
    upgrade.on_upgrade(move |socket| {
        let (write, read) = socket.split();
        handle_socket(read, write, embed_live_view.pubsub)
    })
}

#[derive(Default)]
struct SocketState {
    diff_streams: StreamMap<LiveViewId, BoxStream<'static, RenderedMessage>>,
}

async fn handle_socket<R, W, P>(mut read: R, mut write: W, pubsub: P)
where
    R: Stream<Item = Result<ws::Message, axum::Error>> + Unpin,
    W: Sink<ws::Message> + Unpin,
    W::Error: std::error::Error + Send + Sync + 'static,
    P: PubSub,
{
    let mut state = SocketState::default();

    loop {
        tokio::select! {
            Some(msg) = read.next() => {
                let result = handle_message_from_socket(msg, &pubsub, &mut state, &mut write).await;
                match result {
                    Ok(()) => {},
                    Err(HowBadIsTheErrorReally::ItsFine(err)) => {
                        tracing::trace!(%err, "error handling message from socket");
                        break;
                    }
                    Err(HowBadIsTheErrorReally::ReallyBad(err)) => {
                        tracing::error!(%err, "error handling message from socket");
                        break;
                    }
                }
            }

            Some((live_view_id, msg)) = state.diff_streams.next() => {
                match msg {
                    RenderedMessage::Diff(diff) => {
                        let _ = send_message_to_socket(
                            &mut write,
                            Some(live_view_id),
                            RENDERED_TOPIC,
                            Some(diff),
                        ).await;
                    }
                    RenderedMessage::DiffWithCommands(diff, js_commands) => {
                        let _ = send_message_to_socket(
                            &mut write,
                            Some(live_view_id),
                            RENDERED_TOPIC,
                            Some(diff),
                        ).await;

                        let _ = send_message_to_socket(
                            &mut write,
                            Some(live_view_id),
                            JS_COMMANDS_TOPIC,
                            Some(js_commands),
                        ).await;
                    }
                    RenderedMessage::Commands(js_commands) => {
                        let _ = send_message_to_socket(
                            &mut write,
                            Some(live_view_id),
                            JS_COMMANDS_TOPIC,
                            Some(js_commands),
                        ).await;
                    }
                }
            }
        }
    }

    let live_view_ids = state
        .diff_streams
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();

    for live_view_id in live_view_ids {
        let _ = pubsub
            .broadcast(&topics::socket_disconnected(live_view_id), ())
            .await;
    }

    tracing::trace!("WebSocket task ending");
}

enum HowBadIsTheErrorReally {
    ItsFine(anyhow::Error),
    ReallyBad(anyhow::Error),
}

async fn handle_message_from_socket<P, W>(
    msg: Result<ws::Message, axum::Error>,
    pubsub: &P,
    state: &mut SocketState,
    write: &mut W,
) -> Result<(), HowBadIsTheErrorReally>
where
    P: PubSub,
    W: Sink<ws::Message> + Unpin,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let text = match msg {
        Ok(ws::Message::Text(text)) => text,
        Ok(other) => {
            tracing::warn!(?other, "received unexpected message from socket");
            return Ok(());
        }
        Err(err) => return Err(HowBadIsTheErrorReally::ItsFine(err.into())),
    };

    let msg = serde_json::from_str::<RawMessage>(&text)
        .with_context(|| format!("parsing into `RawMessage`. text = {:?}", text))
        .map_err(HowBadIsTheErrorReally::ReallyBad)?;

    let live_view_id = msg.live_view_id;
    let msg = EventFromBrowser::try_from(msg.clone())
        .with_context(|| format!("Parsing into `EventFromBrowser`. msg={:?}", msg))
        .map_err(HowBadIsTheErrorReally::ReallyBad)?;

    tracing::trace!(?msg, "received message from websocket");

    match msg {
        EventFromBrowser::Mount => {
            let mut initial_render_stream = pubsub
                .subscribe(&topics::initial_render(live_view_id))
                .await
                .map_err(PubSubError::boxed)
                .context("creating initial-render stream")
                .map_err(HowBadIsTheErrorReally::ReallyBad)?;

            pubsub
                .broadcast(&topics::mounted(live_view_id), ())
                .await
                .map_err(PubSubError::boxed)
                .context("broadcasting mounted")
                .map_err(HowBadIsTheErrorReally::ReallyBad)?;

            let Json(initial_render_html) = if let Some(msg) = initial_render_stream.next().await {
                msg
            } else {
                return Err(HowBadIsTheErrorReally::ReallyBad(anyhow::anyhow!(
                    "initial-render never responded"
                )));
            };

            let diff_stream = pubsub
                .subscribe(&topics::rendered(live_view_id))
                .await
                .map_err(PubSubError::boxed)
                .context("creating rendered stream")
                .map_err(HowBadIsTheErrorReally::ReallyBad)?
                .map(|Json(diff)| diff);
            state
                .diff_streams
                .insert(live_view_id, Box::pin(diff_stream));

            send_message_to_socket(
                write,
                Some(live_view_id),
                INITIAL_RENDER_TOPIC,
                Some(initial_render_html),
            )
            .await
            .map_err(HowBadIsTheErrorReally::ReallyBad)?;
        }

        EventFromBrowser::Click(WithoutValue { msg })
        | EventFromBrowser::WindowFocus(WithoutValue { msg })
        | EventFromBrowser::WindowBlur(WithoutValue { msg }) => {
            send_update(live_view_id, msg, None, pubsub)
                .await
                .map_err(HowBadIsTheErrorReally::ReallyBad)?;
        }

        EventFromBrowser::MouseEvent(MouseEvent { msg, fields }) => {
            send_update(
                live_view_id,
                msg,
                Some(AssociatedDataKind::Mouse(fields)),
                pubsub,
            )
            .await
            .map_err(HowBadIsTheErrorReally::ReallyBad)?;
        }

        EventFromBrowser::FormEvent(FormEvent { msg, value }) => {
            send_update(
                live_view_id,
                msg,
                Some(AssociatedDataKind::Form(value)),
                pubsub,
            )
            .await
            .map_err(HowBadIsTheErrorReally::ReallyBad)?;
        }

        EventFromBrowser::KeyEvent(KeyEvent { msg, fields }) => {
            send_update(
                live_view_id,
                msg,
                Some(AssociatedDataKind::Key(fields)),
                pubsub,
            )
            .await
            .map_err(HowBadIsTheErrorReally::ReallyBad)?;
        }
    }

    Ok(())
}

async fn send_update<P>(
    live_view_id: LiveViewId,
    msg: Value,
    data: Option<AssociatedDataKind>,
    pubsub: &P,
) -> anyhow::Result<()>
where
    P: PubSub,
{
    let topic = topics::update(live_view_id);
    let msg = WithAssociatedData { msg, data };

    pubsub
        .broadcast(&topic, Json(msg))
        .await
        .map_err(PubSubError::boxed)
        .context("broadcasting key event")?;

    Ok(())
}

#[derive(Debug, Deserialize, Clone)]
struct RawMessage {
    live_view_id: LiveViewId,
    topic: String,
    data: Value,
}

impl TryFrom<RawMessage> for EventFromBrowser {
    type Error = anyhow::Error;

    fn try_from(raw_message: RawMessage) -> Result<Self, Self::Error> {
        let RawMessage {
            topic,
            data,
            live_view_id: _,
        } = raw_message;

        let topic = topic
            .strip_prefix("axum/")
            .with_context(|| format!("unknown message topic: {:?}", topic))?;

        match &*topic {
            "mount-live-view" => Ok(EventFromBrowser::Mount),

            other => match Axm::from_str(other)? {
                Axm::Click => Ok(EventFromBrowser::Click(from_value(data)?)),

                Axm::WindowFocus => Ok(EventFromBrowser::WindowFocus(from_value(data)?)),

                Axm::WindowBlur => Ok(EventFromBrowser::WindowBlur(from_value(data)?)),

                Axm::Input | Axm::Change | Axm::Focus | Axm::Blur | Axm::Submit => {
                    Ok(EventFromBrowser::FormEvent(from_value(data)?))
                }

                Axm::Keydown | Axm::Keyup | Axm::Key | Axm::WindowKeydown | Axm::WindowKeyup => {
                    Ok(EventFromBrowser::KeyEvent(from_value(data)?))
                }

                Axm::Mouseenter
                | Axm::Mouseover
                | Axm::Mouseleave
                | Axm::Mouseout
                | Axm::Mousemove => Ok(EventFromBrowser::MouseEvent(from_value(data)?)),

                Axm::Throttle | Axm::Debounce => {
                    anyhow::bail!(
                        "{:?} events should never be sent to the WebSocket, they're browser only",
                        topic
                    )
                }
            },
        }
    }
}

#[derive(Debug)]
enum EventFromBrowser {
    Mount,
    Click(WithoutValue),
    WindowFocus(WithoutValue),
    WindowBlur(WithoutValue),
    FormEvent(FormEvent),
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
}

#[derive(Debug, Deserialize)]
struct WithoutValue {
    #[serde(rename = "m")]
    msg: Value,
}

#[derive(Debug, Deserialize)]
struct MouseEvent {
    #[serde(rename = "m")]
    msg: Value,
    #[serde(flatten)]
    fields: MouseEventFields,
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
    #[serde(flatten)]
    fields: KeyEventFields,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct WithAssociatedData<T> {
    pub(crate) msg: T,
    pub(crate) data: Option<AssociatedDataKind>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum AssociatedDataKind {
    Form(FormEventValue),
    Key(KeyEventFields),
    Mouse(MouseEventFields),
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
pub(crate) struct KeyEventFields {
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
    #[serde(rename = "me")]
    pub(crate) meta: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MouseEventFields {
    #[serde(rename = "cx")]
    pub(crate) client_x: f64,
    #[serde(rename = "cy")]
    pub(crate) client_y: f64,
    #[serde(rename = "px")]
    pub(crate) page_x: f64,
    #[serde(rename = "py")]
    pub(crate) page_y: f64,
    #[serde(rename = "ox")]
    pub(crate) offset_x: f64,
    #[serde(rename = "oy")]
    pub(crate) offset_y: f64,
    #[serde(rename = "mx")]
    pub(crate) movement_x: f64,
    #[serde(rename = "my")]
    pub(crate) movement_y: f64,
    #[serde(rename = "sx")]
    pub(crate) screen_x: f64,
    #[serde(rename = "sy")]
    pub(crate) screen_y: f64,
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
        #[attr = "window-focus"]
        WindowFocus,
        #[attr = "window-blur"]
        WindowBlur,
        #[attr = "mouseenter"]
        Mouseenter,
        #[attr = "mouseover"]
        Mouseover,
        #[attr = "mouseleave"]
        Mouseleave,
        #[attr = "mouseout"]
        Mouseout,
        #[attr = "mousemove"]
        Mousemove,
    }
}

const INITIAL_RENDER_TOPIC: &str = "i";
const RENDERED_TOPIC: &str = "r";
const JS_COMMANDS_TOPIC: &str = "j";

async fn send_message_to_socket<W, T>(
    write: &mut W,
    live_view_id: Option<LiveViewId>,
    topic: &'static str,
    msg: Option<T>,
) -> anyhow::Result<()>
where
    W: Sink<ws::Message> + Unpin,
    W::Error: std::error::Error + Send + Sync + 'static,
    T: serde::Serialize,
{
    let msg = json!({
        "i": live_view_id,
        "t": topic,
        "d": msg,
    });
    let msg = serde_json::to_string(&msg).unwrap();
    tracing::trace!(%msg, "sending message to websocket");

    Ok(write.send(ws::Message::Text(msg)).await?)
}
