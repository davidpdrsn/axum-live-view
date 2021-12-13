use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt};

#[derive(Default, Debug)]
pub struct Html {
    fixed: Vec<String>,
    dynamic: Vec<Dynamic>,
}

// TODO(david): document as private API
#[derive(Debug)]
pub enum Dynamic {
    String(String),
    Html(Html),
}

impl<S> From<S> for Dynamic
where
    S: fmt::Display,
{
    fn from(inner: S) -> Self {
        Self::String(inner.to_string())
    }
}

impl From<Html> for Dynamic {
    fn from(inner: Html) -> Self {
        Self::Html(inner)
    }
}

impl Html {
    // TODO(david): document as private API
    pub fn fixed(&mut self, part: &str) {
        self.fixed.push(part.to_owned());
    }

    // TODO(david): document as private API
    pub fn dynamic(&mut self, part: impl Into<Dynamic>) {
        self.dynamic.push(part.into());
    }

    pub(crate) fn diff(&self, other: &Self) -> Diff {
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
                (Dynamic::Html(a), Dynamic::Html(b)) => Some((idx, json!(a.diff(b)))),
                (_, Dynamic::Html(inner)) => Some((idx, json!(inner.serialize()))),
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

        Diff(out)
    }

    pub(crate) fn serialize(&self) -> Serialized {
        let out = self
            .dynamic
            .iter()
            .enumerate()
            .map(|(idx, value)| {
                (
                    idx.to_string(),
                    match value {
                        Dynamic::String(s) => json!(s),
                        Dynamic::Html(inner) => json!(inner.serialize()),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut out = serde_json::to_value(&out).unwrap();

        out.as_object_mut()
            .unwrap()
            .insert("s".to_owned(), serde_json::to_value(&self.fixed).unwrap());

        Serialized(out)
    }

    pub(crate) fn render(self) -> String {
        use itertools::Itertools;

        self.fixed
            .into_iter()
            .interleave(self.dynamic.into_iter().map(|dynamic| match dynamic {
                Dynamic::String(s) => s,
                Dynamic::Html(inner) => inner.render(),
            }))
            .collect::<String>()
    }
}

impl IntoResponse for Html {
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.render()).into_response()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct Serialized(Value);

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct Diff(Value);

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate as axum_liveview;
    use crate::html;

    #[test]
    #[allow(unused_variables)]
    fn these_should_compile() {
        let view = html! {
            <div></div>
        };

        let view = html! {
            <!DOCTYPE html>
        };

        let view = html! {
            "hi there"
        };

        let view = html! {
            <div>"foo"</div>
        };

        let count = 1;
        html! {
            <div>{ count }</div>
        };

        let count = 1;
        html! {
            <div>"foo"</div>
            <div>{ count }</div>
        };

        let view = html! {
            <div>
                <p>"some paragraph..."</p>
            </div>
        };

        let count = 3;
        let view = html! {
            <div>
                <ul>
                    <li>{ count }</li>
                    <li>"2"</li>
                    <li>"3"</li>
                </ul>
            </div>
        };

        html! {
            <div>
                <ul>
                    {
                        html! {
                            <li>"1"</li>
                            <li>"2"</li>
                            <li>"3"</li>
                        }
                    }
                </ul>
            </div>
        };

        html! {
            <div class="col-md">"foo"</div>
        };

        html! {
            <div class="col-md" id="the-thing">"foo"</div>
        };

        html! {
            <div on-click="do thing">"foo"</div>
        };

        let size = 8;
        html! {
            <div class={ format!("col-{}", size) }>"foo"</div>
        };

        let view = html! {
            <div
                class="foo"
                class="foo"
                class={
                    let foo = 123;
                    format!("col-{}", foo)
                }
                class="foo"
            >"foo"</div>
        };

        html! {
            <button disabled>"foo"</button>
        };

        html! {
            <img src="foo.png" />
        };

        let view = html! {
            <div>
                if true {
                    <p>"some paragraph..."</p>
                }
            </div>
        };

        let something = || true;
        let view = html! {
            <div>
                if something() {
                    <p>"something"</p>
                } else {
                    <p>"something else"</p>
                }
            </div>
        };

        let name = Some("bob");
        let view = html! {
            <div>
                if let Some(name) = name {
                    <p>{ format!("Hi {}", name) }</p>
                } else {
                    <p>"Missing name..."</p>
                }
            </div>
        };

        let names = ["alice", "bob", "cindy"];
        let view = html! {
            <ul>
                for name in names {
                    <li>{ name }</li>
                }
            </ul>
        };

        let name = Some("bob");
        let view = html! {
            <div>
                match name {
                    Some(name) => <p>{ format!("Hi {}", name) }</p>,
                    None => <p>"Missing name..."</p>,
                }
            </div>
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&view.serialize()).unwrap()
        );
    }
}
