use crate::{
    html::DiffResult, html::Html, pubsub::PubSubBackend, topics, ws::EventContext, PubSub,
    Subscriptions,
};
use async_stream::stream;
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
    Json,
};
use axum_liveview_macros::html;
use futures_util::{stream::StreamExt, Stream};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;

    fn init(&self, sub: &mut Subscriptions<Self>);

    async fn update(self, msg: Self::Message, ctx: EventContext) -> Self;

    fn render(&self) -> Html<Self::Message>;
}

#[derive(Clone)]
pub struct LiveViewManager {
    pub(crate) pubsub: Arc<dyn PubSubBackend>,
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

    pub fn embed<T>(&self, liveview: T) -> Html<T::Message>
    where
        T: LiveView,
    {
        let initial_content = liveview.render();
        let liveview_id = Uuid::new_v4();
        tokio::spawn(run_liveview(liveview_id, liveview, self.pubsub.clone()));
        wrap_in_liveview_container(liveview_id, initial_content)
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
fn wrap_in_liveview_container<T>(liveview_id: Uuid, markup: Html<T>) -> Html<T> {
    use crate as axum_liveview;
    html! {
        <div class="liveview-container" data-liveview-id={ liveview_id }>
            { markup }
        </div>
    }
}

async fn run_liveview<T, P>(liveview_id: Uuid, liveview: T, pubsub: P) -> anyhow::Result<()>
where
    T: LiveView,
    P: PubSub + Clone,
{
    let mut markup = wrap_in_liveview_container(liveview_id, liveview.render());

    let markup_stream = markup_updates_stream(liveview, pubsub.clone(), liveview_id).await?;
    futures_util::pin_mut!(markup_stream);

    let mut mounted_stream = pubsub.subscribe(&topics::mounted(liveview_id)).await?;

    let mut disconnected_stream = pubsub
        .subscribe(&topics::socket_disconnected(liveview_id))
        .await?;

    let mut mounted_streams_count = 0;

    loop {
        tokio::select! {
            Some(_) = mounted_stream.next() => {
                mounted_streams_count += 1;
                if let Err(err) = pubsub
                    .broadcast(
                        &topics::initial_render(liveview_id),
                        Json(markup.serialize()),
                    )
                    .await
                {
                    tracing::error!(%err, "failed to send `initial-render` on pubsub");
                }
            }

            Some(new_markup) = markup_stream.next() => {
                handle_new_markup(liveview_id, &mut markup, new_markup, &pubsub).await;
            }

            Some(_) = disconnected_stream.next() => {
                mounted_streams_count -= 1;

                if mounted_streams_count == 0 {
                    tracing::debug!(%liveview_id, "shutting down liveview task");
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn markup_updates_stream<T, P>(
    mut liveview: T,
    pubsub: P,
    liveview_id: Uuid,
) -> anyhow::Result<impl Stream<Item = Html<T::Message>> + Send>
where
    T: LiveView,
    P: PubSub,
{
    let mut subscriptions = Subscriptions::new(liveview_id);
    liveview.init(&mut subscriptions);

    let mut stream = subscriptions.into_stream(pubsub).await?;

    Ok(stream! {
        while let Some((callback, msg)) = stream.next().await {
            liveview = callback.call(liveview, msg).await;
            let markup = liveview.render();
            yield markup;
        }
    })
}

async fn handle_new_markup<T, P>(
    liveview_id: Uuid,
    markup: &mut Html<T>,
    new_markup: Html<T>,
    pubsub: &P,
) where
    P: PubSub,
    T: Serialize + PartialEq,
{
    let new_markup = wrap_in_liveview_container(liveview_id, new_markup);

    match markup.diff(&new_markup) {
        DiffResult::Changed(diff) => {
            *markup = new_markup;

            if let Err(err) = pubsub
                .broadcast(&topics::rendered(liveview_id), Json(diff))
                .await
            {
                tracing::error!(%err, "failed to send markup on pubsub");
            }
        }
        DiffResult::Unchanged => {}
    }
}
