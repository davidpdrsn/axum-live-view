use crate::{
    event_data::EventData,
    html::{self, Html},
    js_command::JsCommand,
    live_view::ViewHandle,
    LiveView,
};
use futures_util::{
    sink::{Sink, SinkExt},
    stream::StreamExt,
    TryStream, TryStreamExt,
};
use http::{HeaderMap, Uri};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
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
) -> anyhow::Result<()>
where
    L: LiveView,
    W: Sink<MessageToSocket> + Unpin,
    W::Error: Into<anyhow::Error>,
    R: TryStream<Ok = MessageFromSocket<L::Message>> + Unpin,
    R::Error: Into<anyhow::Error>,
{
    let view = spawn_view(view);

    let (tx, rx) = mpsc::channel(1024);
    let handle = ViewHandle::new(tx);

    view.mount(uri, headers, handle).await?;

    let markup = view.render().await?;

    write_message(&mut write, MessageToSocketData::InitialRender(markup)).await?;

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
                let err = err.into();
                tracing::trace!(%err, "error from socket");
                break;
            }
            None => {
                tracing::trace!("no more messages on socket");
                break;
            }
        };

        let data = Option::<EventData>::from(data);

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

fn spawn_view<L>(mut view: L) -> ViewTaskHandle<L::Message>
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
                    if let Err(err) = view.mount(uri, &headers, handle).await {
                        tracing::error!("mount failed with error: {}", err);
                        break;
                    }
                    let _ = reply_tx.send(());
                }
                ViewRequest::Render { reply_tx } => {
                    let _ = reply_tx.send(markup.serialize());
                }
                ViewRequest::Update {
                    msg,
                    reply_tx,
                    event_data,
                } => {
                    let (new_view, js_commands) = match view.update(msg, event_data).await {
                        Ok(updated) => updated.into_parts(),
                        Err(err) => {
                            tracing::error!("View failed with error: {}", err);
                            break;
                        }
                    };

                    view = new_view;

                    let new_markup = wrap_in_live_view_container(view.render());
                    let diff = markup.diff(&new_markup);
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
    async fn mount(
        &self,
        uri: Uri,
        headers: HeaderMap,
        handle: ViewHandle<M>,
    ) -> anyhow::Result<()> {
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
            .map_err(|_| anyhow::anyhow!("live view task ended unexpectedly"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("live view didn't response to mount request"))
    }

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

    async fn update(
        &self,
        msg: M,
        event_data: Option<EventData>,
    ) -> anyhow::Result<UpdateResponse> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let request = ViewRequest::Update {
            msg,
            event_data,
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
    Mount {
        uri: Uri,
        headers: HeaderMap,
        handle: ViewHandle<M>,
        reply_tx: oneshot::Sender<()>,
    },
    Render {
        reply_tx: oneshot::Sender<html::Serialized>,
    },
    Update {
        msg: M,
        event_data: Option<EventData>,
        reply_tx: oneshot::Sender<UpdateResponse>,
    },
}

enum UpdateResponse {
    Diff(html::Diff),
    JsCommands(Vec<JsCommand>),
    DiffAndJsCommands(html::Diff, Vec<JsCommand>),
    Empty,
}

#[derive(Serialize, Debug)]
pub(crate) struct MessageToSocket {
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
pub(crate) struct MessageFromSocket<M> {
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
        value: InputValue,
    },
    InputFocus {
        #[serde(rename = "v")]
        value: String,
    },
    InputBlur {
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
    use crate as axum_live_view;
    axum_live_view_macros::html! {
        <div id="live-view-container">{ markup }</div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as axum_live_view;
    use crate::html::Html;
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
            json!({ "m": "Incr", "t": "form_submit", "d": { "q": "name=bob&age=20" } }),
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
