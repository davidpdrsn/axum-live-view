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
}

impl From<crate::ws::AssociatedDataKind> for AssociatedDataKind {
    fn from(kind: crate::ws::AssociatedDataKind) -> Self {
        match kind {
            crate::ws::AssociatedDataKind::Click => Self::Click,
            crate::ws::AssociatedDataKind::Form(form_kind) => Self::Form(form_kind.into()),
            crate::ws::AssociatedDataKind::Key(key_kind) => Self::Key(key_kind.into()),
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

impl From<crate::ws::KeyEventValue> for KeyEventValue {
    fn from(
        crate::ws::KeyEventValue {
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
        }: crate::ws::KeyEventValue,
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
