//! Data associated with events from the client such as click or form events.

mod inner {
    use crate::life_cycle::{self, EventMessageFromSocketData};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt;

    /// The data for an event that happened on the client.
    ///
    /// This is passed to [`LiveView::update`].
    ///
    /// [`LiveView::update`]: crate::live_view::LiveView::update
    #[derive(Debug, Clone)]
    #[non_exhaustive]
    pub enum EventData {
        /// An form event.
        ///
        /// See [`Form`] for more details.
        Form(Form),
        /// An input event.
        ///
        /// See [`Input`] for more details.
        Input(Input),
        /// An key event.
        ///
        /// See [`Key`] for more details.
        Key(Key),
        /// A mouse event.
        ///
        /// See [`Mouse`] for more details.
        Mouse(Mouse),
        /// A scroll event.
        ///
        /// See [`Scroll`] for more details.
        Scroll(Scroll),
    }

    impl_from!(EventData::Form);
    impl_from!(EventData::Input);
    impl_from!(EventData::Key);
    impl_from!(EventData::Mouse);
    impl_from!(EventData::Scroll);

    impl EventData {
        /// Get the inner [`Form`] if any.
        pub fn as_form(&self) -> Option<&Form> {
            if let Self::Form(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        /// Get the inner [`Input`] if any.
        pub fn as_input(&self) -> Option<&Input> {
            if let Self::Input(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        /// Get the inner [`Key`] if any.
        pub fn as_key(&self) -> Option<&Key> {
            if let Self::Key(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        /// Get the inner [`Mouse`] if any.
        pub fn as_mouse(&self) -> Option<&Mouse> {
            if let Self::Mouse(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        /// Get the inner [`Scroll`] if any.
        pub fn as_scroll(&self) -> Option<&Scroll> {
            if let Self::Scroll(inner) = self {
                Some(inner)
            } else {
                None
            }
        }
    }

    impl From<EventMessageFromSocketData> for Option<EventData> {
        fn from(data: EventMessageFromSocketData) -> Self {
            match data {
                EventMessageFromSocketData::Click
                | EventMessageFromSocketData::WindowFocus
                | EventMessageFromSocketData::WindowBlur
                | EventMessageFromSocketData::None => None,
                EventMessageFromSocketData::Form { query } => Some(EventData::Form(Form { query })),
                EventMessageFromSocketData::Input { value } => {
                    let value = match value {
                        life_cycle::InputValue::Bool(x) => Input::Bool(x),
                        life_cycle::InputValue::String(x) => Input::String(x),
                        life_cycle::InputValue::Strings(x) => Input::Strings(x),
                    };
                    Some(EventData::Input(value))
                }
                EventMessageFromSocketData::Key {
                    key,
                    code,
                    alt,
                    ctrl,
                    shift,
                    meta,
                } => Some(EventData::Key(Key {
                    key,
                    code,
                    alt,
                    ctrl,
                    shift,
                    meta,
                })),
                EventMessageFromSocketData::Mouse {
                    client_x,
                    client_y,
                    page_x,
                    page_y,
                    offset_x,
                    offset_y,
                    movement_x,
                    movement_y,
                    screen_x,
                    screen_y,
                } => Some(EventData::Mouse(Mouse {
                    client_x,
                    client_y,
                    page_x,
                    page_y,
                    offset_x,
                    offset_y,
                    movement_x,
                    movement_y,
                    screen_x,
                    screen_y,
                })),
                EventMessageFromSocketData::Scroll { scroll_x, scroll_y } => {
                    Some(EventData::Scroll(Scroll { scroll_x, scroll_y }))
                }
            }
        }
    }

    /// A form event.
    ///
    /// This event type is sent for these bindings:
    ///
    /// - `axm-submit`
    /// - `axm-change`
    #[derive(Debug, Clone)]
    pub struct Form {
        query: String,
    }

    impl Form {
        /// Get a [`FormBuilder`] for `Form`.
        ///
        /// This allows creating `Form` events for example for use in tests.
        pub fn builder() -> FormBuilder {
            FormBuilder::default()
        }

        /// Deserialize the form into some type.
        pub fn deserialize<T>(&self) -> Result<T, FormSerializationError>
        where
            T: DeserializeOwned,
        {
            let query = percent_encoding::percent_decode_str(&self.query)
                .decode_utf8()
                .map_err(|err| {
                    FormSerializationError(QuerySerializationErrorKind::Utf8Error(err))
                })?;

            let t = serde_qs::from_str(&*query).map_err(|err| {
                FormSerializationError(QuerySerializationErrorKind::Serialization(err))
            })?;

            Ok(t)
        }
    }

    /// The error returned if a form couldn't be serialized or deserialized.
    #[derive(Debug)]
    pub struct FormSerializationError(QuerySerializationErrorKind);

    #[derive(Debug)]
    enum QuerySerializationErrorKind {
        Utf8Error(std::str::Utf8Error),
        Serialization(serde_qs::Error),
    }

    impl fmt::Display for FormSerializationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.0 {
                QuerySerializationErrorKind::Utf8Error(inner) => inner.fmt(f),
                QuerySerializationErrorKind::Serialization(inner) => inner.fmt(f),
            }
        }
    }

    impl std::error::Error for FormSerializationError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match &self.0 {
                QuerySerializationErrorKind::Utf8Error(inner) => Some(&*inner),
                QuerySerializationErrorKind::Serialization(inner) => Some(&*inner),
            }
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct FormBuilder {
        query: String,
    }

    impl FormBuilder {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn serialize<T>(mut self, value: &T) -> Result<Self, FormSerializationError>
        where
            T: Serialize,
        {
            let query = serde_qs::to_string(value).map_err(|err| {
                FormSerializationError(QuerySerializationErrorKind::Serialization(err))
            })?;
            let query =
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
                    .to_string();
            self.query = query;
            Ok(self)
        }

        pub fn build(self) -> Form {
            Form { query: self.query }
        }
    }

    #[derive(Clone, Debug)]
    pub enum Input {
        Bool(bool),
        String(String),
        Strings(Vec<String>),
    }

    impl Input {
        pub fn as_bool(&self) -> Option<bool> {
            if let Self::Bool(inner) = self {
                Some(*inner)
            } else {
                None
            }
        }

        pub fn as_str(&self) -> Option<&str> {
            if let Self::String(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        pub fn as_strings(&self) -> Option<&[String]> {
            if let Self::Strings(inner) = self {
                Some(inner)
            } else {
                None
            }
        }
    }

    builder! {
        #[builder_name = KeyBuilder]
        #[derive(Debug, Clone)]
        pub struct Key {
            key: String,
            code: String,
            alt: bool,
            ctrl: bool,
            shift: bool,
            meta: bool,
        }
    }

    impl Key {
        pub fn key(&self) -> &str {
            &self.key
        }

        pub fn code(&self) -> &str {
            &self.code
        }

        pub fn alt(&self) -> bool {
            self.alt
        }

        pub fn ctrl(&self) -> bool {
            self.ctrl
        }

        pub fn shift(&self) -> bool {
            self.shift
        }

        pub fn meta(&self) -> bool {
            self.meta
        }
    }

    builder! {
        #[builder_name = MouseBuilder]
        #[derive(Debug, Clone)]
        /// A mouse event.
        ///
        /// This event type is sent for these bindings:
        ///
        /// - `axm-mouseenter`
        /// - `axm-mouseover`
        /// - `axm-mouseleave`
        /// - `axm-mouseout`
        /// - `axm-mousemove`
        ///
        /// See [MDN] for more details about mouse events.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent
        pub struct Mouse {
            client_x: f64,
            client_y: f64,
            page_x: f64,
            page_y: f64,
            offset_x: f64,
            offset_y: f64,
            movement_x: f64,
            movement_y: f64,
            screen_x: f64,
            screen_y: f64,
        }
    }

    impl Mouse {
        /// Horizontal coordinate within the application's viewport at which the event occurred.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/clientX
        pub fn client_x(&self) -> f64 {
            self.client_x
        }

        /// Vertical coordinate within the application's viewport at which the event occurred.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/clientX
        pub fn client_y(&self) -> f64 {
            self.client_y
        }

        /// The horizontal coordinate of the mouse pointer relative to the whole document.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/pageX
        pub fn page_x(&self) -> f64 {
            self.page_x
        }

        /// The vertical coordinate of the mouse pointer relative to the whole document.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/pageY
        pub fn page_y(&self) -> f64 {
            self.page_y
        }

        /// The horizontal coordinate of the mouse pointer relative to the position of the padding
        /// edge of the target node.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/offsetX
        pub fn offset_x(&self) -> f64 {
            self.offset_x
        }

        /// The vertical coordinate of the mouse pointer relative to the position of the padding
        /// edge of the target node.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/offsetY
        pub fn offset_y(&self) -> f64 {
            self.offset_y
        }

        /// The horizontal coordinate of the mouse pointer relative to the position of the last
        /// `mousemove` event.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/movementX
        pub fn movement_x(&self) -> f64 {
            self.movement_x
        }

        /// The vertical coordinate of the mouse pointer relative to the position of the last
        /// `mousemove` event.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/movementY
        pub fn movement_y(&self) -> f64 {
            self.movement_y
        }

        /// The horizontal coordinate of the mouse pointer in global (screen) coordinates.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/screenX
        pub fn screen_x(&self) -> f64 {
            self.screen_x
        }

        /// The vertical coordinate of the mouse pointer in global (screen) coordinates.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/screenY
        pub fn screen_y(&self) -> f64 {
            self.screen_y
        }
    }

    builder! {
        #[builder_name = ScrollBuilder]
        #[derive(Debug, Clone)]
        /// A scroll event.
        ///
        /// This event type is sent for `axm-scroll` bindings.
        pub struct Scroll {
            scroll_x: f64,
            scroll_y: f64,
        }
    }

    impl Scroll {
        /// The number of pixels that the document is currently scrolled horizontally.
        ///
        /// See [MDN] for more details.
        ///
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Window/scrollX
        pub fn scroll_x(&self) -> f64 {
            self.scroll_x
        }

        /// The number of pixels that the document is currently scrolled vertically.
        ///
        /// See [MDN] for more details.
        ///
        /// [MDN]: https://developer.mozilla.org/en-US/docs/Web/API/Window/scrollY
        pub fn scroll_y(&self) -> f64 {
            self.scroll_y
        }
    }
}

pub use self::inner::{EventData, Form, FormSerializationError, Input, Key, Mouse, Scroll};

pub mod builders {
    //! Event data builder types.

    pub use super::inner::{FormBuilder, KeyBuilder, MouseBuilder, ScrollBuilder};
}
