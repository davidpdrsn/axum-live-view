use std::sync::Arc;
use crate::{pubsub::PubSub, LiveView};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
};
use maud::Markup;

#[derive(Clone)]
pub struct LiveViewManager {
    pubsub: Arc<dyn PubSub>,
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
}

impl LiveViewManager {
    pub fn embed<T>(&self, liveview: T) -> Markup
    where
        T: LiveView + Send + 'static,
    {
        let html = liveview.render();

        let pubsub = Arc::clone(&self.pubsub);
        tokio::spawn(async move {
            use futures_util::stream::StreamExt;

            let stream = crate::liveview::run_to_stream(liveview, pubsub).await;
            futures_util::pin_mut!(stream);

            while let Some(markup) = stream.next().await {
                dbg!(markup);
            }
        });

        html
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
