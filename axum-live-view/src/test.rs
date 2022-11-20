//! Utilities for testing live views.
//!
//! # Example
//!
//! ```
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
//! let view_handle = run_live_view(view).mount().await;
//!
//! // Check the initial HTML
//! assert!(view_handle.render().await.contains("0"));
//!
//! // Send the view a message and make sure the HTML changes correctly
//! let (html, js_commands) = view_handle.send(Msg::Increment, None).await;
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
//! impl LiveView for Counter {
//!     type Message = Msg;
//!
//!     fn update(
//!         mut self,
//!         msg: Msg,
//!         data: Option<EventData>,
//!     ) -> Updated<Self> {
//!         match msg {
//!             Msg::Increment => self.count += 1,
//!             Msg::Decrement => self.count -= 1,
//!         }
//!         Updated::new(self)
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

use crate::{
    event_data::EventData,
    js_command::JsCommand,
    life_cycle::{UpdateResponse, ViewRequestError, ViewTaskHandle},
    live_view::ViewHandle,
    LiveView,
};
use http::{HeaderMap, Uri};
use serde::Serialize;
use std::fmt;

/// Spawn a live view on a background task and get a handle that can simulate mounting the view.
pub fn run_live_view<L>(view: L) -> TestViewHandleBuilder<L::Message>
where
    L: LiveView,
{
    let view_task_handle = crate::life_cycle::spawn_view(view, None);

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
pub struct TestViewHandleBuilder<M>
where
    M: 'static,
{
    handle: ViewTaskHandle<M>,
    uri: Option<Uri>,
    headers: Option<HeaderMap>,
}

impl<M> TestViewHandleBuilder<M> {
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
    ///
    /// Also, the futures passed to [`Updated::spawn`] will be ignored, for the same reason.
    ///
    /// [`Updated::spawn`]: crate::live_view::Updated::spawn
    pub async fn mount(self) -> TestViewHandle<M> {
        let (handle, rx) = ViewHandle::new();
        drop(rx);

        let uri = self.uri.unwrap_or_else(|| "/".parse::<Uri>().unwrap());
        let headers = self.headers.unwrap_or_default();

        match self.handle.mount(uri, headers, handle).await {
            Ok(()) => {}
            Err(ViewRequestError::ChannelClosed(_)) => unreachable!(),
        }

        TestViewHandle {
            handle: self.handle,
        }
    }
}

impl<M> fmt::Debug for TestViewHandleBuilder<M> {
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
pub struct TestViewHandle<M>
where
    M: 'static,
{
    handle: ViewTaskHandle<M>,
}

impl<M> TestViewHandle<M>
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
    pub async fn send(&self, msg: M, data: Option<EventData>) -> (String, Vec<JsCommand>) {
        let js_commands = match self.handle.update(msg, data).await {
            Ok(UpdateResponse::Diff(_) | UpdateResponse::Empty) => Vec::new(),
            Ok(UpdateResponse::JsCommands(cmds) | UpdateResponse::DiffAndJsCommands(_, cmds)) => {
                cmds
            }
            Err(ViewRequestError::ChannelClosed(_)) => unreachable!(),
        };

        let html = self.handle.render_to_string().await.unwrap();
        (html, js_commands)
    }
}

impl<M> fmt::Debug for TestViewHandle<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestViewHandle")
            .field("handle", &self.handle)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as axum_live_view;
    use crate::event_data::Input;
    use crate::{live_view::Updated, Html};
    use axum_live_view_macros::html;
    use serde::Deserialize;

    #[tokio::test]
    async fn test_something() {
        let view = run_live_view(Counter::default()).mount().await;

        let html = view.render().await;
        assert!(html.contains('0'));

        let (html, _) = view.send(Msg::Incr, None).await;
        assert!(html.contains('1'));

        let (html, _) = view.send(Msg::Incr, None).await;
        assert!(html.contains('2'));

        let (html, _) = view.send(Msg::Decr, None).await;
        assert!(html.contains('1'));

        let (html, _) = view.send(Msg::Decr, None).await;
        assert!(html.contains('0'));

        let (html, _) = view.send(Msg::Decr, None).await;
        assert!(html.contains('0'));

        let (html, _) = view
            .send(Msg::IncrBy, Some(Input::String("10".to_owned()).into()))
            .await;
        assert!(html.contains("10"));
    }

    #[derive(Default, Clone)]
    struct Counter {
        count: u64,
    }

    impl LiveView for Counter {
        type Message = Msg;

        fn update(mut self, msg: Msg, data: Option<EventData>) -> Updated<Self> {
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

            Updated::new(self)
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
