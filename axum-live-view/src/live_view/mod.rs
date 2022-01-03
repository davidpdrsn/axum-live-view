use crate::{html::Html, js_command::JsCommand};
use axum::async_trait;
use axum_live_view_macros::html;
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
pub trait MakeLiveView: Send + Sync + 'static {
    type LiveView: LiveView;

    async fn make_live_view(&self) -> Self::LiveView;
}

#[derive(Clone, Debug)]
pub struct Shared<T> {
    live_view: T,
}

impl<T> Shared<T> {
    pub fn new(live_view: T) -> Self
    where
        T: LiveView + Clone,
    {
        Self { live_view }
    }
}

#[async_trait]
impl<T> MakeLiveView for Shared<T>
where
    T: LiveView + Clone,
{
    type LiveView = T;

    async fn make_live_view(&self) -> Self::LiveView {
        self.live_view.clone()
    }
}

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

pub(super) fn wrap_in_live_view_container<T>(live_view_id: LiveViewId, markup: Html<T>) -> Html<T> {
    use crate as axum_live_view;

    html! {
        <div class="live-view-container" data-live-view-id={ live_view_id.to_string() }>
            { markup }
        </div>
    }
}

#[derive(Debug, Clone)]
pub struct Updated<T> {
    live_view: T,
    js_commands: Vec<JsCommand>,
    skip_render: bool,
}

impl<T> Updated<T> {
    pub fn new(live_view: T) -> Self {
        Self {
            live_view,
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
