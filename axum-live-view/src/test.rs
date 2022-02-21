//! Utilities for testing live views.
//!
//! # Example
//!
//! ```
//! use axum::async_trait;
//! use axum_live_view::{
//!     Html,
//!     LiveView,
//!     html,
//!     event_data::EventData,
//!     live_view::Updated,
//!     test::run_live_view,
//! };
//! use std::convert::Infallible;
//! use serde::{Deserialize, Serialize};
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Run our view in the background
//! let view = Counter::default();
//! let view_handle = run_live_view(view).mount().await.unwrap();
//!
//! // Check the initial HTML
//! assert!(view_handle.render().await.contains("0"));
//!
//! // Send the view a message and make sure the HTML changes correctly
//! let (html, js_commands) = view_handle.send(Msg::Increment, None).await.unwrap();
//! assert!(html.contains("1"));
//! assert!(js_commands.is_empty());
//! # }
//!
//! // A simple counter live view
//! #[derive(Default)]
//! struct Counter {
//!     count: u64,
//! }
//!
//! #[async_trait]
//! impl LiveView for Counter {
//!     type Message = Msg;
//!     type Error = Infallible;
//!
//!     async fn update(
//!         mut self,
//!         msg: Msg,
//!         data: Option<EventData>,
//!     ) -> Result<Updated<Self>, Self::Error> {
//!         match msg {
//!             Msg::Increment => self.count += 1,
//!             Msg::Decrement => self.count -= 1,
//!         }
//!         Ok(Updated::new(self))
//!     }
//!
//!     fn render(&self) -> Html<Self::Message> {
//!         html! {
//!             { self.count }
//!         }
//!     }
//! }
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! enum Msg {
//!     Increment,
//!     Decrement,
//! }
//! ```

use std::fmt;

use crate::{
    event_data::EventData,
    js_command::JsCommand,
    life_cycle::{UpdateResponse, ViewRequestError, ViewTaskHandle},
    live_view::ViewHandle,
    LiveView,
};
use http::{HeaderMap, Uri};
use serde::Serialize;

/// Spawn a live view on a background task and get a handle that can simulate mounting the view.
pub fn run_live_view<L>(view: L) -> TestViewHandleBuilder<L::Message, L::Error>
where
    L: LiveView,
{
    let view_task_handle = crate::life_cycle::spawn_view(view);

    TestViewHandleBuilder {
        handle: view_task_handle,
        uri: None,
        headers: None,
    }
}

/// Builder type for customizing how to mount a view.
///
/// Created with [`run_live_view`].
///
/// See the [module docs](self) for an example of how test live views.
pub struct TestViewHandleBuilder<M, E>
where
    M: 'static,
{
    handle: ViewTaskHandle<M, E>,
    uri: Option<Uri>,
    headers: Option<HeaderMap>,
}

impl<M, E> TestViewHandleBuilder<M, E> {
    /// Set the URI [`LiveView::mount`] will be called with.
    pub fn mount_uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    /// Set the headers [`LiveView::mount`] will be called with.
    pub fn mount_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Call [`LiveView::mount`] on the view.
    ///
    /// If `Ok(())` is returned then you'll get a [`TestViewHandle`] which can be used to send
    /// messages to the view.
    ///
    /// Note the [`ViewHandle`] passed to [`LiveView::mount`] is a fake and wont actually be
    /// connected to the view. Thus [`ViewHandle::send`] will always return an error.
    /// [`TestViewHandle::send`] should be used to send messages to the view from tests, as this is
    /// deterministic and yields the updated HTML template.
    pub async fn mount(self) -> Result<TestViewHandle<M, E>, E> {
        let (handle, rx) = ViewHandle::new();
        drop(rx);

        let uri = self.uri.unwrap_or_else(|| "/".parse::<Uri>().unwrap());
        let headers = self.headers.unwrap_or_default();

        match self.handle.mount(uri, headers, handle).await {
            Ok(()) => {}
            Err(ViewRequestError::ViewError(err)) => return Err(err),
            Err(ViewRequestError::ChannelClosed(_)) => unreachable!(),
        }

        Ok(TestViewHandle {
            handle: self.handle,
        })
    }
}

impl<M, E> fmt::Debug for TestViewHandleBuilder<M, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestViewHandleBuilder")
            .field("handle", &self.handle)
            .field("uri", &self.uri)
            .field("headers", &self.headers)
            .finish()
    }
}

/// Handle to a live view running in a background task.
///
/// Used to send messages to the view and check that the HTML changes correctly.
///
/// See the [module docs](self) for an example of how test live views.
pub struct TestViewHandle<M, E>
where
    M: 'static,
{
    handle: ViewTaskHandle<M, E>,
}

impl<M, E> TestViewHandle<M, E>
where
    M: Serialize,
{
    /// Re-render the HTML template.
    ///
    /// This calls [`LiveView::render`] on the view. This method is async because the view is
    /// running on a background task.
    pub async fn render(&self) -> String {
        self.handle.render_to_string().await.unwrap()
    }

    /// Send the view a message
    ///
    /// This calls [`LiveView::update`] on the view followed by [`LiveView::render`] and returns
    /// the HTML template and any [`JsCommand`]s included.
    pub async fn send(
        &self,
        msg: M,
        data: Option<EventData>,
    ) -> Result<(String, Vec<JsCommand>), E> {
        let js_commands = match self.handle.update(msg, data).await {
            Ok(UpdateResponse::Diff(_) | UpdateResponse::Empty) => Vec::new(),
            Ok(UpdateResponse::JsCommands(cmds) | UpdateResponse::DiffAndJsCommands(_, cmds)) => {
                cmds
            }
            Err(ViewRequestError::ViewError(err)) => return Err(err),
            Err(ViewRequestError::ChannelClosed(_)) => unreachable!(),
        };

        let html = self.handle.render_to_string().await.unwrap();
        Ok((html, js_commands))
    }
}

impl<M, E> fmt::Debug for TestViewHandle<M, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestViewHandle")
            .field("handle", &self.handle)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate as axum_live_view;
    use crate::event_data::Input;
    use crate::{live_view::Updated, Html};
    use async_trait::async_trait;
    use axum_live_view_macros::html;
    use serde::Deserialize;
    use std::convert::Infallible;

    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn test_something() {
        let view = run_live_view(Counter::default()).mount().await.unwrap();

        let html = view.render().await;
        assert!(html.contains('0'));

        let (html, _) = view.send(Msg::Incr, None).await.unwrap();
        assert!(html.contains('1'));

        let (html, _) = view.send(Msg::Incr, None).await.unwrap();
        assert!(html.contains('2'));

        let (html, _) = view.send(Msg::Decr, None).await.unwrap();
        assert!(html.contains('1'));

        let (html, _) = view.send(Msg::Decr, None).await.unwrap();
        assert!(html.contains('0'));

        let (html, _) = view.send(Msg::Decr, None).await.unwrap();
        assert!(html.contains('0'));

        let (html, _) = view
            .send(Msg::IncrBy, Some(Input::String("10".to_owned()).into()))
            .await
            .unwrap();
        assert!(html.contains("10"));
    }

    #[derive(Default, Clone)]
    struct Counter {
        count: u64,
    }

    #[async_trait]
    impl LiveView for Counter {
        type Message = Msg;
        type Error = Infallible;

        async fn update(
            mut self,
            msg: Msg,
            data: Option<EventData>,
        ) -> Result<Updated<Self>, Self::Error> {
            match msg {
                Msg::Incr => self.count += 1,
                Msg::Decr => {
                    if self.count > 0 {
                        self.count -= 1;
                    }
                }
                Msg::IncrBy => {
                    self.count += data
                        .expect("no event data")
                        .as_input()
                        .expect("not input")
                        .as_str()
                        .expect("not string")
                        .parse::<u64>()
                        .expect("parse error");
                }
            }

            Ok(Updated::new(self))
        }

        fn render(&self) -> Html<Self::Message> {
            html! {
                { self.count }
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    enum Msg {
        Incr,
        Decr,
        IncrBy,
    }
}
