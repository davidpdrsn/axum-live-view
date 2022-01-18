mod inner {
    use crate::life_cycle::{self, EventMessageFromSocketData};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt;

    #[derive(Debug, Clone)]
    #[non_exhaustive]
    pub enum EventData {
        Form(Form),
        Input(Input),
        Key(Key),
        Mouse(Mouse),
        Scroll(Scroll),
    }

    impl_from!(EventData::Form);
    impl_from!(EventData::Input);
    impl_from!(EventData::Key);
    impl_from!(EventData::Mouse);
    impl_from!(EventData::Scroll);

    impl EventData {
        pub fn as_form(&self) -> Option<&Form> {
            if let Self::Form(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        pub fn as_input(&self) -> Option<&Input> {
            if let Self::Input(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        pub fn as_key(&self) -> Option<&Key> {
            if let Self::Key(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

        pub fn as_mouse(&self) -> Option<&Mouse> {
            if let Self::Mouse(inner) = self {
                Some(inner)
            } else {
                None
            }
        }

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

    #[derive(Debug, Clone)]
    pub struct Form {
        query: String,
    }

    impl Form {
        pub fn builder() -> FormBuilder {
            FormBuilder::default()
        }

        pub fn deserialize<T>(&self) -> Result<T, QuerySerializationError>
        where
            T: DeserializeOwned,
        {
            let query = percent_encoding::percent_decode_str(&self.query)
                .decode_utf8()
                .map_err(|err| {
                    QuerySerializationError(QuerySerializationErrorKind::Utf8Error(err))
                })?;

            let t = serde_qs::from_str(&*query).map_err(|err| {
                QuerySerializationError(QuerySerializationErrorKind::Serialization(err))
            })?;

            Ok(t)
        }
    }

    #[derive(Debug)]
    pub struct QuerySerializationError(QuerySerializationErrorKind);

    #[derive(Debug)]
    enum QuerySerializationErrorKind {
        Utf8Error(std::str::Utf8Error),
        Serialization(serde_qs::Error),
    }

    impl fmt::Display for QuerySerializationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.0 {
                QuerySerializationErrorKind::Utf8Error(inner) => inner.fmt(f),
                QuerySerializationErrorKind::Serialization(inner) => inner.fmt(f),
            }
        }
    }

    impl std::error::Error for QuerySerializationError {
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

        pub fn serialize<T>(mut self, value: &T) -> Result<Self, QuerySerializationError>
        where
            T: Serialize,
        {
            let query = serde_qs::to_string(value).map_err(|err| {
                QuerySerializationError(QuerySerializationErrorKind::Serialization(err))
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
        pub fn client_x(&self) -> f64 {
            self.client_x
        }

        pub fn client_y(&self) -> f64 {
            self.client_y
        }

        pub fn page_x(&self) -> f64 {
            self.page_x
        }

        pub fn page_y(&self) -> f64 {
            self.page_y
        }

        pub fn offset_x(&self) -> f64 {
            self.offset_x
        }

        pub fn offset_y(&self) -> f64 {
            self.offset_y
        }

        pub fn movement_x(&self) -> f64 {
            self.movement_x
        }

        pub fn movement_y(&self) -> f64 {
            self.movement_y
        }

        pub fn screen_x(&self) -> f64 {
            self.screen_x
        }

        pub fn screen_y(&self) -> f64 {
            self.screen_y
        }
    }

    builder! {
        #[builder_name = ScrollBuilder]
        #[derive(Debug, Clone)]
        pub struct Scroll {
            scroll_x: f64,
            scroll_y: f64,
        }
    }

    impl Scroll {
        pub fn scroll_x(&self) -> f64 {
            self.scroll_x
        }

        pub fn scroll_y(&self) -> f64 {
            self.scroll_y
        }
    }
}

pub use self::inner::{EventData, Form, Input, Key, Mouse, QuerySerializationError, Scroll};

pub mod builders {
    pub use super::inner::{FormBuilder, KeyBuilder, MouseBuilder, ScrollBuilder};
}
