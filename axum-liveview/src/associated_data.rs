use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct WithAssociatedData<T> {
    pub(crate) msg: T,
    pub(crate) data: AssociatedData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AssociatedData {
    kind: Kind,
}

impl AssociatedData {
    pub(crate) fn click() -> Self {
        Self { kind: Kind::Click }
    }

    pub(crate) fn form(value: FormEventValue) -> Self {
        Self {
            kind: Kind::Form(value),
        }
    }

    pub(crate) fn key(value: KeyEventValue) -> Self {
        Self {
            kind: Kind::Key(value),
        }
    }

    pub fn as_click(&self) -> Option<()> {
        if let Kind::Click = &self.kind {
            Some(())
        } else {
            None
        }
    }

    pub fn as_form(&self) -> Option<&FormEventValue> {
        if let Kind::Form(kind) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }

    pub fn as_key(&self) -> Option<&KeyEventValue> {
        if let Kind::Key(kind) = &self.kind {
            Some(kind)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Kind {
    Click,
    Form(FormEventValue),
    Key(KeyEventValue),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum FormEventValue {
    String(String),
    Strings(Vec<String>),
    Bool(bool),
    Map(HashMap<String, FormEventValue>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyEventValue {
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "kc")]
    pub code: String,
    #[serde(rename = "a")]
    pub alt: bool,
    #[serde(rename = "c")]
    pub ctrl: bool,
    #[serde(rename = "s")]
    pub shift: bool,
    #[serde(rename = "m")]
    pub meta: bool,
}
