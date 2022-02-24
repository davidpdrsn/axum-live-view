//! Server-rendered live views.

use crate::{event_data::EventData, html::Html, js_command::JsCommand};
use axum::{
    async_trait,
    http::{HeaderMap, Uri},
};
use futures_util::Stream;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;
use tokio::sync::mpsc;

mod combine;

/// A server-rendered live view.
///
/// This is the trait you implement to create live views.
///
/// # Example
///
/// ```
/// use axum::async_trait;
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
/// #[async_trait]
/// impl LiveView for Counter {
///     type Message = Msg;
///     type Error = Infallible;
///
///     // Update the view based on which message it receives.
///     async fn update(
///         mut self,
///         msg: Msg,
///         data: Option<EventData>,
///     ) -> Result<Updated<Self>, Self::Error> {
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
///         Ok(Updated::new(self))
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
#[async_trait]
pub trait LiveView: Sized + Send + Sync + 'static {
    /// The message type this view receives.
    type Message: Serialize + DeserializeOwned + PartialEq + Send + Sync + 'static;

    /// The error type that [`mount`] and [`update`] might fail with.
    ///
    /// [`mount`]: LiveView::mount
    /// [`update`]: LiveView::update
    type Error: fmt::Display + Send + Sync + 'static;

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
    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// React to a message and asynchronously update the view.
    ///
    /// If an error is returned the view will be shutdown, the JavaScript client will reconnect,
    /// and a fresh instance of the view will be created. Ideally you should handle errors
    /// gracefully and present them to the end user.
    async fn update(
        self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error>;

    /// Render the views HTML.
    ///
    /// This method will be called after [`update`](LiveView::update) and the changes will be
    /// effeciently sent to the client.
    fn render(&self) -> Html<Self::Message>;

    /// Map the error type of this view into another type.
    ///
    /// Used with [`combine`](fn@combine) to combine views that have otherwise different error types.
    fn map_err<F, E2>(self, f: F) -> MapErr<Self, F>
    where
        F: Fn(Self::Error) -> E2 + Send + Sync + 'static,
        E2: fmt::Display + Send + Sync + 'static,
    {
        assert_live_view::<_, Self::Message, E2>(MapErr { view: self, f })
    }
}

/// An updated live view as returned by [`LiveView::update`].
#[derive(Debug, Clone)]
pub struct Updated<T> {
    pub(crate) live_view: T,
    pub(crate) js_commands: Vec<JsCommand>,
}

impl<T> Updated<T> {
    /// Create a new `Updated` from the given view.
    pub fn new(live_view: T) -> Self {
        Self {
            live_view,
            js_commands: Default::default(),
        }
    }

    /// Include a [`JsCommand`] with this update.
    ///
    /// [`JsCommand`] can be used to perform updates that can't otherwise be done with
    /// [`LiveView::render`], such as setting the `<title>` or navigating to another page.
    pub fn with(mut self, js_command: JsCommand) -> Self {
        self.js_commands.push(js_command);
        self
    }

    /// Include an iterator of [`JsCommand`]s with this update.
    ///
    /// [`JsCommand`] can be used to perform updates that can't otherwise be done with
    /// [`LiveView::render`], such as setting the `<title>` or navigating to another page.
    pub fn with_all<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = JsCommand>,
    {
        self.extend(commands);
        self
    }

    /// Map the contained view with a function.
    ///
    /// This does not alter the attached [`JsCommand`]s.
    pub fn map<F, K>(self, f: F) -> Updated<K>
    where
        F: FnOnce(T) -> K,
    {
        Updated {
            live_view: f(self.live_view),
            js_commands: self.js_commands,
        }
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

/// Helper function used to verify that some type is actually a `LiveView`.
#[inline]
fn assert_live_view<V, M, E>(v: V) -> V
where
    V: LiveView<Message = M, Error = E>,
{
    v
}

/// A [`LiveView`] that has had its error type mapped via function.
///
/// Created with [`LiveView::map_err`].
pub struct MapErr<V, F> {
    view: V,
    f: F,
}

impl<V, F> fmt::Debug for MapErr<V, F>
where
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapErr")
            .field("view", &self.view)
            .field("f", &format_args!("{}", std::any::type_name::<F>()))
            .finish()
    }
}

#[async_trait]
impl<V, F, E2> LiveView for MapErr<V, F>
where
    V: LiveView,
    F: Fn(V::Error) -> E2 + Send + Sync + 'static,
    E2: fmt::Display + Send + Sync + 'static,
{
    type Message = V::Message;
    type Error = E2;

    async fn mount(
        &mut self,
        uri: Uri,
        request_headers: &HeaderMap,
        handle: ViewHandle<Self::Message>,
    ) -> Result<(), Self::Error> {
        self.view
            .mount(uri, request_headers, handle)
            .await
            .map_err(&self.f)
    }

    async fn update(
        mut self,
        msg: Self::Message,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        let Updated {
            live_view,
            js_commands,
        } = self.view.update(msg, data).await.map_err(&self.f)?;
        self.view = live_view;
        Ok(Updated::new(self).with_all(js_commands))
    }

    fn render(&self) -> Html<Self::Message> {
        self.view.render()
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
    /// use axum::{async_trait, response::IntoResponse, http::{Uri, HeaderMap}};
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
    /// #[async_trait]
    /// impl LiveView for MyView {
    ///     type Message = Msg;
    ///
    ///     async fn mount(
    ///         &mut self,
    ///         uri: Uri,
    ///         request_headers: &HeaderMap,
    ///         handle: ViewHandle<Self::Message>,
    ///     ) -> Result<(), Self::Error> {
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
    ///
    ///         Ok(())
    ///     }
    ///
    ///     // ...
    ///     # type Error = Infallible;
    ///     # async fn update(
    ///     #     mut self,
    ///     #     msg: Msg,
    ///     #     data: Option<axum_live_view::event_data::EventData>,
    ///     # ) -> Result<axum_live_view::live_view::Updated<Self>, Self::Error> {
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
    ///
    /// # Example
    ///
    /// This can for example to be used to subscribe to events emitted elsewhere in the application
    /// and update the view:
    ///
    /// ```
    /// use axum::{async_trait, response::IntoResponse, http::{Uri, HeaderMap}};
    /// use axum_live_view::{
    ///     Html,
    ///     LiveView,
    ///     live_view::ViewHandle,
    /// };
    /// use std::{convert::Infallible, time::Duration};
    /// use serde::{Deserialize, Serialize};
    /// use tokio::sync::broadcast;
    /// use futures_util::stream::StreamExt;
    ///
    /// struct MyView {
    ///     tx: broadcast::Sender<Msg>,
    /// }
    ///
    /// #[async_trait]
    /// impl LiveView for MyView {
    ///     type Message = Msg;
    ///
    ///     async fn mount(
    ///         &mut self,
    ///         uri: Uri,
    ///         request_headers: &HeaderMap,
    ///         handle: ViewHandle<Self::Message>,
    ///     ) -> Result<(), Self::Error> {
    ///         // create another receiver
    ///         let rx = self.tx.subscribe();
    ///
    ///         // convert the receiver into a `Stream`
    ///         let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
    ///             // ignore errors
    ///             .filter_map(|result| async move { result.ok() });
    ///
    ///         // forward all messages from the stream to the view
    ///         tokio::spawn(handle.forward(stream));
    ///         
    ///         Ok(())
    ///     }
    ///
    ///     // ...
    ///     # type Error = Infallible;
    ///     # async fn update(
    ///     #     mut self,
    ///     #     msg: Msg,
    ///     #     data: Option<axum_live_view::event_data::EventData>,
    ///     # ) -> Result<axum_live_view::live_view::Updated<Self>, Self::Error> {
    ///     #     unimplemented!()
    ///     # }
    ///     # fn render(&self) -> axum_live_view::Html<Self::Message> {
    ///     #     unimplemented!()
    ///     # }
    /// }
    ///
    /// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    /// enum Msg {
    ///     SomethingHappened,
    /// }
    /// ```
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
/// use axum::{async_trait, response::IntoResponse};
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
/// #[async_trait]
/// impl LiveView for Foo {
///     type Message = FooMsg;
///
///     // ...
///     # type Error = Infallible;
///     # async fn update(
///     #     mut self,
///     #     msg: FooMsg,
///     #     data: Option<EventData>,
///     # ) -> Result<Updated<Self>, Self::Error> {
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
/// #[async_trait]
/// impl LiveView for Bar {
///     type Message = BarMsg;
///
///     // ...
///     # type Error = Infallible;
///     # async fn update(
///     #     mut self,
///     #     msg: BarMsg,
///     #     data: Option<EventData>,
///     # ) -> Result<Updated<Self>, Self::Error> {
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
///
/// # Combining views with different error types
///
/// `combine` requires the views to have the same error type. Use [`LiveView::map_err`] to convert
/// error types if necessary:
///
/// ```
/// use axum::{async_trait, response::IntoResponse};
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
/// // `Foo` and `Bar` are live views with different message types _and different error types_
/// struct Foo {}
///
/// #[async_trait]
/// impl LiveView for Foo {
///     type Message = FooMsg;
///     type Error = std::io::Error;
///
///     // ...
///     # async fn update(
///     #     mut self,
///     #     msg: FooMsg,
///     #     data: Option<EventData>,
///     # ) -> Result<Updated<Self>, Self::Error> {
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
/// #[async_trait]
/// impl LiveView for Bar {
///     type Message = BarMsg;
///     type Error = Infallible;
///
///     // ...
///     # async fn update(
///     #     mut self,
///     #     msg: BarMsg,
///     #     data: Option<EventData>,
///     # ) -> Result<Updated<Self>, Self::Error> {
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
///
///         let bar = Bar {};
///         // convert `Infallible` into a `std::io::Error` so it matches `Foo`'s error type
///         let bar = bar.map_err(|err: Infallible| match err {});
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
