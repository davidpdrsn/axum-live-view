use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize, Deserialize, Debug)]
enum Kind {
    Click,
    Form(FormEventValue),
    Key(KeyEventValue),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum FormEventValue {
    String(String),
    Strings(Vec<String>),
    Bool(bool),
    Map(HashMap<String, FormEventValue>),
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct KeyEventValue {
    #[serde(rename = "k")]
    pub(crate) key: String,
    #[serde(rename = "kc")]
    pub(crate) code: String,
    #[serde(rename = "a")]
    pub(crate) alt: bool,
    #[serde(rename = "c")]
    pub(crate) ctrl: bool,
    #[serde(rename = "s")]
    pub(crate) shift: bool,
    #[serde(rename = "m")]
    pub(crate) meta: bool,
}
