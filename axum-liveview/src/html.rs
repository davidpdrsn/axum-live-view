use __private::*;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct Html {
    fixed: Vec<String>,
    dynamic: Vec<Dynamic>,
}

#[doc(hidden)]
pub mod __private {
    /// Private API. Do _not_ use anything from this module!
    use super::*;

    pub fn html() -> Html {
        Html {
            fixed: Default::default(),
            dynamic: Default::default(),
        }
    }

    pub fn fixed(html: &mut Html, part: &str) {
        html.fixed.push(part.to_owned());
    }

    pub fn dynamic(html: &mut Html, part: impl Into<Dynamic>) {
        html.dynamic.push(part.into());
    }

    #[derive(Debug)]
    pub enum Dynamic {
        String(String),
        Html(Html),
    }

    impl Dynamic {
        pub(super) fn serialize(&self) -> Value {
            match self {
                Dynamic::String(s) => json!(s),
                Dynamic::Html(inner) => json!(inner.serialize()),
            }
        }
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
}

impl Html {
    pub(crate) fn diff(&self, other: &Self) -> DiffResult {
        let mut out = self
            .dynamic
            .iter()
            .map(Some)
            .chain(std::iter::repeat(None))
            .zip(
                other
                    .dynamic
                    .iter()
                    .map(Some)
                    .chain(std::iter::repeat(None)),
            )
            .take_while(|(a, b)| a.is_some() || b.is_some())
            .enumerate()
            .filter_map(|(idx, (prev, current))| {
                let value = match (prev, current) {
                    (Some(prev), Some(current)) => match (prev, current) {
                        (Dynamic::String(a), Dynamic::String(b)) => {
                            if a == b {
                                None
                            } else {
                                Some(json!(b))
                            }
                        }
                        #[allow(clippy::needless_borrow)] // false positive
                        (Dynamic::Html(a), Dynamic::Html(b)) => match a.diff(&b) {
                            DiffResult::Changed(diff) => Some(json!(diff)),
                            DiffResult::Unchanged => None,
                        },
                        (_, Dynamic::Html(inner)) => Some(json!(inner.serialize())),
                        (_, Dynamic::String(inner)) => Some(json!(inner)),
                    },
                    (None, Some(current)) => Some(current.serialize()),
                    (Some(_prev), None) => {
                        // a placeholder has been removed
                        // we have to somehow be able to tell the difference between
                        // a placeholder not having changed and removed
                        Some(json!(null))
                    }
                    (None, None) => unreachable!("double nones are filtered out earlier"),
                };

                value.map(|value| {
                    // can we avoid allocating strings here?
                    (idx.to_string(), value)
                })
            })
            .collect::<HashMap<_, _>>();

        if self.fixed.len() != other.fixed.len()
            || self.fixed.iter().zip(&other.fixed).any(|(a, b)| a != b)
        {
            out.insert(
                FIXED.to_owned(),
                serde_json::to_value(&other.fixed).unwrap(),
            );
        }

        if out.is_empty() {
            DiffResult::Unchanged
        } else {
            DiffResult::Changed(Diff(json!(out)))
        }
    }

    pub(crate) fn serialize(&self) -> Serialized {
        let out = self
            .dynamic
            .iter()
            .enumerate()
            .map(|(idx, value)| (idx.to_string(), value.serialize()))
            .collect::<HashMap<_, _>>();

        let mut out = serde_json::to_value(&out).unwrap();

        out.as_object_mut()
            .unwrap()
            .insert(FIXED.to_owned(), serde_json::to_value(&self.fixed).unwrap());

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

const FIXED: &str = "f";

impl IntoResponse for Html {
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.render()).into_response()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct Serialized(Value);

#[derive(Debug)]
pub(crate) enum DiffResult {
    Changed(Diff),
    Unchanged,
}

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
    fn basic() {
        let view = html! { <div></div> };
        assert_eq!(view.render(), "<div></div>");
    }

    #[test]
    fn doctype() {
        let view = html! { <!DOCTYPE html> };
        assert_eq!(view.render(), "<!DOCTYPE html>");
    }

    #[test]
    fn text() {
        let view = html! { "foo" };
        assert_eq!(view.render(), "foo");
    }

    #[test]
    fn text_inside_tag() {
        let view = html! { <div>"foo"</div> };
        assert_eq!(view.render(), "<div>foo</div>");
    }

    #[test]
    fn interpolate() {
        let count = 1;
        let view = html! { <div>{ count }</div> };
        assert_eq!(view.render(), "<div>1</div>");
    }

    #[test]
    fn fixed_next_to_dynamic() {
        let count = 1;
        let view = html! {
            <div>"foo"</div>
            <div>{ count }</div>
        };
        assert_eq!(view.render(), "<div>foo</div><div>1</div>");
    }

    #[test]
    fn nested_tags() {
        let view = html! {
            <div>
                <p>"foo"</p>
            </div>
        };
        assert_eq!(view.render(), "<div><p>foo</p></div>");
    }

    #[test]
    fn deeply_nested() {
        let count = 1;
        let view = html! {
            <div>
                <ul>
                    <li>{ count }</li>
                    <li>"2"</li>
                    <li>"3"</li>
                </ul>
            </div>
        };
        assert_eq!(
            view.render(),
            "<div><ul><li>1</li><li>2</li><li>3</li></ul></div>"
        );
    }

    #[test]
    fn nested_with_more_html_calls() {
        let view = html! {
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
        assert_eq!(
            view.render(),
            "<div><ul><li>1</li><li>2</li><li>3</li></ul></div>"
        );
    }

    #[test]
    fn attribute() {
        let view = html! {
            <div class="col-md">"foo"</div>
        };
        assert_eq!(view.render(), "<div class=\"col-md\">foo</div>");
    }

    #[test]
    fn multiple_attributes() {
        let view = html! {
            <div class="col-md" id="the-thing">"foo"</div>
        };
        assert_eq!(
            view.render(),
            "<div class=\"col-md\" id=\"the-thing\">foo</div>"
        );
    }

    #[test]
    fn attribute_with_dash() {
        let view = html! {
            <div on-click="do thing">"foo"</div>
        };
        assert_eq!(view.render(), "<div on-click=\"do thing\">foo</div>");
    }

    #[test]
    fn interpolate_class() {
        let size = 8;
        let view = html! {
            <div class={ format!("col-{}", size) }>"foo"</div>
        };
        assert_eq!(view.render(), "<div class=\"col-8\">foo</div>");
    }

    #[test]
    fn empty_attribute() {
        let view = html! {
            <button disabled>"foo"</button>
        };
        assert_eq!(view.render(), "<button disabled>foo</button>");
    }

    #[test]
    fn empty_tag() {
        let view = html! {
            <img src="foo.png" />
        };
        assert_eq!(view.render(), "<img src=\"foo.png\">");
    }

    #[test]
    fn conditional() {
        let view = html! {
            <div>
                if true {
                    <p>"some paragraph..."</p>
                }
            </div>
        };
        assert_eq!(view.render(), "<div><p>some paragraph...</p></div>");
    }

    #[test]
    fn conditional_else() {
        let view = html! {
            <div>
                if true {
                    <p>"some paragraph..."</p>
                } else {
                    <p>"wat"</p>
                }
            </div>
        };
        assert_eq!(view.render(), "<div><p>some paragraph...</p></div>");
    }

    #[test]
    fn conditional_else_if() {
        let view = html! {
            <div>
                if true {
                    <p>"some paragraph..."</p>
                } else if false {
                    <p>"wat"</p>
                } else {
                    <p>"wat"</p>
                }
            </div>
        };
        assert_eq!(view.render(), "<div><p>some paragraph...</p></div>");
    }

    #[test]
    fn if_let() {
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
        assert_eq!(view.render(), "<div><p>Hi bob</p></div>");
    }

    #[test]
    fn for_loop() {
        let names = ["alice", "bob", "cindy"];
        let view = html! {
            <ul>
                for name in names {
                    <li>{ name }</li>
                }
            </ul>
        };
        assert_eq!(
            view.render(),
            "<ul><li>alice</li><li>bob</li><li>cindy</li></ul>"
        );
    }

    #[test]
    fn match_() {
        let name = Some("bob");
        let view = html! {
            <div>
                match name {
                    Some(name) => <p>{ format!("Hi {}", name) }</p>,
                    None => <p>"Missing name..."</p>,
                }
            </div>
        };
        assert_eq!(view.render(), "<div><p>Hi bob</p></div>");
    }

    #[test]
    fn match_guard() {
        let count = Some(10);
        let view = html! {
            <div>
                match count {
                    Some(count) if count == 0 => <p>"its zero!"</p>,
                    Some(count) => <p>{ count }</p>,
                    None => <p>"Missing count..."</p>,
                }
            </div>
        };
        assert_eq!(view.render(), "<div><p>10</p></div>");
    }

    #[test]
    fn keyword_attribute() {
        let view = html! {
            <input type="text" />
        };
        assert_eq!(view.render(), "<input type=\"text\">");
    }

    #[test]
    fn if_up_front() {
        let content = "bar";
        let view = html! {
            if false {}
            "foo"
            { content }
        };
        assert_eq!(view.render(), "foobar");
    }

    #[test]
    fn if_up_front_nested() {
        let content = "bar";
        let view = html! {
            <div>
                if false {}
                "foo"
                { content }
            </div>
        };
        assert_eq!(view.render(), "<div>foobar</div>");
    }

    #[test]
    fn optional_attribute() {
        let view = html! { <input required=() /> };
        assert_eq!(view.render(), "<input required>");

        let view = html! { <input required=Some(()) /> };
        assert_eq!(view.render(), "<input required>");

        let view = html! { <input required=Some("true") /> };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view = html! { <input required=Some(Some("true")) /> };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view = html! { <input required=Some(Some(None)) /> };
        assert_eq!(view.render(), "<input>");

        let view = html! { <input required=Some(Some({ (1 + 2).to_string() })) /> };
        assert_eq!(view.render(), "<input required=\"3\">");

        let view = html! { <input required=None /> };
        assert_eq!(view.render(), "<input>");

        let view = html! {
            <input required=if true { "true" } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view = html! {
            <input required=if false { "wat" } else { "true" } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view = html! {
            <input required=if true { () } />
        };
        assert_eq!(view.render(), "<input required>");

        let view = html! {
            <input required=if false { "wat" } else { () } />
        };
        assert_eq!(view.render(), "<input required>");

        let view = html! {
            <input required=if true { Some(()) } />
        };
        assert_eq!(view.render(), "<input required>");

        let view = html! {
            <input required=if true { None } />
        };
        assert_eq!(view.render(), "<input>");

        let view = html! {
            <input required=if true { Some(()) } else { None } />
        };
        assert_eq!(view.render(), "<input required>");

        let view = html! {
            <input required=if false { Some(()) } else { None } />
        };
        assert_eq!(view.render(), "<input>");

        let view = html! {
            <input required=if true { Some("true") } else { None } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view = html! {
            <input required=if false { Some("true") } else { None } />
        };
        assert_eq!(view.render(), "<input>");

        let value = Some("true");
        let view = html! {
            <input required=if let Some(value) = value { Some({ value }) } else { None } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let value = None::<String>;
        let view = html! {
            <input required=if let Some(value) = value { Some({ value }) } else { None } />
        };
        assert_eq!(view.render(), "<input>");
    }
}
