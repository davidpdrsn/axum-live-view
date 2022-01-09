use super::{DynamicFragment, Html};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, PartialEq)]
pub(crate) struct HtmlDiff<'a, T> {
    #[serde(rename = "f", skip_serializing_if = "super::empty_slice")]
    fixed: &'static [&'static str],
    #[serde(rename = "d", skip_serializing_if = "BTreeMap::is_empty")]
    dynamic: BTreeMap<usize, Option<DynamicFragmentDiff<'a, T>>>,
}

impl<'a, T> From<&'a Html<T>> for HtmlDiff<'a, T> {
    fn from(html: &'a Html<T>) -> Self {
        Self {
            fixed: html.fixed,
            dynamic: html
                .dynamic
                .iter()
                .map(|(idx, dynamic)| (*idx, Some(dynamic.into())))
                .collect(),
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum DynamicFragmentDiff<'a, T> {
    String(&'a str),
    #[serde(serialize_with = "super::serialize_dynamic_fragment_message")]
    Message(&'a T),
    HtmlDiff(HtmlDiff<'a, T>),
    DedupLoop {
        #[serde(rename = "f", skip_serializing_if = "super::empty_slice")]
        fixed: &'static [&'static str],
        #[serde(rename = "b", skip_serializing_if = "Vec::is_empty")]
        dynamic: Vec<Option<BTreeMap<usize, DynamicFragmentDiff<'a, T>>>>,
    },
}

impl<'a, T> From<&'a DynamicFragment<T>> for DynamicFragmentDiff<'a, T> {
    fn from(other: &'a DynamicFragment<T>) -> Self {
        match other {
            DynamicFragment::String(s) => Self::String(s),
            DynamicFragment::Message(msg) => Self::Message(msg),
            DynamicFragment::Html(html) => Self::HtmlDiff(html.into()),
            DynamicFragment::DedupLoop { fixed, dynamic } => Self::DedupLoop {
                fixed,
                dynamic: dynamic
                    .iter()
                    .map(|map| {
                        Some(
                            map.iter()
                                .map(|(idx, dynamic)| (*idx, dynamic.into()))
                                .collect(),
                        )
                    })
                    .collect(),
            },
        }
    }
}

impl<T> Html<T> {
    #[allow(warnings)]
    pub(crate) fn diff<'a>(&'a self, other: &'a Self) -> Option<HtmlDiff<'a, T>>
    where
        T: PartialEq + Serialize,
    {
        let fixed = diff_fixed(self.fixed, other.fixed);

        let dynamic = zip(self.dynamic.iter(), other.dynamic.iter())
            .filter_map(|pair| match pair {
                Zipped::Both((self_idx, self_value), (other_idx, other_value)) => {
                    debug_assert_eq!(self_idx, other_idx);
                    if let Some(diff) = self_value.diff(other_value) {
                        Some((*self_idx, Some(diff)))
                    } else {
                        None
                    }
                }
                Zipped::Left((other_idx, _)) => Some((*other_idx, None)),
                Zipped::Right(from_self) => todo!(),
            })
            .collect::<BTreeMap<usize, Option<DynamicFragmentDiff<T>>>>();

        if !fixed.is_empty() || !dynamic.is_empty() {
            Some(HtmlDiff { fixed, dynamic })
        } else {
            None
        }
    }
}

impl<T> DynamicFragment<T> {
    #[allow(warnings)]
    pub(crate) fn diff<'a>(&'a self, other: &'a Self) -> Option<DynamicFragmentDiff<'a, T>>
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
            (Self::Message { .. }, Self::Message { .. }) => {
                todo!("diff message")
            }
            (Self::Html(self_value), Self::Html(other_value)) => self_value
                .diff(other_value)
                .map(DynamicFragmentDiff::HtmlDiff),
            (
                Self::DedupLoop {
                    fixed: self_fixed,
                    dynamic: self_dynamic,
                },
                Self::DedupLoop {
                    fixed: dynamic_fixed,
                    dynamic: other_dynamic,
                },
            ) => {
                if self_fixed == dynamic_fixed && self_dynamic == other_dynamic {
                    return None;
                }

                let fixed = diff_fixed(self_fixed, dynamic_fixed);

                let dynamic = zip(self_dynamic.iter(), other_dynamic.iter())
                    .map(|pair| match pair {
                        Zipped::Left(_) => None,
                        Zipped::Right(from_other) => {
                            Some(from_other.iter().map(|(idx, c)| (*idx, c.into())).collect())
                        }
                        Zipped::Both(from_self, from_other) => {
                            let map = zip(from_self.iter(), from_other.iter())
                                .filter_map(|pair| match pair {
                                    Zipped::Left(_) => todo!("Zipped::Left inner"),
                                    Zipped::Right(_) => todo!("Zipped::Right inner"),
                                    Zipped::Both(
                                        (self_idx, self_value),
                                        (other_idx, other_value),
                                    ) => {
                                        debug_assert_eq!(self_idx, other_idx);
                                        self_value.diff(other_value).map(|diff| (*self_idx, diff))
                                    }
                                })
                                .collect::<BTreeMap<_, _>>();

                            Some(map)
                        }
                    })
                    .collect::<Vec<_>>();

                if fixed.is_empty() && dynamic.is_empty() {
                    None
                } else if dynamic
                    .iter()
                    .all(|maybe_map| maybe_map.as_ref().filter(|map| map.is_empty()).is_some())
                {
                    Some(DynamicFragmentDiff::DedupLoop {
                        fixed,
                        dynamic: Vec::new(),
                    })
                } else {
                    Some(DynamicFragmentDiff::DedupLoop { fixed, dynamic })
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
