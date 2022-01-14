use crate::{
    event_data::EventData, html::Html, js_command::JsCommand, live_view::ViewHandle, LiveView,
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
use tokio_stream::wrappers::ReceiverStream;

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
    let view = spawn_view(view);

    let (handle, rx) = ViewHandle::new();

    view.mount(uri, headers, handle)
        .await
        .map_err(|err| err.to_string())?;

    let markup = view.render().await.map_err(|err| err.to_string())?;

    write_message(&mut write, MessageToSocketData::InitialRender(markup))
        .await
        .map_err(|err| err.to_string())?;

    let rx_stream = ReceiverStream::new(rx).map(|msg| {
        Ok(MessageFromSocket {
            msg,
            data: MessageFromSocketData::None,
        })
    });
    let mut stream = tokio_stream::StreamExt::merge(read.into_stream(), rx_stream);

    loop {
        let MessageFromSocket {
            msg: msg_for_view,
            data,
        } = match stream.next().await {
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

    Ok(())
}

pub(crate) fn spawn_view<L>(mut view: L) -> ViewTaskHandle<L::Message, L::Error>
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
                } => match view.mount(uri, &headers, handle).await {
                    Ok(value) => {
                        let _ = reply_tx.send(Ok(value));
                    }
                    Err(err) => {
                        let _ = reply_tx.send(Err(err));
                        break;
                    }
                },
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
                    let (new_view, js_commands) = match view.update(msg, event_data).await {
                        Ok(updated) => updated.into_parts(),
                        Err(err) => {
                            let _ = reply_tx.send(Err(err));
                            break;
                        }
                    };

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

                    let _ = reply_tx.send(Ok(response));
                }
            }
        }
    });

    ViewTaskHandle { tx }
}

pub(crate) struct ViewTaskHandle<M, E> {
    tx: mpsc::Sender<ViewRequest<M, E>>,
}

impl<M, E> Clone for ViewTaskHandle<M, E> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M, E> ViewTaskHandle<M, E> {
    pub(crate) async fn mount(
        &self,
        uri: Uri,
        headers: HeaderMap,
        handle: ViewHandle<M>,
    ) -> Result<(), ViewRequestError<E>> {
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
            Ok(Ok(val)) => Ok(val),
            Ok(Err(err)) => Err(ViewRequestError::ViewError(err)),
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
    ) -> Result<UpdateResponse, ViewRequestError<E>> {
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
            Ok(Ok(val)) => Ok(val),
            Ok(Err(err)) => Err(ViewRequestError::ViewError(err)),
            Err(_) => Err(ViewRequestError::ChannelClosed(ChannelClosed)),
        }
    }
}

enum ViewRequest<M, E> {
    Mount {
        uri: Uri,
        headers: HeaderMap,
        handle: ViewHandle<M>,
        reply_tx: oneshot::Sender<Result<(), E>>,
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
        reply_tx: oneshot::Sender<Result<UpdateResponse, E>>,
    },
}

#[derive(Debug)]
pub(crate) enum ViewRequestError<E> {
    ChannelClosed(ChannelClosed),
    ViewError(E),
}

impl<E> fmt::Display for ViewRequestError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChannelClosed(err) => err.fmt(f),
            Self::ViewError(err) => err.fmt(f),
        }
    }
}

impl<E> std::error::Error for ViewRequestError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ChannelClosed(_) => None,
            Self::ViewError(err) => Some(&*err),
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
}

async fn write_message<W>(write: &mut W, data: MessageToSocketData) -> Result<(), W::Error>
where
    W: Sink<MessageToSocket> + Unpin,
{
    let msg = MessageToSocket { data };
    // tracing::trace!(?msg, "sending message to socket");
    write.send(msg).await
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MessageFromSocket<M>
where
    M: DeserializeOwned,
{
    #[serde(rename = "m", deserialize_with = "deserialize_msg")]
    msg: M,
    #[serde(flatten)]
    data: MessageFromSocketData,
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
pub(crate) enum MessageFromSocketData {
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
            MessageFromSocket {
                msg: Msg::Incr,
                data: MessageFromSocketData::Click
            }
        );

        let msg = serde_json::from_value::<MessageFromSocket<Msg>>(
            json!({ "m": "%22Incr%22", "t": "form", "d": { "q": "name=bob&age=20" } }),
        )
        .unwrap();
        assert_eq!(
            msg,
            MessageFromSocket {
                msg: Msg::Incr,
                data: MessageFromSocketData::Form {
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
