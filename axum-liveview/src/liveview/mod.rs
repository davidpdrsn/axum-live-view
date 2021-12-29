use crate::{html::Html, AssociatedData, Subscriptions};
use axum::async_trait;
use axum_liveview_macros::html;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub(crate) mod embed;
mod lifecycle;

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;

    fn init(&self, _subscriptions: &mut Subscriptions<Self>) {}

    async fn update(self, msg: Self::Message, data: AssociatedData) -> Self;

    fn render(&self) -> Html<Self::Message>;
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct LiveViewId(Uuid);

impl LiveViewId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for LiveViewId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub(super) fn wrap_in_liveview_container<T>(liveview_id: LiveViewId, markup: Html<T>) -> Html<T> {
    use crate as axum_liveview;

    html! {
        <div class="liveview-container" data-liveview-id={ liveview_id.to_string() }>
            { markup }
        </div>
    }
}
