use super::{wrap_in_liveview_container, LiveView, LiveViewId};
use crate::{
    html::Html,
    pubsub::{PubSub, PubSubBackend},
};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct EmbedLiveView {
    pub(crate) pubsub: Arc<dyn PubSubBackend>,
}

impl fmt::Debug for EmbedLiveView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedLiveView")
            .field("pubsub", &"...")
            .finish()
    }
}

impl EmbedLiveView {
    pub(crate) fn new<P>(pubsub: P) -> Self
    where
        P: PubSub,
    {
        Self {
            pubsub: Arc::new(pubsub),
        }
    }

    pub fn embed<T>(&self, liveview: T) -> Html<T::Message>
    where
        T: LiveView,
    {
        let initial_markup = liveview.render();
        let liveview_id = LiveViewId::new();
        tokio::spawn(super::lifecycle::run_liveview(
            liveview_id,
            liveview,
            self.pubsub.clone(),
        ));
        wrap_in_liveview_container(liveview_id, initial_markup)
    }
}

#[async_trait]
impl<B> FromRequest<B> for EmbedLiveView
where
    B: Send,
{
    type Rejection = <Extension<EmbedLiveView> as FromRequest<B>>::Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(manager) = Extension::<Self>::from_request(req).await?;
        Ok(manager)
    }
}
