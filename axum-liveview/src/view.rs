#![allow(warnings)]

use serde_json::{json, Value};
use std::{collections::HashMap, fmt};

#[derive(Default, Debug)]
pub struct View {
    fixed: Vec<String>,
    dynamic: Vec<Dynamic>,
}

#[derive(Debug)]
pub enum Dynamic {
    String(String),
    View(View),
}

impl<S> From<S> for Dynamic
where
    S: fmt::Display,
{
    fn from(inner: S) -> Self {
        Self::String(inner.to_string())
    }
}

impl From<View> for Dynamic {
    fn from(inner: View) -> Self {
        Self::View(inner)
    }
}

impl View {
    pub fn fixed(&mut self, part: &str) {
        self.fixed.push(part.to_owned());
    }

    pub fn dynamic(&mut self, part: impl Into<Dynamic>) {
        self.dynamic.push(part.into());
    }

    fn diff(&self, other: &Self) -> Value {
        let out = self
            .dynamic
            .iter()
            .zip(&other.dynamic)
            .enumerate()
            .filter_map(|(idx, (a, b))| match (a, b) {
                (Dynamic::String(a), Dynamic::String(b)) => {
                    if a == b {
                        None
                    } else {
                        Some((idx, json!(b)))
                    }
                }
                (Dynamic::View(a), Dynamic::View(b)) => Some((idx, a.diff(b))),
                (_, Dynamic::View(inner)) => Some((idx, inner.serialize())),
                (_, Dynamic::String(inner)) => Some((idx, json!(inner))),
            })
            .collect::<HashMap<_, _>>();
        let mut out = json!(out);

        if self.fixed.len() != other.fixed.len()
            || self.fixed.iter().zip(&other.fixed).any(|(a, b)| a != b)
        {
            out.as_object_mut()
                .unwrap()
                .insert("s".to_owned(), serde_json::to_value(&other.fixed).unwrap());
        }

        out
    }

    fn serialize(&self) -> Value {
        let out = self
            .dynamic
            .iter()
            .enumerate()
            .map(|(idx, value)| {
                (
                    idx.to_string(),
                    match value {
                        Dynamic::String(s) => json!(s),
                        Dynamic::View(inner) => inner.serialize(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut out = serde_json::to_value(&out).unwrap();

        out.as_object_mut()
            .unwrap()
            .insert("s".to_owned(), serde_json::to_value(&self.fixed).unwrap());

        out
    }
}
