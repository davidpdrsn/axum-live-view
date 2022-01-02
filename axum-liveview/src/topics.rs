use crate::{
    html,
    js::JsCommand,
    liveview::LiveViewId,
    pubsub::{Decode, Encode, Topic},
    ws::WithAssociatedData,
};
use axum::Json;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, marker::PhantomData};

pub(crate) fn mounted(liveview_id: LiveViewId) -> impl Topic<Message = ()> {
    liveview_local(liveview_id, "mounted")
}

pub(crate) fn initial_render(
    liveview_id: LiveViewId,
) -> impl Topic<Message = Json<html::Serialized>> {
    liveview_local(liveview_id, "initial-render")
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum RenderedMessage {
    Diff(html::Diff),
    DiffWithCommands(html::Diff, Vec<JsCommand>),
    Commands(Vec<JsCommand>),
}

pub(crate) fn rendered(liveview_id: LiveViewId) -> impl Topic<Message = Json<RenderedMessage>> {
    liveview_local(liveview_id, "rendered")
}

pub(crate) fn socket_disconnected(liveview_id: LiveViewId) -> impl Topic<Message = ()> {
    liveview_local(liveview_id, "socket-disconnected")
}

pub(crate) fn update<M>(liveview_id: LiveViewId) -> FixedTopic<Json<WithAssociatedData<M>>>
where
    M: Serialize + DeserializeOwned + Send + 'static,
{
    liveview_local(liveview_id, "update")
}

fn liveview_local<M>(liveview_id: LiveViewId, topic: &str) -> FixedTopic<M>
where
    M: Encode + Decode + Send + 'static,
{
    FixedTopic::new(format!("liveview/{}/{}", liveview_id, topic))
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
