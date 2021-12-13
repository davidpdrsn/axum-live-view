use crate::{html, liveview::liveview_local_topic, pubsub::PubSub, Html, LiveView, PubSubExt};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
    Json,
};
use futures_util::StreamExt;
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
    pub fn embed<T>(&self, liveview: T) -> Html
    where
        T: LiveView + Send + 'static,
    {
        let initial_content = liveview.render();
        let liveview_id = Uuid::new_v4();
        tokio::spawn(run_liveview(liveview_id, liveview, self.pubsub.clone()));
        wrap_in_liveview_container(liveview_id, initial_content)
    }
}

fn wrap_in_liveview_container(liveview_id: Uuid, markup: Html) -> Html {
    use crate as axum_liveview;

    html! {
        <div class="liveview-container" data-liveview-id={ liveview_id }>
            { markup }
        </div>
    }
}

async fn run_liveview<T, P>(liveview_id: Uuid, liveview: T, pubsub: P)
where
    T: LiveView,
    P: PubSub + Clone,
{
    let mut markup = wrap_in_liveview_container(liveview_id, liveview.render());

    let markup_stream = crate::liveview::run_to_stream(liveview, pubsub.clone(), liveview_id).await;

    futures_util::pin_mut!(markup_stream);

    let mut mounted_stream = pubsub
        .subscribe::<()>(&liveview_local_topic(liveview_id, "mounted"))
        .await;

    let mut disconnected_stream = pubsub
        .subscribe::<()>(&liveview_local_topic(liveview_id, "socket-disconnected"))
        .await;

    loop {
        tokio::select! {
            Some(new_markup) = markup_stream.next() => {
                let new_markup = wrap_in_liveview_container(liveview_id, new_markup);
                let diff = markup.diff(&new_markup);
                markup = new_markup;

                if let Err(err) = pubsub
                    .broadcast(
                        &liveview_local_topic(liveview_id, "rendered"),
                        Json(diff),
                    )
                    .await
                {
                    tracing::error!(%err, "failed to send markup on pubsub");
                }
            }

            Some(_) = mounted_stream.next() => {
                if let Err(err) = pubsub
                    .broadcast(
                        &liveview_local_topic(liveview_id, "initial-render"),
                        Json(markup.serialize()),
                    )
                    .await
                {
                    tracing::error!(%err, "failed to send `initial-render` on pubsub");
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
