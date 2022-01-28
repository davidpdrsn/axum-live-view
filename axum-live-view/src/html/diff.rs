use super::{empty_slice, serialize_msg, DynamicFragment, Html, IndexMap};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(crate) struct HtmlDiff<'a, T> {
    #[serde(rename = "f", skip_serializing_if = "Option::is_none")]
    fixed: Option<&'static [&'static str]>,
    #[serde(rename = "d", skip_serializing_if = "BTreeMap::is_empty")]
    dynamic: IndexMap<Option<DynamicFragmentDiff<'a, T>>>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum DynamicFragmentDiff<'a, T> {
    String(&'a str),
    #[serde(serialize_with = "serialize_msg")]
    Message(&'a T),
    HtmlDiff(HtmlDiff<'a, T>),
    Loop {
        #[serde(rename = "f", skip_serializing_if = "empty_slice")]
        fixed: &'static [&'static str],
        #[serde(rename = "b", skip_serializing_if = "BTreeMap::is_empty")]
        dynamic: IndexMap<Option<IndexMap<DynamicFragmentDiff<'a, T>>>>,
    },
}

impl<'a, T> From<&'a Html<T>> for HtmlDiff<'a, T> {
    fn from(html: &'a Html<T>) -> Self {
        Self {
            fixed: Some(html.fixed),
            dynamic: html
                .dynamic
                .iter()
                .map(|(idx, dynamic)| (*idx, Some(dynamic.into())))
                .collect(),
        }
    }
}

impl<'a, T> From<&'a DynamicFragment<T>> for DynamicFragmentDiff<'a, T> {
    fn from(other: &'a DynamicFragment<T>) -> Self {
        match other {
            DynamicFragment::String(s) => Self::String(s),
            DynamicFragment::Message(msg) => Self::Message(msg),
            DynamicFragment::Html(html) => Self::HtmlDiff(html.into()),
            DynamicFragment::Loop { fixed, dynamic } => Self::Loop {
                fixed,
                dynamic: dynamic
                    .iter()
                    .map(|(idx, map)| {
                        (
                            *idx,
                            Some(
                                map.iter()
                                    .map(|(idx, dynamic)| (*idx, dynamic.into()))
                                    .collect::<IndexMap<_>>(),
                            ),
                        )
                    })
                    .collect(),
            },
        }
    }
}

impl<T> Html<T> {
    pub(crate) fn diff<'a>(&self, other: &'a Self) -> Option<HtmlDiff<'a, T>>
    where
        T: PartialEq + Serialize,
    {
        let dynamic = zip(self.dynamic.iter(), other.dynamic.iter())
            .filter_map(|pair| match pair {
                Zipped::Both((self_idx, self_value), (other_idx, other_value)) => {
                    debug_assert_eq!(self_idx, other_idx);
                    self_value
                        .diff(other_value)
                        .map(|diff| (*self_idx, Some(diff)))
                }
                Zipped::Left((other_idx, _)) => Some((*other_idx, None)),
                Zipped::Right((a, b)) => Some((*a, Some(b.into()))),
            })
            .collect::<BTreeMap<usize, Option<DynamicFragmentDiff<T>>>>();

        let new_fixed = (self.fixed != other.fixed).then(|| other.fixed);
        let new_dynamic = (!dynamic.is_empty()).then(|| dynamic);

        match (new_fixed, new_dynamic) {
            (None, None) => None,
            (Some(fixed), None) => Some(HtmlDiff {
                fixed: Some(fixed),
                dynamic: Default::default(),
            }),
            (None, Some(dynamic)) => Some(HtmlDiff {
                fixed: None,
                dynamic,
            }),
            (Some(fixed), Some(dynamic)) => Some(HtmlDiff {
                fixed: Some(fixed),
                dynamic,
            }),
        }
    }
}

impl<T> DynamicFragment<T> {
    pub(crate) fn diff<'a>(&self, other: &'a Self) -> Option<DynamicFragmentDiff<'a, T>>
    where
        T: PartialEq + Serialize,
    {
        match (self, other) {
            (Self::String(self_value), Self::String(other_value)) => {
                if self_value == other_value {
                    None
                } else {
                    Some(DynamicFragmentDiff::String(other_value))
                }
            }
            (Self::Message(from_self), Self::Message(from_other)) => {
                if from_self == from_other {
                    None
                } else {
                    Some(DynamicFragmentDiff::Message(from_other))
                }
            }
            (Self::Html(self_value), Self::Html(other_value)) => self_value
                .diff(other_value)
                .map(DynamicFragmentDiff::HtmlDiff),
            (
                Self::Loop {
                    fixed: self_fixed,
                    dynamic: self_dynamic,
                },
                Self::Loop {
                    fixed: dynamic_fixed,
                    dynamic: other_dynamic,
                },
            ) => {
                if self_fixed == dynamic_fixed && self_dynamic == other_dynamic {
                    return None;
                }

                let fixed = diff_fixed(self_fixed, dynamic_fixed);

                let dynamic = zip(self_dynamic.iter(), other_dynamic.iter())
                    .filter_map(|pair| match pair {
                        Zipped::Left((idx, _)) => Some((*idx, None)),
                        Zipped::Right((idx, from_other)) => Some((
                            *idx,
                            Some(
                                from_other
                                    .iter()
                                    .map(|(idx, c)| (*idx, c.into()))
                                    .collect::<IndexMap<_>>(),
                            ),
                        )),
                        Zipped::Both((from_idx, from_self), (other_idx, from_other)) => {
                            debug_assert_eq!(from_idx, other_idx);
                            let map = zip(from_self.iter(), from_other.iter())
                                .filter_map(|pair| match pair {
                                    Zipped::Left(_) => {
                                        unreachable!("unable to find a way to hit this yolo")
                                    }
                                    Zipped::Right(_) => {
                                        unreachable!("unable to find a way to hit this yolo")
                                    }
                                    Zipped::Both(
                                        (self_idx, self_value),
                                        (other_idx, other_value),
                                    ) => {
                                        debug_assert_eq!(self_idx, other_idx);
                                        self_value.diff(other_value).map(|diff| (*self_idx, diff))
                                    }
                                })
                                .collect::<BTreeMap<_, _>>();
                            if map.is_empty() {
                                None
                            } else {
                                Some((*from_idx, Some(map)))
                            }
                        }
                    })
                    .collect::<IndexMap<_>>();

                if fixed.is_empty() && dynamic.is_empty() {
                    None
                } else if dynamic
                    .iter()
                    .all(|(_, maybe_map)| maybe_map.as_ref().filter(|map| map.is_empty()).is_some())
                {
                    Some(DynamicFragmentDiff::Loop {
                        fixed,
                        dynamic: Default::default(),
                    })
                } else {
                    Some(DynamicFragmentDiff::Loop { fixed, dynamic })
                }
            }
            (_, other) => Some(other.into()),
        }
    }
}

fn diff_fixed(a: &'static [&'static str], b: &'static [&'static str]) -> &'static [&'static str] {
    if a.len() == b.len() && a == b {
        &[]
    } else {
        b
    }
}

#[derive(Debug)]
enum Zipped<A, B> {
    Left(A),
    Right(B),
    Both(A, B),
}

fn zip<T, K>(a: T, b: K) -> impl Iterator<Item = Zipped<T::Item, K::Item>>
where
    T: Iterator,
    T::Item: Clone,
    K: Iterator,
    K::Item: Clone,
{
    let a = a.map(Some).chain(std::iter::repeat(None));
    let b = b.map(Some).chain(std::iter::repeat(None));
    a.zip(b)
        .map_while(|(from_self, from_other)| match (from_self, from_other) {
            (Some(from_self), Some(from_other)) => Some(Zipped::Both(from_self, from_other)),
            (None, Some(from_other)) => Some(Zipped::Right(from_other)),
            (Some(from_self), None) => Some(Zipped::Left(from_self)),
            (None, None) => None,
        })
}
