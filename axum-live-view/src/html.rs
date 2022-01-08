use axum::response::IntoResponse;
use private::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone)]
pub struct Html<T> {
    fixed: Vec<String>,
    dynamic: Vec<DynamicFragment<T>>,
}

pub(crate) mod private {
    //! Private API. Do _not_ use anything from this module!

    use super::*;

    pub fn html<T>() -> Html<T> {
        Html {
            fixed: Default::default(),
            dynamic: Default::default(),
        }
    }

    pub fn fixed<T>(html: &mut Html<T>, part: &str) {
        html.fixed.push(part.to_owned());
    }

    pub fn string<T>(html: &mut Html<T>, part: impl Into<DynamicFragment<T>>) {
        html.dynamic.push(part.into());
    }

    pub fn message<T>(html: &mut Html<T>, msg: T) {
        html.dynamic.push(DynamicFragment::Message(msg));
    }

    #[derive(Debug, Clone)]
    pub enum DynamicFragment<T> {
        String(String),
        Message(T),
        Html(Html<T>),
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
}

impl<T> DynamicFragment<T> {
    fn serialize(&self) -> Value
    where
        T: Serialize,
    {
        match self {
            DynamicFragment::String(s) => json!(s),
            DynamicFragment::Message(msg) => json!(serde_json::to_string(&msg).unwrap()),
            DynamicFragment::Html(inner) => json!(inner.serialize()),
        }
    }

    fn map_with_mut<F, K>(self, f: &mut F) -> DynamicFragment<K>
    where
        F: FnMut(T) -> K,
    {
        match self {
            DynamicFragment::String(s) => DynamicFragment::String(s),
            DynamicFragment::Message(msg) => DynamicFragment::Message(f(msg)),
            DynamicFragment::Html(inner) => DynamicFragment::Html(inner.map_with_mut(f)),
        }
    }
}

impl<T> Html<T> {
    pub fn unit(self) -> Html<()> {
        self.map(|_| ())
    }

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
            .map(move |d| d.map_with_mut(f))
            .collect();
        Html {
            fixed: self.fixed,
            dynamic,
        }
    }

    pub(crate) fn diff(&self, other: &Self) -> Option<Diff>
    where
        T: PartialEq + Serialize,
    {
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
                        (DynamicFragment::Message(a), new @ DynamicFragment::Message(b)) => {
                            if a == b {
                                None
                            } else {
                                Some(new.serialize())
                            }
                        }
                        #[allow(clippy::needless_borrow)] // false positive
                        (DynamicFragment::Html(a), DynamicFragment::Html(b)) => {
                            a.diff(&b).map(|diff| json!(diff))
                        }
                        (DynamicFragment::String(a), new @ DynamicFragment::String(b)) => {
                            if a == b {
                                None
                            } else {
                                Some(new.serialize())
                            }
                        }

                        (_, new) => Some(new.serialize()),
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
            None
        } else {
            Some(Diff(json!(out)))
        }
    }

    pub(crate) fn serialize(&self) -> Serialized
    where
        T: Serialize,
    {
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

    pub(crate) fn render(&self) -> String
    where
        T: Serialize,
    {
        use std::borrow::Cow;

        let fixed_rendered = self.fixed.iter().map(|s| Cow::Borrowed(s));

        let dynamic_rendered = self.dynamic.iter().map(|dynamic| match dynamic {
            DynamicFragment::Message(msg) => Cow::Owned(serde_json::to_string(msg).unwrap()),
            DynamicFragment::Html(inner) => Cow::Owned(inner.render()),
            DynamicFragment::String(s) => Cow::Borrowed(s),
        });

        crate::util::interleave::interleave(fixed_rendered, dynamic_rendered).fold(
            String::new(),
            |mut s, a| {
                s.push_str(a.as_str());
                s
            },
        )
    }
}

const FIXED: &str = "f";

impl<T> IntoResponse for Html<T>
where
    T: Serialize,
{
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
    use crate as axum_live_view;
    use crate::html;

    #[test]
    fn basic() {
        let view: Html<()> = html! { <div></div> };
        assert_eq!(view.render(), "<div></div>");
    }

    #[test]
    fn doctype() {
        let view: Html<()> = html! { <!DOCTYPE html> };
        assert_eq!(view.render(), "<!DOCTYPE html>");
    }

    #[test]
    fn text() {
        let view: Html<()> = html! { "foo" };
        assert_eq!(view.render(), "foo");
    }

    #[test]
    fn text_inside_tag() {
        let view: Html<()> = html! { <div>"foo"</div> };
        assert_eq!(view.render(), "<div>foo</div>");
    }

    #[test]
    fn interpolate() {
        let count = 1;
        let view: Html<()> = html! { <div>{ count }</div> };
        assert_eq!(view.render(), "<div>1</div>");
    }

    #[test]
    fn fixed_next_to_dynamic() {
        let count = 1;
        let view: Html<()> = html! {
            <div>"foo"</div>
            <div>{ count }</div>
        };
        assert_eq!(view.render(), "<div>foo</div><div>1</div>");
    }

    #[test]
    fn nested_tags() {
        let view: Html<()> = html! {
            <div>
                <p>"foo"</p>
            </div>
        };
        assert_eq!(view.render(), "<div><p>foo</p></div>");
    }

    #[test]
    fn deeply_nested() {
        let count = 1;
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
            <div>
                <ul>
                    {
                        let nested: Html<()> = html! {
                            <li>"1"</li>
                            <li>"2"</li>
                            <li>"3"</li>
                        };
                        nested
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
        let view: Html<()> = html! {
            <div class="col-md">"foo"</div>
        };
        assert_eq!(view.render(), "<div class=\"col-md\">foo</div>");
    }

    #[test]
    fn multiple_attributes() {
        let view: Html<()> = html! {
            <div class="col-md" id="the-thing">"foo"</div>
        };
        assert_eq!(
            view.render(),
            "<div class=\"col-md\" id=\"the-thing\">foo</div>"
        );
    }

    #[test]
    fn attribute_with_dash() {
        let view: Html<()> = html! {
            <div on-click="do thing">"foo"</div>
        };
        assert_eq!(view.render(), "<div on-click=\"do thing\">foo</div>");
    }

    #[test]
    fn interpolate_class() {
        let size = 8;
        let view: Html<String> = html! {
            <div class={ format!("col-{}", size) }>"foo"</div>
        };
        assert_eq!(view.render(), "<div class=\"col-8\">foo</div>");
    }

    #[test]
    fn empty_attribute() {
        let view: Html<()> = html! {
            <button disabled>"foo"</button>
        };
        assert_eq!(view.render(), "<button disabled>foo</button>");
    }

    #[test]
    fn empty_tag() {
        let view: Html<()> = html! {
            <img src="foo.png" />
        };
        assert_eq!(view.render(), "<img src=\"foo.png\">");
    }

    #[test]
    fn conditional() {
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
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
        let view: Html<()> = html! {
            <input type="text" />
        };
        assert_eq!(view.render(), "<input type=\"text\">");
    }

    #[test]
    fn if_up_front() {
        let content = "bar";
        let view: Html<()> = html! {
            if false {}
            "foo"
            { content }
        };
        assert_eq!(view.render(), "foobar");
    }

    #[test]
    fn if_up_front_nested() {
        let content = "bar";
        let view: Html<()> = html! {
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
        let view: Html<()> = html! { <input required=() /> };
        assert_eq!(view.render(), "<input required>");

        let view: Html<()> = html! { <input required=Some(()) /> };
        assert_eq!(view.render(), "<input required>");

        let view: Html<()> = html! { <input required=Some("true") /> };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view: Html<()> = html! { <input required=Some(Some("true")) /> };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view: Html<()> = html! { <input required=Some(Some(None)) /> };
        assert_eq!(view.render(), "<input>");

        let view: Html<()> = html! { <input required=Some(Some({ (1 + 2).to_string() })) /> };
        assert_eq!(view.render(), "<input required=\"3\">");

        let view: Html<()> = html! { <input required=None /> };
        assert_eq!(view.render(), "<input>");

        let view: Html<()> = html! {
            <input required=if true { "true" } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view: Html<()> = html! {
            <input required=if false { "wat" } else { "true" } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view: Html<()> = html! {
            <input required=if true { () } />
        };
        assert_eq!(view.render(), "<input required>");

        let view: Html<()> = html! {
            <input required=if false { "wat" } else { () } />
        };
        assert_eq!(view.render(), "<input required>");

        let view: Html<()> = html! {
            <input required=if true { Some(()) } />
        };
        assert_eq!(view.render(), "<input required>");

        let view: Html<()> = html! {
            <input required=if true { None } />
        };
        assert_eq!(view.render(), "<input>");

        let view: Html<()> = html! {
            <input required=if true { Some(()) } else { None } />
        };
        assert_eq!(view.render(), "<input required>");

        let view: Html<()> = html! {
            <input required=if false { Some(()) } else { None } />
        };
        assert_eq!(view.render(), "<input>");

        let view: Html<()> = html! {
            <input required=if true { Some("true") } else { None } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let view: Html<()> = html! {
            <input required=if false { Some("true") } else { None } />
        };
        assert_eq!(view.render(), "<input>");

        let value = Some("true");
        let view: Html<()> = html! {
            <input required=if let Some(value) = value { Some({ value }) } else { None } />
        };
        assert_eq!(view.render(), "<input required=\"true\">");

        let value = None::<String>;
        let view: Html<()> = html! {
            <input required=if let Some(value) = value { Some({ value }) } else { None } />
        };
        assert_eq!(view.render(), "<input>");
    }

    #[test]
    fn axm_attribute() {
        let view: Html<&str> = html! { <input axm-foo={ "foo" } /> };
        assert_eq!(view.render(), "<input axm-foo=\"foo\">");

        let view: Html<&str> = html! { <input axm-foo=if true { "foo" } else { "bar" } /> };
        assert_eq!(view.render(), "<input axm-foo=\"foo\">");

        let view: Html<Option<&str>> =
            html! { <input axm-foo=if true { Some("foo") } else { None } /> };
        assert_eq!(view.render(), "<input axm-foo=\"foo\">");

        #[derive(Serialize)]
        enum Msg {
            Foo,
            Bar { value: i32 },
        }

        let view: Html<Msg> = html! { <input axm-foo={ Msg::Foo } /> };
        assert_eq!(view.render(), "<input axm-foo=\"Foo\">");

        let view: Html<Msg> = html! { <input axm-foo={ Msg::Bar { value: 123 } } /> };
        assert_eq!(view.render(), "<input axm-foo={\"Bar\":{\"value\":123}}>");
    }

    #[test]
    fn axm_enum_update_attribute() {
        #[derive(Serialize)]
        struct Msg {
            n: i32,
        }

        let view = html! { <foo axm-click={ Msg { n: 1 } } /> };
        let json = json!(view.serialize());
        dbg!(&json);
        assert_eq!(
            json,
            serde_json::json!({
                "0": "{\"n\":1}",
                "f": [
                    "<foo axm-click=",
                    ">",
                ],
            })
        );
    }
}
