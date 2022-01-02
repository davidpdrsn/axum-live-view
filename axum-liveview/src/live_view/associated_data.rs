use crate::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EventData {
    kind: Option<EventDataKind>,
}

impl EventData {
    pub(crate) fn new(kind: Option<ws::AssociatedDataKind>) -> Self {
        Self {
            kind: kind.map(Into::into),
        }
    }

    pub fn as_mouse(&self) -> Option<&MouseEventData> {
        if let Some(EventDataKind::Mouse(kind)) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }

    pub fn as_form(&self) -> Option<&FormEventData> {
        if let Some(EventDataKind::Form(kind)) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }

    pub fn as_key(&self) -> Option<&KeyEventData> {
        if let Some(EventDataKind::Key(kind)) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
enum EventDataKind {
    Form(FormEventData),
    Key(KeyEventData),
    Mouse(MouseEventData),
}

impl From<ws::AssociatedDataKind> for EventDataKind {
    fn from(kind: ws::AssociatedDataKind) -> Self {
        match kind {
            ws::AssociatedDataKind::Form(form_kind) => Self::Form(form_kind.into()),
            ws::AssociatedDataKind::Key(key_kind) => Self::Key(key_kind.into()),
            ws::AssociatedDataKind::Mouse(mouse_kind) => Self::Mouse(mouse_kind.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormEventData {
    String(String),
    Strings(Vec<String>),
    Bool(bool),
    Map(HashMap<String, FormEventData>),
}

impl From<ws::FormEventValue> for FormEventData {
    fn from(value: ws::FormEventValue) -> Self {
        match value {
            ws::FormEventValue::String(inner) => Self::String(inner),
            ws::FormEventValue::Strings(inner) => Self::Strings(inner),
            ws::FormEventValue::Bool(inner) => Self::Bool(inner),
            ws::FormEventValue::Map(inner) => {
                Self::Map(inner.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyEventData {
    key: String,
    code: String,
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
}

impl KeyEventData {
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

impl From<ws::KeyEventFields> for KeyEventData {
    fn from(
        ws::KeyEventFields {
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
        }: ws::KeyEventFields,
    ) -> Self {
        Self {
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MouseEventData {
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

impl From<ws::MouseEventFields> for MouseEventData {
    fn from(
        ws::MouseEventFields {
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
        }: ws::MouseEventFields,
    ) -> Self {
        Self {
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
        }
    }
}

impl MouseEventData {
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
