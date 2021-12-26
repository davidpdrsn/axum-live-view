use crate::{
    html,
    liveview::WithEventContext,
    pubsub::{Decode, Encode, Topic},
    ws::JsCommand,
};
use axum::Json;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use uuid::Uuid;

pub(crate) fn mounted(liveview_id: Uuid) -> impl Topic<Message = ()> {
    liveview_local(liveview_id, "mounted")
}

pub(crate) fn initial_render(liveview_id: Uuid) -> impl Topic<Message = Json<html::Serialized>> {
    liveview_local(liveview_id, "initial-render")
}

pub(crate) fn rendered(liveview_id: Uuid) -> impl Topic<Message = Json<html::Diff>> {
    liveview_local(liveview_id, "rendered")
}

pub(crate) fn js_command(liveview_id: Uuid) -> impl Topic<Message = Json<JsCommand>> {
    liveview_local(liveview_id, "js-command")
}

pub(crate) fn socket_disconnected(liveview_id: Uuid) -> impl Topic<Message = ()> {
    liveview_local(liveview_id, "socket-disconnected")
}

pub(crate) fn update<M>(liveview_id: Uuid) -> FixedTopic<Json<WithEventContext<M>>>
where
    M: Serialize + DeserializeOwned + Send + 'static,
{
    liveview_local(liveview_id, "update")
}

fn liveview_local<M>(liveview_id: Uuid, topic: &str) -> FixedTopic<M>
where
    M: Encode + Decode + Send + 'static,
{
    FixedTopic::new(format!("liveview/{}/{}", liveview_id, topic))
}

pub(crate) struct FixedTopic<M> {
    topic: String,
    _marker: PhantomData<fn() -> M>,
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
