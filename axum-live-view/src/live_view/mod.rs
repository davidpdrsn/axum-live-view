use crate::{html::Html, js_command::JsCommand};
use axum::async_trait;
use axum_liveview_macros::html;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

mod associated_data;
pub(crate) mod embed;
mod lifecycle;
mod subscriptions;

pub use self::{
    associated_data::{EventData, FormEventData, KeyEventData, MouseEventData},
    embed::EmbedLiveView,
    subscriptions::Subscriptions,
};

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;

    fn init(&self, _subscriptions: &mut Subscriptions<Self>) {}

    async fn update(self, msg: Self::Message, data: EventData) -> Updated<Self>;

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

#[derive(Debug, Clone)]
pub struct Updated<T> {
    liveview: T,
    js_commands: Vec<JsCommand>,
    skip_render: bool,
}

impl<T> Updated<T> {
    pub fn new(liveview: T) -> Self {
        Self {
            liveview,
            js_commands: Default::default(),
            skip_render: false,
        }
    }

    pub fn with(mut self, js_command: JsCommand) -> Self {
        self.js_commands.push(js_command);
        self
    }

    pub fn with_all<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.extend(commands);
        self
    }

    pub fn skip_render(mut self, skip: bool) -> Self {
        self.skip_render = skip;
        self
    }
}

impl<T> Extend<JsCommand> for Updated<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.js_commands.extend(iter);
    }
}
