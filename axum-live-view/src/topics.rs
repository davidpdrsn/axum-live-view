use crate::{
    html,
    js_command::JsCommand,
    live_view::LiveViewId,
    pubsub::{Decode, Encode, Topic},
    ws::WithAssociatedData,
};
use axum::Json;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, marker::PhantomData};

pub(crate) fn mounted(live_view_id: LiveViewId) -> impl Topic<Message = ()> {
    live_view_local(live_view_id, "mounted")
}

pub(crate) fn initial_render(
    live_view_id: LiveViewId,
) -> impl Topic<Message = Json<html::Serialized>> {
    live_view_local(live_view_id, "initial-render")
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum RenderedMessage {
    Diff(html::Diff),
    DiffWithCommands(html::Diff, Vec<JsCommand>),
    Commands(Vec<JsCommand>),
}

pub(crate) fn rendered(live_view_id: LiveViewId) -> impl Topic<Message = Json<RenderedMessage>> {
    live_view_local(live_view_id, "rendered")
}

pub(crate) fn socket_disconnected(live_view_id: LiveViewId) -> impl Topic<Message = ()> {
    live_view_local(live_view_id, "socket-disconnected")
}

pub(crate) fn update<M>(live_view_id: LiveViewId) -> FixedTopic<Json<WithAssociatedData<M>>>
where
    M: Serialize + DeserializeOwned + Send + 'static,
{
    live_view_local(live_view_id, "update")
}

fn live_view_local<M>(live_view_id: LiveViewId, topic: &str) -> FixedTopic<M>
where
    M: Encode + Decode + Send + 'static,
{
    FixedTopic::new(format!("live_view/{}/{}", live_view_id, topic))
}

pub(crate) struct FixedTopic<M> {
    topic: String,
    _marker: PhantomData<fn() -> M>,
}

impl<M> fmt::Debug for FixedTopic<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { topic, _marker } = self;

        f.debug_struct("FixedTopic")
            .field("topic", &topic)
            .field("_marker", &_marker)
            .finish()
    }
}

impl<M> FixedTopic<M> {
    pub(crate) fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            _marker: PhantomData,
        }
    }
}

impl<M> Topic for FixedTopic<M>
where
    M: Encode + Decode + Send + 'static,
{
    type Message = M;

    fn topic(&self) -> &str {
        &self.topic
    }
}
