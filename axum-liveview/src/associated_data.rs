use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub struct AssociatedData {
    kind: AssociatedDataKind,
}

impl AssociatedData {
    pub(crate) fn new(kind: crate::ws::AssociatedDataKind) -> Self {
        Self { kind: kind.into() }
    }

    pub fn as_click(&self) -> Option<()> {
        if let AssociatedDataKind::Click = &self.kind {
            Some(())
        } else {
            None
        }
    }

    pub fn as_window_focus_blur(&self) -> Option<()> {
        if let AssociatedDataKind::WindowFocusBlur = &self.kind {
            Some(())
        } else {
            None
        }
    }

    pub fn as_mouse(&self) -> Option<&MouseEventValue> {
        if let AssociatedDataKind::Mouse(kind) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }

    pub fn as_form(&self) -> Option<&FormEventValue> {
        if let AssociatedDataKind::Form(kind) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }

    pub fn as_key(&self) -> Option<&KeyEventValue> {
        if let AssociatedDataKind::Key(kind) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }
}

#[derive(Debug)]
enum AssociatedDataKind {
    Click,
    Form(FormEventValue),
    Key(KeyEventValue),
    WindowFocusBlur,
    Mouse(MouseEventValue),
}

impl From<crate::ws::AssociatedDataKind> for AssociatedDataKind {
    fn from(kind: crate::ws::AssociatedDataKind) -> Self {
        match kind {
            crate::ws::AssociatedDataKind::Click => Self::Click,
            crate::ws::AssociatedDataKind::Form(form_kind) => Self::Form(form_kind.into()),
            crate::ws::AssociatedDataKind::Key(key_kind) => Self::Key(key_kind.into()),
            crate::ws::AssociatedDataKind::WindowFocusBlur => Self::WindowFocusBlur,
            crate::ws::AssociatedDataKind::Mouse(mouse_kind) => Self::Mouse(mouse_kind.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FormEventValue {
    String(String),
    Strings(Vec<String>),
    Bool(bool),
    Map(HashMap<String, FormEventValue>),
}

impl From<crate::ws::FormEventValue> for FormEventValue {
    fn from(value: crate::ws::FormEventValue) -> Self {
        match value {
            crate::ws::FormEventValue::String(inner) => Self::String(inner),
            crate::ws::FormEventValue::Strings(inner) => Self::Strings(inner),
            crate::ws::FormEventValue::Bool(inner) => Self::Bool(inner),
            crate::ws::FormEventValue::Map(inner) => {
                Self::Map(inner.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

#[derive(Debug)]
pub struct KeyEventValue {
    key: String,
    code: String,
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
}

impl KeyEventValue {
    pub fn key(&self) -> &str {
        self.key.as_ref()
    }

    pub fn code(&self) -> &str {
        self.code.as_ref()
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

impl From<crate::ws::KeyEventFields> for KeyEventValue {
    fn from(
        crate::ws::KeyEventFields {
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
        }: crate::ws::KeyEventFields,
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

#[derive(Debug)]
pub struct MouseEventValue {
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

impl From<crate::ws::MouseEventFields> for MouseEventValue {
    fn from(
        crate::ws::MouseEventFields {
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
        }: crate::ws::MouseEventFields,
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

impl MouseEventValue {
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
