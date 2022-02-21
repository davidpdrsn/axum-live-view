//! Private API. Do _not_ use anything from this module!

#![allow(missing_docs)]

use super::*;

#[derive(Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum DynamicFragment<T> {
    String(String),
    #[serde(serialize_with = "serialize_msg")]
    Message(T),
    Html(Html<T>),
    Loop {
        #[serde(rename = "f")]
        fixed: &'static [&'static str],
        #[serde(rename = "b", skip_serializing_if = "BTreeMap::is_empty")]
        dynamic: IndexMap<IndexMap<DynamicFragment<T>>>,
    },
}

pub trait DynamicFragmentVecExt<T> {
    fn push_fragment(&mut self, part: impl Into<DynamicFragment<T>>);

    fn push_fragments(
        &mut self,
        fixed: &'static [&'static str],
        dynamic: Vec<Vec<DynamicFragment<T>>>,
    );

    fn push_message(&mut self, msg: T);
}

impl<T> DynamicFragmentVecExt<T> for Vec<DynamicFragment<T>> {
    #[inline]
    fn push_fragment(&mut self, part: impl Into<DynamicFragment<T>>) {
        self.push(part.into())
    }

    #[inline]
    fn push_fragments(
        &mut self,
        fixed: &'static [&'static str],
        dynamic: Vec<Vec<DynamicFragment<T>>>,
    ) {
        let dynamic = dynamic
            .into_iter()
            .enumerate()
            .map(|(idx, inner)| (idx, inner.into_iter().enumerate().collect()))
            .collect();
        self.push(DynamicFragment::Loop { fixed, dynamic })
    }

    #[inline]
    fn push_message(&mut self, msg: T) {
        self.push(DynamicFragment::Message(msg))
    }
}

#[derive(Debug, Clone)]
pub struct HtmlBuilder<T> {
    pub fixed: &'static [&'static str],
    pub dynamic: Vec<DynamicFragment<T>>,
}

impl<T> HtmlBuilder<T> {
    pub fn into_html(self) -> Html<T> {
        Html {
            fixed: self.fixed,
            dynamic: self.dynamic.into_iter().enumerate().collect(),
        }
    }
}
