use crate::{
    event_data::EventData,
    html::{self, Html},
    js_command::JsCommand,
    LiveView, Updated,
};
use async_trait::async_trait;
use axum::{
    extract::{
        ws::{self, WebSocketUpgrade},
        FromRequest, RequestParts,
    },
    response::{IntoResponse, Response},
};
use futures_util::{
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt, TryStreamExt},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    convert::Infallible,
    fmt::{self, Debug},
};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;

fn spawn_view<L>(mut view: L) -> ViewTaskHandle<L::Message>
where
    L: LiveView,
{
    let (tx, mut rx) = mpsc::channel(1024);

    crate::util::spawn_unit(async move {
        let mut markup = wrap_in_live_view_container(view.render());

        while let Some(request) = rx.recv().await {
            match request {
                ViewRequest::Render { reply_tx } => {
                    let _ = reply_tx.send(markup.serialize());
                }
                ViewRequest::Update {
                    msg,
                    reply_tx,
                    event_data,
                } => {
                    let Updated {
                        live_view: new_view,
                        js_commands,
                    } = view.update(msg, event_data).await;

                    view = new_view;

                    let new_markup = wrap_in_live_view_container(view.render());
                    let diff = markup.diff(&new_markup);
                    markup = new_markup;

                    let response = match diff {
                        Some(diff) if js_commands.is_empty() => UpdateResponse::Diff(diff),
                        Some(diff) => UpdateResponse::DiffAndJsCommands(diff, js_commands),
                        None if js_commands.is_empty() => UpdateResponse::JsCommands(js_commands),
                        None => UpdateResponse::Empty,
                    };

                    let _ = reply_tx.send(response);
                }
            }
        }
    });

    ViewTaskHandle { tx }
}

struct ViewTaskHandle<M> {
    tx: mpsc::Sender<ViewRequest<M>>,
}

impl<M> Clone for ViewTaskHandle<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M> ViewTaskHandle<M> {
    async fn render(&self) -> anyhow::Result<html::Serialized> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::Render { reply_tx };

        self.tx
            .send(request)
            .await
            .map_err(|_| anyhow::anyhow!("live view task ended unexpectedly"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("live view didn't response to render request"))
    }

    async fn update(&self, msg: M, data: EventData) -> anyhow::Result<UpdateResponse> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::Update {
            msg,
            event_data: data,
            reply_tx,
        };

        self.tx
            .send(request)
            .await
            .map_err(|_| anyhow::anyhow!("live view task ended unexpectedly"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("live view didn't response to update request"))
    }
}

enum ViewRequest<M> {
    Render {
        reply_tx: oneshot::Sender<html::Serialized>,
    },
    Update {
        msg: M,
        event_data: EventData,
        reply_tx: oneshot::Sender<UpdateResponse>,
    },
}

enum UpdateResponse {
    Diff(html::Diff),
    JsCommands(Vec<JsCommand>),
    DiffAndJsCommands(html::Diff, Vec<JsCommand>),
    Empty,
}

#[derive(Debug)]
pub struct LiveViewUpgrade {
    inner: LiveViewUpgradeInner,
}

#[derive(Debug)]
enum LiveViewUpgradeInner {
    Http,
    Ws(WebSocketUpgrade),
}

#[async_trait]
impl<B> FromRequest<B> for LiveViewUpgrade
where
    B: Send,
{
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let ws = WebSocketUpgrade::from_request(req).await.ok();

        if let Some(ws) = ws {
            Ok(Self {
                inner: LiveViewUpgradeInner::Ws(ws),
            })
        } else {
            Ok(Self {
                inner: LiveViewUpgradeInner::Http,
            })
        }
    }
}

impl LiveViewUpgrade {
    pub fn response<F, M>(self, render: F) -> Response
    where
        F: FnOnce(EmbedLiveView<'_, M>) -> Html<M>,
        M: Serialize + DeserializeOwned + Send + 'static,
    {
        let ws = match self.inner {
            LiveViewUpgradeInner::Http => {
                let embed = EmbedLiveView { handle: None };
                return render(embed).into_response();
            }
            LiveViewUpgradeInner::Ws(ws) => ws,
        };

        let mut handle = None;
        let (tx, rx) = mpsc::channel(1024);

        let embed = EmbedLiveView {
            handle: Some((&mut handle, tx)),
        };

        render(embed);

        let handle = if let Some(handle) = handle {
            handle
        } else {
            return ws.on_upgrade(|_| async {}).into_response();
        };

        ws.on_upgrade(move |socket| async move {
            let (write, read) = socket.split();

            let write = write.with(|msg| async move {
                let encoded_msg = ws::Message::Text(serde_json::to_string(&msg)?);
                Ok::<_, anyhow::Error>(encoded_msg)
            });
            futures_util::pin_mut!(write);

            let read = read
                .map_err(anyhow::Error::from)
                .and_then(|msg| async move {
                    if let ws::Message::Text(text) = msg {
                        serde_json::from_str(&text).map_err(Into::into)
                    } else {
                        anyhow::bail!("received message from socket that wasn't text")
                    }
                });
            futures_util::pin_mut!(read);

            if let Err(err) = process_view_messages(write, read, handle, rx).await {
                tracing::error!(%err, "encountered while processing socket");
            }
        })
        .into_response()
    }
}

pub struct EmbedLiveView<'a, M> {
    handle: Option<(&'a mut Option<ViewTaskHandle<M>>, mpsc::Sender<M>)>,
}

impl<'a, M> EmbedLiveView<'a, M> {
    pub fn embed<L>(self, view: L) -> Html<M>
    where
        L: LiveView<Message = M>,
    {
        self.embed_handle(view).0
    }

    pub fn embed_handle<L>(self, view: L) -> (Html<M>, Option<ViewHandle<L::Message>>)
    where
        L: LiveView<Message = M>,
    {
        let html = wrap_in_live_view_container(view.render());

        if let Some((handle, tx)) = self.handle {
            *handle = Some(spawn_view(view));
            (html, Some(ViewHandle { tx }))
        } else {
            (html, None)
        }
    }

    pub fn connected(&self) -> bool {
        self.handle.is_some()
    }
}

impl<'a, M> fmt::Debug for EmbedLiveView<'a, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedLiveView").finish()
    }
}

pub struct ViewHandle<M> {
    tx: mpsc::Sender<M>,
}

impl<M> ViewHandle<M> {
    pub async fn send(&self, msg: M) -> Result<(), ViewHandleSendError> {
        self.tx.send(msg).await.map_err(|_| ViewHandleSendError)?;
        Ok(())
    }
}

impl<M> Clone for ViewHandle<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M> fmt::Debug for ViewHandle<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewHandle").finish()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ViewHandleSendError;

impl fmt::Display for ViewHandleSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to send message to view")
    }
}

impl std::error::Error for ViewHandleSendError {}

#[allow(
    unused_variables,
    clippy::diverging_sub_expression,
    unreachable_code,
    clippy::todo
)]
async fn process_view_messages<W, R, RE, M>(
    mut write: W,
    read: R,
    view: ViewTaskHandle<M>,
    rx: mpsc::Receiver<M>,
) -> anyhow::Result<()>
where
    W: Sink<MessageToSocket> + Unpin,
    W::Error: Into<anyhow::Error>,
    R: Stream<Item = Result<MessageFromSocket<M>, RE>> + Unpin,
    RE: Into<anyhow::Error>,
    M: DeserializeOwned,
{
    let markup = view.render().await?;
    write_message(&mut write, MessageToSocketData::InitialRender(markup)).await?;

    let rx_stream = ReceiverStream::new(rx).map(|msg| {
        Ok(MessageFromSocket {
            msg,
            data: MessageFromSocketData::None,
        })
    });

    let mut stream = tokio_stream::StreamExt::merge(read, rx_stream);

    loop {
        let MessageFromSocket {
            msg: msg_for_view,
            data,
        } = match stream.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(err)) => {
                let err = err.into();
                tracing::trace!(%err, "error from socket");
                break;
            }
            None => {
                tracing::trace!("no more messages on socket");
                break;
            }
        };

        let data = EventData::from(data);

        match view.update(msg_for_view, data).await? {
            UpdateResponse::Diff(diff) => {
                write_message(&mut write, MessageToSocketData::Render(diff)).await?;
            }
            UpdateResponse::JsCommands(commands) => {
                write_message(&mut write, MessageToSocketData::JsCommands(commands)).await?;
            }
            UpdateResponse::DiffAndJsCommands(diff, commands) => {
                write_message(&mut write, MessageToSocketData::Render(diff)).await?;
                write_message(&mut write, MessageToSocketData::JsCommands(commands)).await?;
            }
            UpdateResponse::Empty => {}
        }
    }

    Ok(())
}

#[derive(Serialize, Debug)]
struct MessageToSocket {
    #[serde(flatten)]
    data: MessageToSocketData,
}

#[derive(Serialize, Debug)]
#[serde(tag = "t", content = "d")]
enum MessageToSocketData {
    #[serde(rename = "i")]
    InitialRender(html::Serialized),
    #[serde(rename = "r")]
    Render(html::Diff),
    #[serde(rename = "j")]
    JsCommands(Vec<JsCommand>),
}

async fn write_message<W>(write: &mut W, data: MessageToSocketData) -> anyhow::Result<()>
where
    W: Sink<MessageToSocket> + Unpin,
    W::Error: Into<anyhow::Error>,
{
    let msg = MessageToSocket { data };
    tracing::trace!(?msg, "sending message to socket");
    Ok(write.send(msg).await.map_err(Into::into)?)
}

#[derive(Debug, Deserialize, PartialEq)]
struct MessageFromSocket<M> {
    #[serde(rename = "m")]
    msg: M,
    #[serde(flatten)]
    data: MessageFromSocketData,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "snake_case")]
pub(crate) enum MessageFromSocketData {
    None,
    Click,
    WindowFocus,
    WindowBlur,
    FormSubmit {
        #[serde(rename = "q")]
        query: String,
    },
    FormChange {
        #[serde(rename = "q")]
        query: String,
    },
    InputChange {
        #[serde(rename = "v")]
        value: String,
    },
    Key {
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
    },
    Mouse {
        #[serde(rename = "cx")]
        client_x: f64,
        #[serde(rename = "cy")]
        client_y: f64,
        #[serde(rename = "px")]
        page_x: f64,
        #[serde(rename = "py")]
        page_y: f64,
        #[serde(rename = "ox")]
        offset_x: f64,
        #[serde(rename = "oy")]
        offset_y: f64,
        #[serde(rename = "mx")]
        movement_x: f64,
        #[serde(rename = "my")]
        movement_y: f64,
        #[serde(rename = "sx")]
        screen_x: f64,
        #[serde(rename = "sy")]
        screen_y: f64,
    },
}

fn wrap_in_live_view_container<T>(markup: Html<T>) -> Html<T> {
    use crate as axum_live_view;
    axum_live_view_macros::html! {
        <div id="live-view-container">{ markup }</div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as axum_live_view;
    use crate::Html;
    use axum_live_view_macros::html;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn serialize_message_to_socket() {
        fn make_html(value: &'static str) -> Html<()> {
            html! { <div>{ value }</div> }
        }

        let html = make_html("foo");
        let msg = json!(MessageToSocketData::InitialRender(html.serialize()));

        assert_eq!(
            msg,
            json!({
                "t": "i",
                "d": {
                    "0": "foo",
                    "f": ["<div>", "</div>"]
                }
            })
        );

        let new_html = make_html("bar");
        let diff = html.diff(&new_html).unwrap();
        let msg = json!(MessageToSocketData::Render(diff));

        assert_eq!(
            msg,
            json!({
                "t": "r",
                "d": {
                    "0": "bar",
                }
            })
        );
    }

    #[test]
    fn serialize_js_commands() {
        let cmd = crate::js_command::set_title("foo").delay(Duration::from_millis(500));
        let msg = json!(MessageToSocketData::JsCommands(Vec::from([cmd])));

        assert_eq!(
            msg,
            json!({
                "t": "j",
                "d": [
                    {
                        "delay_ms": 500,
                        "kind": {
                            "t": "set_title",
                            "title": "foo"
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn deserialize_message_from_socket_mount() {
        let msg =
            serde_json::from_value::<MessageFromSocket<Msg>>(json!({ "m": "Incr", "t": "click" }))
                .unwrap();
        assert_eq!(
            msg,
            MessageFromSocket {
                msg: Msg::Incr,
                data: MessageFromSocketData::Click
            }
        );

        let msg = serde_json::from_value::<MessageFromSocket<Msg>>(
            json!({ "m": "Incr", "t": "submit", "d": { "q": "name=bob&age=20" } }),
        )
        .unwrap();
        assert_eq!(
            msg,
            MessageFromSocket {
                msg: Msg::Incr,
                data: MessageFromSocketData::FormSubmit {
                    query: "name=bob&age=20".to_owned()
                }
            }
        );
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    enum Msg {
        Incr,
        Decr,
    }
}
