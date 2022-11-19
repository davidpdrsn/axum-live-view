use crate::{
    event_data::EventData,
    html::Html,
    js_command::JsCommand,
    live_view::{Updated, ViewHandle},
    util::ReceiverStream,
    LiveView,
};
use futures_util::{
    sink::{Sink, SinkExt},
    stream::StreamExt,
    TryStream, TryStreamExt,
};
use http::{HeaderMap, Uri};
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Serialize,
};
use serde_json::Value;
use std::{fmt, marker::PhantomData};
use tokio::sync::{mpsc, oneshot};

/// Type used to embed live views in HTML templates.
pub struct EmbedLiveView<'a, L> {
    view: Option<&'a mut Option<L>>,
}

impl<'a, L> EmbedLiveView<'a, L> {
    pub(crate) fn noop() -> Self {
        Self { view: None }
    }

    pub(crate) fn new(view: &'a mut Option<L>) -> Self {
        Self { view: Some(view) }
    }

    /// Embed a live view in a HTML template.
    pub fn embed(self, view: L) -> Html<L::Message>
    where
        L: LiveView,
    {
        let html = wrap_in_live_view_container(view.render());

        if let Some(view_handle) = self.view {
            *view_handle = Some(view);
        }

        html
    }

    /// Check whether the request was a WebSocket upgrade request.
    ///
    /// This can be used to initialize the view differently depending on which part of the live
    /// view life cycle we're in. See the [root module docs](crate) for more details on the life
    /// cycle.
    pub fn connected(&self) -> bool {
        self.view.is_some()
    }
}

impl<'a, M> fmt::Debug for EmbedLiveView<'a, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedLiveView").finish()
    }
}

pub(crate) async fn run_view<W, R, L>(
    mut write: W,
    read: R,
    view: L,
    uri: Uri,
    headers: HeaderMap,
) -> Result<(), String>
where
    L: LiveView,
    W: Sink<MessageToSocket> + Unpin,
    W::Error: fmt::Display + Send + Sync + 'static,
    R: TryStream<Ok = MessageFromSocket<L::Message>> + Unpin,
    R::Error: fmt::Display + Send + Sync + 'static,
{
    let (handle, rx) = ViewHandle::new();

    let view = spawn_view(view, Some(handle.clone()));

    view.mount(uri, headers, handle)
        .await
        .map_err(|err| err.to_string())?;

    let markup = view.render().await.map_err(|err| err.to_string())?;

    write_message(&mut write, MessageToSocketData::InitialRender(markup))
        .await
        .map_err(|err| err.to_string())?;

    let rx_stream = ReceiverStream::new(rx).map(|msg| {
        Ok(MessageFromSocket::Event {
            msg,
            data: EventMessageFromSocketData::None,
        })
    });
    let mut stream = crate::util::StreamExt::merge(read.into_stream(), rx_stream);

    loop {
        let msg = match stream.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(err)) => {
                let err = err.to_string();
                tracing::trace!(%err, "error from socket");
                break;
            }
            None => {
                tracing::trace!("no more messages on socket");
                break;
            }
        };

        match msg {
            MessageFromSocket::Event {
                msg: msg_for_view,
                data,
            } => {
                let data = Option::<EventData>::from(data);

                match view
                    .update(msg_for_view, data)
                    .await
                    .map_err(|err| err.to_string())?
                {
                    UpdateResponse::Diff(diff) => {
                        write_message(&mut write, MessageToSocketData::Render(diff))
                            .await
                            .map_err(|err| err.to_string())?;
                    }
                    UpdateResponse::JsCommands(commands) => {
                        write_message(&mut write, MessageToSocketData::JsCommands(commands))
                            .await
                            .map_err(|err| err.to_string())?;
                    }
                    UpdateResponse::DiffAndJsCommands(diff, commands) => {
                        write_message(&mut write, MessageToSocketData::Render(diff))
                            .await
                            .map_err(|err| err.to_string())?;
                        write_message(&mut write, MessageToSocketData::JsCommands(commands))
                            .await
                            .map_err(|err| err.to_string())?;
                    }
                    UpdateResponse::Empty => {}
                }
            }
            MessageFromSocket::Internal {
                data: InternalMessageFromSocketData::Health,
            } => {
                write_message(&mut write, MessageToSocketData::Health)
                    .await
                    .map_err(|err| err.to_string())?;
            }
        }
    }

    Ok(())
}

pub(crate) fn spawn_view<L>(
    mut view: L,
    view_handle: Option<ViewHandle<L::Message>>,
) -> ViewTaskHandle<L::Message>
where
    L: LiveView,
{
    let (tx, mut rx) = mpsc::channel(1024);

    crate::util::spawn_unit(async move {
        let mut markup = wrap_in_live_view_container(view.render());

        while let Some(request) = rx.recv().await {
            match request {
                ViewRequest::Mount {
                    uri,
                    headers,
                    handle,
                    reply_tx,
                } => {
                    let _ = reply_tx.send(view.mount(uri, &headers, handle));
                }
                ViewRequest::Render { reply_tx } => {
                    let _ = reply_tx
                        .send(serde_json::to_value(&markup).expect("failed to serialize HTML"));
                }
                ViewRequest::RenderToString { reply_tx } => {
                    let _ = reply_tx.send(markup.render());
                }
                ViewRequest::Update {
                    msg,
                    reply_tx,
                    event_data,
                } => {
                    let Updated {
                        live_view: new_view,
                        js_commands,
                        spawns,
                    } = view.update(msg, event_data);

                    if let Some(view_handle) = &view_handle {
                        for future in spawns {
                            let view_handle = view_handle.clone();
                            crate::util::spawn_unit(async move {
                                let msg = future.await;
                                let _ = view_handle.send(msg).await;
                            });
                        }
                    }

                    view = new_view;

                    let new_markup = wrap_in_live_view_container(view.render());
                    let diff = markup.diff(&new_markup).map(|diff| {
                        serde_json::to_value(&diff).expect("failed to serialize HTML diff")
                    });
                    markup = new_markup;

                    let response = match (diff, js_commands.is_empty()) {
                        (None, true) => UpdateResponse::Empty,
                        (None, false) => UpdateResponse::JsCommands(js_commands),
                        (Some(diff), true) => UpdateResponse::Diff(diff),
                        (Some(diff), false) => UpdateResponse::DiffAndJsCommands(diff, js_commands),
                    };

                    let _ = reply_tx.send(response);
                }
            }
        }
    });

    ViewTaskHandle { tx }
}

pub(crate) struct ViewTaskHandle<M> {
    tx: mpsc::Sender<ViewRequest<M>>,
}

impl<M> Clone for ViewTaskHandle<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M> fmt::Debug for ViewTaskHandle<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewTaskHandle")
            .field("tx", &self.tx)
            .finish()
    }
}

impl<M> ViewTaskHandle<M> {
    pub(crate) async fn mount(
        &self,
        uri: Uri,
        headers: HeaderMap,
        handle: ViewHandle<M>,
    ) -> Result<(), ViewRequestError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::Mount {
            reply_tx,
            uri,
            headers,
            handle,
        };

        self.tx
            .send(request)
            .await
            .map_err(|_| ViewRequestError::ChannelClosed(ChannelClosed))?;

        match reply_rx.await {
            Ok(()) => Ok(()),
            Err(_) => Err(ViewRequestError::ChannelClosed(ChannelClosed)),
        }
    }

    pub(crate) async fn render(&self) -> Result<Value, ChannelClosed> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::Render { reply_tx };

        self.tx.send(request).await.map_err(|_| ChannelClosed)?;

        reply_rx.await.map_err(|_| ChannelClosed)
    }

    pub(crate) async fn render_to_string(&self) -> Result<String, ChannelClosed> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::RenderToString { reply_tx };

        self.tx.send(request).await.map_err(|_| ChannelClosed)?;

        reply_rx.await.map_err(|_| ChannelClosed)
    }

    pub(crate) async fn update(
        &self,
        msg: M,
        event_data: Option<EventData>,
    ) -> Result<UpdateResponse, ViewRequestError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::Update {
            msg,
            event_data,
            reply_tx,
        };

        self.tx
            .send(request)
            .await
            .map_err(|_| ViewRequestError::ChannelClosed(ChannelClosed))?;

        match reply_rx.await {
            Ok(updated) => Ok(updated),
            Err(_) => Err(ViewRequestError::ChannelClosed(ChannelClosed)),
        }
    }
}

enum ViewRequest<M> {
    Mount {
        uri: Uri,
        headers: HeaderMap,
        handle: ViewHandle<M>,
        reply_tx: oneshot::Sender<()>,
    },
    Render {
        reply_tx: oneshot::Sender<Value>,
    },
    RenderToString {
        reply_tx: oneshot::Sender<String>,
    },
    Update {
        msg: M,
        event_data: Option<EventData>,
        reply_tx: oneshot::Sender<UpdateResponse>,
    },
}

#[derive(Debug)]
pub(crate) enum ViewRequestError {
    ChannelClosed(ChannelClosed),
}

impl fmt::Display for ViewRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChannelClosed(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ViewRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ChannelClosed(_) => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChannelClosed;

impl fmt::Display for ChannelClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel sender or received closed unexpectedly")
    }
}

impl std::error::Error for ChannelClosed {}

pub(crate) enum UpdateResponse {
    Diff(Value),
    JsCommands(Vec<JsCommand>),
    DiffAndJsCommands(Value, Vec<JsCommand>),
    Empty,
}

#[derive(Serialize)]
pub(crate) struct MessageToSocket {
    #[serde(flatten)]
    data: MessageToSocketData,
}

#[derive(Serialize)]
#[serde(tag = "t", content = "d")]
enum MessageToSocketData {
    #[serde(rename = "i")]
    InitialRender(Value),
    #[serde(rename = "r")]
    Render(Value),
    #[serde(rename = "j")]
    JsCommands(Vec<JsCommand>),
    #[serde(rename = "h")]
    Health,
}

async fn write_message<W>(write: &mut W, data: MessageToSocketData) -> Result<(), W::Error>
where
    W: Sink<MessageToSocket> + Unpin,
{
    let msg = MessageToSocket { data };
    write.send(msg).await
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum MessageFromSocket<M>
where
    M: DeserializeOwned,
{
    Event {
        #[serde(rename = "m", deserialize_with = "deserialize_msg")]
        msg: M,
        #[serde(flatten)]
        data: EventMessageFromSocketData,
    },
    Internal {
        #[serde(flatten)]
        data: InternalMessageFromSocketData,
    },
}

fn deserialize_msg<'de, D, M>(d: D) -> Result<M, D::Error>
where
    D: de::Deserializer<'de>,
    M: DeserializeOwned,
{
    struct MsgVisitor<M>(PhantomData<M>);

    impl<'de, M> de::Visitor<'de> for MsgVisitor<M>
    where
        M: DeserializeOwned,
    {
        type Value = M;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a percent encoded JSON string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let s = percent_encoding::percent_decode_str(v)
                .decode_utf8()
                .map_err(E::custom)?;
            let msg = serde_json::from_str::<M>(&s).map_err(E::custom)?;
            Ok(msg)
        }
    }

    d.deserialize_str(MsgVisitor(PhantomData))
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventMessageFromSocketData {
    None,
    Click,
    WindowFocus,
    WindowBlur,
    Form {
        #[serde(rename = "q")]
        query: String,
    },
    Input {
        #[serde(rename = "v")]
        value: InputValue,
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
    Scroll {
        #[serde(rename = "sx")]
        scroll_x: f64,
        #[serde(rename = "sy")]
        scroll_y: f64,
    },
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub(crate) enum InputValue {
    Bool(bool),
    String(String),
    Strings(Vec<String>),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "t")]
pub(crate) enum InternalMessageFromSocketData {
    #[serde(rename = "h")]
    Health,
}

fn wrap_in_live_view_container<T>(markup: Html<T>) -> Html<T> {
    crate::html::private::HtmlBuilder {
        dynamic: Vec::from([crate::html::DynamicFragment::Html(markup)]),
        fixed: &["<div id=\"live-view-container\">", "</div>"],
    }
    .into_html()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::time::Duration;

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
        let msg = serde_json::from_value::<MessageFromSocket<Msg>>(
            json!({ "m": "%22Incr%22", "t": "click" }),
        )
        .unwrap();
        assert_eq!(
            msg,
            MessageFromSocket::Event {
                msg: Msg::Incr,
                data: EventMessageFromSocketData::Click
            }
        );

        let msg = serde_json::from_value::<MessageFromSocket<Msg>>(
            json!({ "m": "%22Incr%22", "t": "form", "d": { "q": "name=bob&age=20" } }),
        )
        .unwrap();
        assert_eq!(
            msg,
            MessageFromSocket::Event {
                msg: Msg::Incr,
                data: EventMessageFromSocketData::Form {
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
