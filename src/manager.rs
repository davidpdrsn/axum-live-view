use crate::{liveview::liveview_local_topic, pubsub::PubSub, LiveView, PubSubExt};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
};
use futures_util::StreamExt;
use maud::{html, Markup};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct LiveViewManager {
    pub(crate) pubsub: Arc<dyn PubSub>,
}

impl LiveViewManager {
    pub(crate) fn new<P>(pubsub: P) -> Self
    where
        P: PubSub,
    {
        Self {
            pubsub: Arc::new(pubsub),
        }
    }

    pub fn pubsub(&self) -> impl PubSub {
        Arc::clone(&self.pubsub)
    }
}

impl LiveViewManager {
    pub fn embed<T>(&self, liveview: T) -> Markup
    where
        T: LiveView + Send + 'static,
    {
        let initial_content = liveview.render();
        let liveview_id = Uuid::new_v4();
        tokio::spawn(run_liveview(liveview_id, liveview, self.pubsub.clone()));
        wrap_in_liveview_container(liveview_id, initial_content)
    }
}

fn wrap_in_liveview_container(liveview_id: Uuid, markup: Markup) -> Markup {
    html! {
        div.liveview-container data-liveview-id=(liveview_id) {
            (markup)
        }
    }
}

async fn run_liveview<T, P>(liveview_id: Uuid, liveview: T, pubsub: P)
where
    T: LiveView,
    P: PubSub + Clone,
{
    let markup_stream = crate::liveview::run_to_stream(liveview, pubsub.clone(), liveview_id).await;

    futures_util::pin_mut!(markup_stream);

    let mut disconnected_stream = pubsub
        .subscribe(&liveview_local_topic(liveview_id, "socket-disconnected"))
        .await;

    loop {
        tokio::select! {
            Some(markup) = markup_stream.next() => {
                let markup = wrap_in_liveview_container(liveview_id, markup);
                if let Err(err) = pubsub
                    .send(
                        &liveview_local_topic(liveview_id, "rendered"),
                        markup.into_string(),
                    )
                    .await
                {
                    tracing::error!(%err, "failed to send markup on pubsub");
                }
            }

            Some(_) = disconnected_stream.next() => {
                tracing::trace!(%liveview_id, "shutting down liveview task");
                return;
            }
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for LiveViewManager
where
    B: Send,
{
    type Rejection = <Extension<LiveViewManager> as FromRequest<B>>::Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(manager) = Extension::<Self>::from_request(req).await?;
        Ok(manager)
    }
}
