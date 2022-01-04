use super::{wrap_in_live_view_container, LiveView, LiveViewId, MakeLiveView};
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

    pub fn embed<T>(&self, live_view: T) -> Html<T::Message>
    where
        T: LiveView + Clone,
        P: PubSub + Clone,
    {
        let initial_markup = live_view.render();
        let live_view_id = LiveViewId::new();
        crate::spawn_unit(super::lifecycle::run_live_view(
            live_view_id,
            live_view.clone(),
            super::Shared::new(live_view),
            self.pubsub.clone(),
        ));
        wrap_in_live_view_container(live_view_id, initial_markup)
    }

    pub async fn embed_make_liveview<T>(
        &self,
        make_live_view: T,
    ) -> Html<<T::LiveView as LiveView>::Message>
    where
        T: MakeLiveView,
        P: PubSub + Clone,
    {
        let live_view = make_live_view.make_live_view().await;
        let initial_markup = live_view.render();
        let live_view_id = LiveViewId::new();
        crate::spawn_unit(super::lifecycle::run_live_view(
            live_view_id,
            live_view,
            make_live_view,
            self.pubsub.clone(),
        ));
        wrap_in_live_view_container(live_view_id, initial_markup)
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
