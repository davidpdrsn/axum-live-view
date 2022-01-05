use crate::{event_data::EventData, html::Html, js_command::JsCommand};
use axum::{
    async_trait,
    http::{HeaderMap, Uri},
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;
    type Error: fmt::Display;

    async fn mount(&mut self, _uri: Uri, _request_headers: &HeaderMap) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn update(
        self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error>;

    fn render(&self) -> Html<Self::Message>;
}

#[derive(Debug, Clone)]
pub struct Updated<T> {
    pub(crate) live_view: T,
    pub(crate) js_commands: Vec<JsCommand>,
}

impl<T> Updated<T> {
    pub fn new(live_view: T) -> Self {
        Self {
            live_view,
            js_commands: Default::default(),
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

    pub fn into_parts(self) -> (T, Vec<JsCommand>) {
        (self.live_view, self.js_commands)
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
