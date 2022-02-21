use axum::response::IntoResponse;
use serde::Serialize;
use std::{collections::BTreeMap, fmt};

pub(crate) use self::private::*;

mod diff;
pub(crate) mod private;
mod render;

#[cfg(test)]
mod tests;

type IndexMap<T> = BTreeMap<usize, T>;

/// An HTML template created with [`html!`].
///
/// See [`html!`] for more details.
///
/// [`html!`]: crate::html!
#[derive(Clone, Serialize, PartialEq)]
pub struct Html<T> {
    #[serde(rename = "f")]
    fixed: &'static [&'static str],
    #[serde(rename = "d", skip_serializing_if = "BTreeMap::is_empty")]
    dynamic: IndexMap<DynamicFragment<T>>,
}

impl<T> std::fmt::Debug for Html<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Html")
            .field("fixed", &self.fixed)
            .field("dynamic", &self.dynamic)
            .finish()
    }
}

fn empty_slice<T>(s: &[T]) -> bool {
    s.is_empty()
}

impl<T> std::fmt::Debug for DynamicFragment<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(inner) => f.debug_tuple("String").field(inner).finish(),
            Self::Message(_) => f.debug_tuple("Message").finish(),
            Self::Html(inner) => f.debug_tuple("Html").field(inner).finish(),
            Self::Loop { fixed, dynamic } => f
                .debug_struct("Loop")
                .field("fixed", &fixed)
                .field("dynamic", &dynamic)
                .finish(),
        }
    }
}

impl<S, T> From<S> for DynamicFragment<T>
where
    S: fmt::Display,
{
    fn from(x: S) -> Self {
        DynamicFragment::String(x.to_string())
    }
}

impl<T> From<Html<T>> for DynamicFragment<T> {
    fn from(x: Html<T>) -> Self {
        DynamicFragment::Html(x)
    }
}

fn serialize_msg<S, T>(msg: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Serialize,
{
    let encoded = serde_json::to_string(msg).unwrap();
    let encoded = std::borrow::Cow::from(percent_encoding::utf8_percent_encode(
        &encoded,
        ENCODE_FRAGMENT,
    ));
    serializer.serialize_str(&encoded)
}

impl<T> DynamicFragment<T> {
    fn map_with_mut<F, K>(self, f: &mut F) -> DynamicFragment<K>
    where
        F: FnMut(T) -> K,
    {
        match self {
            DynamicFragment::String(s) => DynamicFragment::String(s),
            DynamicFragment::Message(msg) => DynamicFragment::Message(f(msg)),
            DynamicFragment::Html(inner) => DynamicFragment::Html(inner.map_with_mut(f)),
            DynamicFragment::Loop { fixed, dynamic } => DynamicFragment::Loop {
                fixed,
                dynamic: dynamic
                    .into_iter()
                    .map(move |(idx, map)| {
                        (
                            idx,
                            map.into_iter()
                                .map(|(idx, value)| (idx, value.map_with_mut(f)))
                                .collect(),
                        )
                    })
                    .collect(),
            },
        }
    }
}

impl<T> Html<T> {
    /// Map the messages to a different type.
    pub fn map<F, K>(self, mut f: F) -> Html<K>
    where
        F: FnMut(T) -> K,
    {
        self.map_with_mut(&mut f)
    }

    fn map_with_mut<F, K>(self, f: &mut F) -> Html<K>
    where
        F: FnMut(T) -> K,
    {
        let dynamic = self
            .dynamic
            .into_iter()
            .map(move |(idx, d)| (idx, d.map_with_mut(f)))
            .collect();
        Html {
            fixed: self.fixed,
            dynamic,
        }
    }
}

impl<T> IntoResponse for Html<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.render()).into_response()
    }
}

const ENCODE_FRAGMENT: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`');
