use super::{wrap_in_liveview_container, LiveView, LiveViewId};
use crate::{
    html::Html,
    pubsub::{Logging, PubSub},
};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
};

#[derive(Clone, Debug)]
pub struct EmbedLiveView<P> {
    pub(crate) pubsub: Logging<P>,
}

impl<P> EmbedLiveView<P> {
    pub(crate) fn new(pubsub: P) -> Self
    where
        P: PubSub + Clone,
    {
        Self {
            pubsub: Logging::new(pubsub),
        }
    }

    pub fn embed<T>(&self, liveview: T) -> Html<T::Message>
    where
        T: LiveView,
        P: PubSub + Clone,
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
impl<B, P> FromRequest<B> for EmbedLiveView<P>
where
    B: Send,
    P: PubSub + Clone,
{
    type Rejection = <Extension<Self> as FromRequest<B>>::Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(manager) = Extension::<Self>::from_request(req).await?;
        Ok(manager)
    }
}
