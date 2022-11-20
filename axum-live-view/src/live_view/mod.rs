//! Server-rendered live views.

use crate::{event_data::EventData, html::Html, js_command::JsCommand};
use axum::http::{HeaderMap, Uri};
use futures_util::Stream;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, future::Future, pin::Pin};
use tokio::sync::mpsc;

mod combine;

/// A server-rendered live view.
///
/// This is the trait you implement to create live views.
///
/// # Example
///
/// ```
/// use axum_live_view::{
///     Html,
///     LiveView,
///     html,
///     event_data::EventData,
///     live_view::Updated,
/// };
/// use std::convert::Infallible;
/// use serde::{Deserialize, Serialize};
///
/// struct Counter {
///     count: u64,
/// }
///
/// impl LiveView for Counter {
///     type Message = Msg;
///
///     // Update the view based on which message it receives.
///     fn update(
///         mut self,
///         msg: Msg,
///         data: Option<EventData>,
///     ) -> Updated<Self> {
///         match msg {
///             Msg::Increment => {
///                 self.count += 1;
///             }
///             Msg::Decrement => {
///                 if self.count > 0 {
///                     self.count -= 1;
///                 }
///             }
///         }
///
///         Updated::new(self)
///     }
///
///     // Render the live view into an HTML template.
///     fn render(&self) -> Html<Self::Message> {
///         html! {
///             <div>
///                 "Counter value: "
///                 { self.count }
///             </div>
///
///             <div>
///                 // `axm-click` is a binding that sends the view a message when its clicked
///                 <button axm-click={ Msg::Increment }>"+"</button>
///                 <button axm-click={ Msg::Decrement }>"-"</button>
///             </div>
///         }
///     }
/// }
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq)]
/// enum Msg {
///     Increment,
///     Decrement,
/// }
/// ```
pub trait LiveView: Sized + Send + Sync + 'static {
    /// The message type this view receives.
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;

    /// Perform additional setup of the view once its fully connected to the WebSocket.
    ///
    /// The default implementation does nothing and simply returns immediately.
    ///
    /// This can be used to load additional data that isn't necessary for the initial render.
    ///
    /// If an error is returned the view will be shutdown, the JavaScript client will reconnect,
    /// and a fresh instance of the view will be created.
    ///
    /// The provided [`ViewHandle`] can be used to send messages to the view that don't come from
    /// the client. See the documentation for [`ViewHandle`] for examples.
    #[allow(unused_variables)]
    fn mount(&mut self, uri: Uri, request_headers: &HeaderMap, handle: ViewHandle<Self::Message>) {}

    /// React to a message and asynchronously update the view.
    ///
    /// If an error is returned the view will be shutdown, the JavaScript client will reconnect,
    /// and a fresh instance of the view will be created. Ideally you should handle errors
    /// gracefully and present them to the end user.
    fn update(self, msg: Self::Message, data: Option<EventData>) -> Updated<Self>;

    /// Render the views HTML.
    ///
    /// This method will be called after [`update`](LiveView::update) and the changes will be
    /// effeciently sent to the client.
    fn render(&self) -> Html<Self::Message>;
}

/// An updated live view as returned by [`LiveView::update`].
pub struct Updated<T>
where
    T: LiveView,
{
    pub(crate) live_view: T,
    pub(crate) js_commands: Vec<JsCommand>,
    pub(crate) spawns: Vec<Pin<Box<dyn Future<Output = T::Message> + Send + 'static>>>,
}

impl<T> fmt::Debug for Updated<T>
where
    T: LiveView + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            live_view,
            js_commands,
            spawns: _,
        } = self;

        f.debug_struct("Updated")
            .field("live_view", &live_view)
            .field("js_commands", &js_commands)
            .finish()
    }
}

impl<T> Updated<T>
where
    T: LiveView,
{
    /// Create a new `Updated` from the given view.
    pub fn new(live_view: T) -> Self {
        Self {
            live_view,
            js_commands: Default::default(),
            spawns: Default::default(),
        }
    }

    /// Include a [`JsCommand`] with this update.
    ///
    /// [`JsCommand`] can be used to perform updates that can't otherwise be done with
    /// [`LiveView::render`], such as setting the `<title>` or navigating to another page.
    ///
    /// Calling this method multiple times will not override previous values.
    pub fn with(mut self, js_command: JsCommand) -> Self {
        self.js_commands.push(js_command);
        self
    }

    /// Include an iterator of [`JsCommand`]s with this update.
    ///
    /// [`JsCommand`] can be used to perform updates that can't otherwise be done with
    /// [`LiveView::render`], such as setting the `<title>` or navigating to another page.
    ///
    /// Calling this method multiple times will not override previous values.
    pub fn with_all<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.extend(commands);
        self
    }

    /// Spawn a future to run when this `Updated` is passed back to axum-live-view.
    ///
    /// The future must yield a message which is trigger [`LiveView::update`] to be called.
    ///
    /// This can be used to perform an async operation, have it run in the background so the view
    /// is free to handle other messages, and get called when the future yields a message.
    ///
    /// Calling this method multiple times will not override previous values.
    ///
    /// Note that if the view was spawned with [`test::run_live_view`] the futures
    /// will _not_ be spawned but will simply be dropped. [`TestViewHandle::send`] should be used
    /// to send messages to the view from tests, as this is deterministic and yields the updated
    /// HTML template.
    ///
    /// [`TestViewHandle::send`]: crate::test::TestViewHandle::send
    /// [`test::run_live_view`]: crate::test::run_live_view
    pub fn spawn<F>(mut self, future: F) -> Self
    where
        F: Future<Output = T::Message> + Send + 'static,
    {
        self.spawns.push(Box::pin(future));
        self
    }
}

impl<T> Extend<JsCommand> for Updated<T>
where
    T: LiveView,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.js_commands.extend(iter);
    }
}

/// A handle to a [`LiveView`] that can be used to send messages to the view that don't come from
/// the client.
///
/// A [`ViewHandle`] is passed to [`LiveView::mount`].
pub struct ViewHandle<M> {
    tx: mpsc::Sender<M>,
}

impl<M> ViewHandle<M> {
    pub(crate) fn new() -> (Self, mpsc::Receiver<M>) {
        let (tx, rx) = mpsc::channel(1024);
        (Self { tx }, rx)
    }

    /// Send a message to the view.
    ///
    /// This will cause [`LiveView::update`] to be called.
    ///
    /// # Example
    ///
    /// This can for example to be used to re-render a view on an interval:
    ///
    /// ```
    /// use axum::{response::IntoResponse, http::{Uri, HeaderMap}};
    /// use axum_live_view::{
    ///     Html,
    ///     LiveView,
    ///     live_view::ViewHandle,
    /// };
    /// use std::{convert::Infallible, time::Duration};
    /// use serde::{Deserialize, Serialize};
    ///
    /// struct MyView;
    ///
    /// impl LiveView for MyView {
    ///     type Message = Msg;
    ///
    ///     fn mount(
    ///         &mut self,
    ///         uri: Uri,
    ///         request_headers: &HeaderMap,
    ///         handle: ViewHandle<Self::Message>,
    ///     ) {
    ///         tokio::spawn(async move {
    ///             let mut interval = tokio::time::interval(Duration::from_secs(1));
    ///             loop {
    ///                 interval.tick().await;
    ///                 if handle.send(Msg::Tick).await.is_err() {
    ///                     // view has been shutdown
    ///                     break;
    ///                 }
    ///             }
    ///         });
    ///     }
    ///
    ///     // ...
    ///     # fn update(
    ///     #     self,
    ///     #     msg: Msg,
    ///     #     data: Option<axum_live_view::event_data::EventData>,
    ///     # ) -> axum_live_view::live_view::Updated<Self> {
    ///     #     unimplemented!()
    ///     # }
    ///     # fn render(&self) -> axum_live_view::Html<Self::Message> {
    ///     #     unimplemented!()
    ///     # }
    /// }
    ///
    /// #[derive(Serialize, Deserialize, Debug, PartialEq)]
    /// enum Msg {
    ///     Tick,
    /// }
    /// ```
    pub async fn send(&self, msg: M) -> Result<(), ViewHandleSendError> {
        self.tx.send(msg).await.map_err(|_| ViewHandleSendError)?;
        Ok(())
    }

    /// Forward all messages from a [`Stream`] to the view.
    ///
    /// This will cause [`LiveView::update`] to be called whenever the stream yields a new item.
    ///
    /// Note the future returned by this method will loop until the stream ends. Therefore its
    /// commonly used with [`tokio::spawn`].
    pub async fn forward<S>(self, stream: S)
    where
        S: Stream<Item = M>,
    {
        use futures_util::StreamExt;

        futures_util::pin_mut!(stream);

        while let Some(msg) = stream.next().await {
            if self.send(msg).await.is_err() {
                break;
            }
        }
    }

    pub(crate) fn with<F, M2>(self, f: F) -> ViewHandle<M2>
    where
        F: Fn(M2) -> M + Send + Sync + 'static,
        M2: Send + 'static,
        M: Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<M2>(1024);
        let old_tx = self.tx;

        // probably not the most effecient thing to spawn here
        // might be worth moving to using a `Sink` and using `SinkExt::with`
        // will probably require boxing since `ViewHandle` should only
        // be generic over the message
        crate::util::spawn_unit(async move {
            while let Some(msg) = rx.recv().await {
                if old_tx.send(f(msg)).await.is_err() {
                    break;
                }
            }
        });

        ViewHandle { tx }
    }
}

impl<M> Clone for ViewHandle<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M> fmt::Debug for ViewHandle<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewHandle").finish()
    }
}

/// Error returned from [`ViewHandle::send`] if the view has been shutdown.
#[non_exhaustive]
#[derive(Debug)]
pub struct ViewHandleSendError;

impl fmt::Display for ViewHandleSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to send message to view")
    }
}

impl std::error::Error for ViewHandleSendError {}

/// Combine multiple live views into one.
///
/// Normally you cannot combine two or more views in the same `html!` template because the message
/// types are different. `combine` can be used to work around that.
///
/// # Example
///
/// ```
/// use axum::response::IntoResponse;
/// use axum_live_view::{
///     Html,
///     LiveView,
///     LiveViewUpgrade,
///     html,
///     event_data::EventData,
///     live_view::Updated,
/// };
/// use std::convert::Infallible;
/// use serde::{Deserialize, Serialize};
///
/// // `Foo` and `Bar` are live views with different message types
/// struct Foo {}
///
/// impl LiveView for Foo {
///     type Message = FooMsg;
///
///     // ...
///     # fn update(
///     #     mut self,
///     #     msg: FooMsg,
///     #     data: Option<EventData>,
///     # ) -> Updated<Self> {
///     #     unimplemented!()
///     # }
///     # fn render(&self) -> Html<Self::Message> {
///     #     unimplemented!()
///     # }
/// }
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq)]
/// enum FooMsg {}
///
/// struct Bar {}
///
/// impl LiveView for Bar {
///     type Message = BarMsg;
///
///     // ...
///     # fn update(
///     #     mut self,
///     #     msg: BarMsg,
///     #     data: Option<EventData>,
///     # ) -> Updated<Self> {
///     #     unimplemented!()
///     # }
///     # fn render(&self) -> Html<Self::Message> {
///     #     unimplemented!()
///     # }
/// }
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq)]
/// enum BarMsg {}
///
/// async fn handle(live: LiveViewUpgrade) -> impl IntoResponse {
///     live.response(|embed_live_view| {
///         // instantiate each view
///         let foo = Foo {};
///         let bar = Bar {};
///
///         // combine them into one view
///         let combined_view = axum_live_view::live_view::combine(
///             // a tuple with each view we want to combine
///             (foo, bar),
///             // a closure that takes each view and returns the combined HTML
///             |foo, bar| {
///                 html! {
///                     <div class="foo">{ foo }</div>
///                     <div class="bar">{ bar }</div>
///                 }
///             });
///
///         html! {
///             { embed_live_view.embed(combined_view) }
///         }
///     })
/// }
/// ```
pub fn combine<V, F>(views: V, render: F) -> combine::Combine<V, F> {
    combine::Combine { views, render }
}
