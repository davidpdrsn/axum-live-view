//! Data associated with events from the client such as click or form events.

mod inner {
    use crate::life_cycle::{self, EventMessageFromSocketData};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::{self, Display};

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
        /// A file upload event.
        /// 
        /// See [`FileEvent`] for more details.
        FileEvent(FileEvent)
    }

    impl_from!(EventData::Form);
    impl_from!(EventData::Input);
    impl_from!(EventData::Key);
    impl_from!(EventData::Mouse);
    impl_from!(EventData::Scroll);
    impl_from!(EventData::FileEvent);

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

        /// Get the inner [`FileEvent`] if any.
        pub fn as_file_event(&self) -> Option<&FileEvent> {
            if let Self::FileEvent(inner) = self {
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
                },
                EventMessageFromSocketData::File { 
                    length_computable, 
                    loaded, 
                    total, 
                    file_last_modified, 
                    file_name, 
                    file_webkit_relative_path, 
                    file_size, 
                    file_type, 
                    ready_state, 
                    dom_exception, 
                    result
                } => Some(EventData::FileEvent(FileEvent {
                    length_computable,
                    loaded,
                    total,
                    file_last_modified,
                    file_webkit_relative_path,
                    file_name,
                    file_size,
                    file_type,
                    ready_state,
                    dom_exception,
                    result
                }))
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

    builder! {

        #[builder_name = FileEventBuilder]
        #[derive(Debug, Clone)]
        /// A file event.
        /// 
        /// This event type is sent for `axm-file-*` bindings
        /// no elements that have `axm-input` and have `type="file"`.
        pub struct FileEvent {
            length_computable: bool,
            loaded: u64,
            total: u64,
            file_last_modified: u64,
            file_name: String,
            file_webkit_relative_path: String,
            file_size: u64,
            file_type: String,
            ready_state: u8,
            dom_exception: serde_json::Value,
            result: String,
        }
    }

    impl Display for FileEvent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "length_computable={}", self.length_computable);
            writeln!(f, "total={}", self.total);
            writeln!(f, "file_last_modified={}", self.file_last_modified);
            writeln!(f, "file_name={}", self.file_name);
            writeln!(f, "file_webkit_relative_path={}", self.file_webkit_relative_path);
            writeln!(f, "file_size={}", self.file_size);
            writeln!(f, "file_type={}", self.file_type);
            writeln!(f, "ready_state={}", self.ready_state);
            writeln!(f, "dom_exception={}", self.dom_exception)
        }
    }

    impl FileEvent {
        /// A boolean flag indicating if the total work to be done, 
        /// and the amount of work already done, 
        /// by the underlying process is calculable. 
        /// In other words, it tells if the progress is measurable or not.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/ProgressEvent/lengthComputable
        pub fn length_computable(&self) -> bool {
            self.length_computable
        }

        /// A 64-bit unsigned integer value indicating the amount of work already 
        /// performed by the underlying process. The ratio of work done can be 
        /// calculated by dividing total by the value of this property. 
        /// 
        /// When downloading a resource using HTTP, 
        /// this only counts the body of the HTTP message, 
        /// and doesn't include headers and other overhead.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/ProgressEvent/loaded
        pub fn loaded(&self) -> u64 {
            self.loaded
        }

        /// A 64-bit unsigned integer representing the total amount of work that 
        /// the underlying process is in the progress of performing. 
        /// 
        /// When downloading a resource using HTTP, 
        /// this is the Content-Length (the size of the body of the message), 
        /// and doesn't include the headers and other overhead.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/ProgressEvent/total
        pub fn total(&self) -> u64 {
            self.total
        }

        /// The last modified time of the file, 
        /// in millisecond since the UNIX epoch (January 1st, 1970 at Midnight)
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/File/lastModified
        pub fn file_last_modified(&self) -> u64 {
            self.file_last_modified
        }

        /// The name of the file referenced by the File object.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/File/name
        pub fn file_name(&self) -> &str {
            &self.file_name
        }

        /// The path the URL of the File is relative to.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/File/webkitRelativePath
        pub fn file_webkit_relative_path(&self) -> &str {
            &self.file_webkit_relative_path
        }

        /// The size of the file in bytes.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Blob/size
        pub fn file_size(&self) -> u64 {
            self.file_size
        }

        /// The [MIME] type of the file.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mime]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/File/type
        pub fn file_type(&self) -> &str {
            &self.file_type
        }

        /// A number indicating the state of the FileReader. This is one of the following:
        /// 
        /// - `0`: No data has been loaded yet.
        /// - `1`: Data is currently being loaded.
        /// - `2`: The entire read request has been completed.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/FileReader/readyState
        pub fn ready_state(&self) -> u8 {
            self.ready_state
        }
        /// The file's contents. 
        /// This property is only valid after the read operation is complete, 
        /// and the format of the data is a utf-8 encoded string that can be decoded for
        /// the actual raw bytes.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/FileReader/result
        pub fn result(&self) -> &str {
            &self.result
        }

        /// A DOMException representing the error that occurred while reading the file.
        /// 
        /// See [MDN] for more details.
        /// 
        /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/FileReader/error
        pub fn dom_exception(&self) -> &serde_json::Value {
            &self.dom_exception
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
