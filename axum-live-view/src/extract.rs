use self::rejection::*;
use crate::{html::Html, life_cycle::run_view, LiveView};
use async_trait::async_trait;
use axum::{
    extract::{
        ws::{self, WebSocket, WebSocketUpgrade},
        FromRequest, RequestParts,
    },
    http::{HeaderMap, Uri},
    response::{IntoResponse, Response},
};
use futures_util::{
    sink::SinkExt,
    stream::{StreamExt, TryStreamExt},
};
use std::fmt::Debug;

pub use crate::life_cycle::EmbedLiveView;

#[derive(Debug)]
pub struct LiveViewUpgrade {
    inner: LiveViewUpgradeInner,
}

#[derive(Debug)]
enum LiveViewUpgradeInner {
    Http,
    Ws(Box<(WebSocketUpgrade, Uri, HeaderMap)>),
}

#[async_trait]
impl<B> FromRequest<B> for LiveViewUpgrade
where
    B: Send,
{
    type Rejection = LiveViewUpgradeRejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        if let Ok(ws) = WebSocketUpgrade::from_request(req).await {
            let uri = req.uri().clone();
            let headers = req.headers().cloned().ok_or_else(|| {
                LiveViewUpgradeRejection::HeadersAlreadyExtracted(Default::default())
            })?;

            Ok(Self {
                inner: LiveViewUpgradeInner::Ws(Box::new((ws, uri, headers))),
            })
        } else {
            Ok(Self {
                inner: LiveViewUpgradeInner::Http,
            })
        }
    }
}

impl LiveViewUpgrade {
    pub fn response<F, L>(self, gather_view: F) -> Response
    where
        L: LiveView,
        F: FnOnce(EmbedLiveView<'_, L>) -> Html<L::Message>,
    {
        let (ws, uri, headers) = match self.inner {
            LiveViewUpgradeInner::Http => {
                let embed = EmbedLiveView::noop();
                return gather_view(embed).into_response();
            }
            LiveViewUpgradeInner::Ws(data) => *data,
        };

        let mut view = None;

        let embed = EmbedLiveView::new(&mut view);

        gather_view(embed);

        let view = if let Some(view) = view {
            view
        } else {
            return ws.on_upgrade(|_| async {}).into_response();
        };

        ws.on_upgrade(|socket| run_view_on_socket(socket, view, uri, headers))
            .into_response()
    }
}

async fn run_view_on_socket<L>(socket: WebSocket, view: L, uri: Uri, headers: HeaderMap)
where
    L: LiveView,
{
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

    if let Err(err) = run_view(write, read, view, uri, headers).await {
        tracing::error!(%err, "encountered while processing socket");
    }
}

pub mod rejection {
    use axum::{
        extract::rejection::HeadersAlreadyExtracted,
        response::{IntoResponse, Response},
    };

    #[derive(Debug)]
    #[non_exhaustive]
    pub enum LiveViewUpgradeRejection {
        HeadersAlreadyExtracted(HeadersAlreadyExtracted),
    }

    impl IntoResponse for LiveViewUpgradeRejection {
        fn into_response(self) -> Response {
            match self {
                Self::HeadersAlreadyExtracted(inner) => inner.into_response(),
            }
        }
    }
}
