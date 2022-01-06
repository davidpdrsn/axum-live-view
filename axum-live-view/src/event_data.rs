use crate::life_cycle::{self, MessageFromSocketData};
use serde::de::DeserializeOwned;
use std::fmt;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EventData {
    Form(Form),
    Input(Input),
    InputFocus(InputFocus),
    InputBlur(InputBlur),
    Key(Key),
    Mouse(Mouse),
    Scroll(Scroll),
}

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

    pub fn as_input_focus(&self) -> Option<&InputFocus> {
        if let Self::InputFocus(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_input_blur(&self) -> Option<&InputBlur> {
        if let Self::InputBlur(inner) = self {
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

impl From<MessageFromSocketData> for Option<EventData> {
    fn from(data: MessageFromSocketData) -> Self {
        match data {
            MessageFromSocketData::Click
            | MessageFromSocketData::WindowFocus
            | MessageFromSocketData::WindowBlur
            | MessageFromSocketData::None => None,
            MessageFromSocketData::FormSubmit { query } => Some(EventData::Form(Form { query })),
            MessageFromSocketData::FormChange { query } => Some(EventData::Form(Form { query })),
            MessageFromSocketData::InputFocus { value } => {
                Some(EventData::InputFocus(InputFocus { value }))
            }
            MessageFromSocketData::InputBlur { value } => {
                Some(EventData::InputBlur(InputBlur { value }))
            }
            MessageFromSocketData::InputChange { value } => {
                let value = match value {
                    life_cycle::InputValue::Bool(x) => Input::Bool(x),
                    life_cycle::InputValue::String(x) => Input::String(x),
                    life_cycle::InputValue::Strings(x) => Input::Strings(x),
                };
                Some(EventData::Input(value))
            }
            MessageFromSocketData::Key {
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
            MessageFromSocketData::Mouse {
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
            MessageFromSocketData::Scroll { scroll_x, scroll_y } => {
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
    pub fn deserialize<T>(&self) -> Result<T, DeserializeQueryError>
    where
        T: DeserializeOwned,
    {
        let query = percent_encoding::percent_decode_str(&self.query)
            .decode_utf8()
            .map_err(|err| DeserializeQueryError(DeserializeQueryErrorKind::Utf8Error(err)))?;

        let t = serde_qs::from_str(&*query)
            .map_err(|err| DeserializeQueryError(DeserializeQueryErrorKind::Deserialize(err)))?;

        Ok(t)
    }
}

#[derive(Debug)]
pub struct DeserializeQueryError(DeserializeQueryErrorKind);

#[derive(Debug)]
enum DeserializeQueryErrorKind {
    Utf8Error(std::str::Utf8Error),
    Deserialize(serde_qs::Error),
}

impl fmt::Display for DeserializeQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            DeserializeQueryErrorKind::Utf8Error(inner) => inner.fmt(f),
            DeserializeQueryErrorKind::Deserialize(inner) => inner.fmt(f),
        }
    }
}

impl std::error::Error for DeserializeQueryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            DeserializeQueryErrorKind::Utf8Error(inner) => Some(&*inner),
            DeserializeQueryErrorKind::Deserialize(inner) => Some(&*inner),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputFocus {
    value: String,
}

impl InputFocus {
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone)]
pub struct InputBlur {
    value: String,
}

impl InputBlur {
    pub fn value(&self) -> &str {
        &self.value
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

#[derive(Debug, Clone)]
pub struct Key {
    key: String,
    code: String,
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
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

#[derive(Debug, Clone)]
pub struct Scroll {
    scroll_x: f64,
    scroll_y: f64,
}

impl Scroll {
    pub fn scroll_x(&self) -> f64 {
        self.scroll_x
    }

    pub fn scroll_y(&self) -> f64 {
        self.scroll_y
    }
}

// #[cfg(test)]
// mod tests {
//     use std::collections::HashMap;
//     use serde::Deserialize;
//     use percent_encoding::{percent_decode, percent_decode_str};

//     #[allow(unused_imports)]
//     use super::*;

//     #[test]
//     fn test_decode_form_query() {
//         let query = "input=foo&textarea=bar&number=2&numbers%5B%5D=0&numbers%5B%5D=1&numbers%5B%5D=2&radio=2&checkboxes%5B3%5D=3&checkboxes%5B4%5D=4";
//         let query = percent_decode_str(query).decode_utf8().unwrap();
//         dbg!(&query);

//         let query: FormValues = serde_qs::from_str(&*query).unwrap();

//         dbg!(&query);
//     }

//     #[derive(Debug, Deserialize)]
//     struct FormValues {
//         input: String,
//         textarea: String,
//         number: String,
//         numbers: Vec<String>,
//         radio: Option<String>,
//         checkboxes: HashMap<String, bool>,
//     }
// }
