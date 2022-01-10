use super::*;
use crate as axum_live_view;
use crate::html;
use serde_json::json;

fn pretty_print<T>(t: T) -> T
where
    T: Serialize,
{
    println!("{}", serde_json::to_string_pretty(&t).unwrap());
    t
}

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
        concat!(
            "<ul>",
            "<li>alice</li>",
            "<li>bob</li>",
            "<li>cindy</li>",
            "</ul>",
        ),
    );
}

#[test]
fn for_loop_with_conditional() {
    let ns = [1, 11, 2];
    let view: Html<()> = html! {
        <ul>
            for n in ns {
                <li>
                if n >= 10 {
                    <strong>"big number"</strong>
                } else {
                    { n }
                }
                </li>
            }
        </ul>
    };
    assert_eq!(
        view.render(),
        concat!(
            "<ul>",
            "<li>1</li>",
            "<li><strong>big number</strong></li>",
            "<li>2</li>",
            "</ul>",
        ),
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
    let json = json!(view);
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    assert_json_diff::assert_json_eq!(
        json,
        json!({
            "d": {
                "0": "{\"n\":1}",
            },
            "f": [
                "<foo axm-click=",
                ">",
            ],
        })
    );
}

#[test]
fn diffing_fixed() {
    let old: Html<()> = html! { <div>"old"</div> };
    let new: Html<()> = html! { <div>"new"</div> };
    let diff = old.diff(&new);
    assert_json_diff::assert_json_eq!(
        diff,
        json!({
            "f": ["<div>new</div>"],
        })
    );
}

#[test]
fn diffing_dynamic() {
    fn render(value: i32) -> Html<()> {
        html! { <div>{ value }</div> }
    }
    let old = render(1);
    let new = render(2);
    let diff = old.diff(&new);
    assert_json_diff::assert_json_eq!(
        diff,
        json!({
            "d": {
                "0": "2"
            }
        })
    );
}

#[test]
fn diffing_dynamic_multiple_dynamics() {
    fn render(one: i32, two: i32) -> Html<()> {
        html! { <div>{ one } " and " { two }</div> }
    }

    let a = render(1, 2);

    let b = render(1, 2);
    assert_json_diff::assert_json_eq!(a.diff(&b), json!(null));

    let b = render(2, 2);
    assert_json_diff::assert_json_eq!(
        a.diff(&b),
        json!({
            "d": {
                "0": "2",
            }
        })
    );

    let b = render(2, 3);
    assert_json_diff::assert_json_eq!(
        a.diff(&b),
        json!({
            "d": {
                "0": "2",
                "1": "3",
            }
        })
    );
}

#[test]
fn diffing_dynamic_changing_fixed() {
    fn render(n: i32) -> Html<()> {
        html! {
            <div>{ n }</div>
            if n >= 10 {
                <div>"big number"</div>
            }
        }
    }

    let a = render(1);
    let b = render(11);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": "11",
                "1": {
                    "f": ["<div>big number</div>"],
                }
            },
        })
    );

    let a = render(11);
    let b = render(12);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": "12",
            },
        })
    );
}

#[test]
fn diffing_loop_dynaming_changes() {
    fn render(ns: &[i32]) -> Html<()> {
        html! {
            <ul>
                for n in ns {
                    <li>{ n }</li>
                }
            </ul>
        }
    }

    let a = render(&[1, 2, 3]);
    let b = render(&[1, 2, 3]);
    assert_json_diff::assert_json_eq!(pretty_print(a.diff(&b)), json!(null));

    let a = render(&[1, 2]);
    let b = render(&[3, 4]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "b": {
                        "0": { "0": "3" },
                        "1": { "0": "4" }
                    }
                }
            }
        })
    );

    let a = render(&[1, 2]);
    let b = render(&[2, 2]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "b": {
                        "0": { "0": "2" },
                    }
                }
            }
        })
    );

    let a = render(&[1]);
    let b = render(&[1, 2]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "b": {
                        "1": { "0": "2" },
                    }
                }
            }
        })
    );
}

#[test]
fn diffing_loop_fixed_changes() {
    fn render_one(ns: &[i32]) -> Html<()> {
        html! {
            <ul>
                for n in ns {
                    <li>{ n }</li>
                }
            </ul>
        }
    }

    fn render_two(ns: &[i32]) -> Html<()> {
        html! {
            <ul>
                for n in ns {
                    <li disabled>{ n }</li>
                }
            </ul>
        }
    }

    let a = render_one(&[1, 2, 3]);
    let b = render_two(&[1, 2, 3]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "f": [
                        "<li disabled>",
                        "</li>"
                    ],
                }
            }
        })
    );

    let a = render_one(&[1, 2]);
    let b = render_two(&[1, 2, 3]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "f": [
                        "<li disabled>",
                        "</li>"
                    ],
                    "b": {
                        "2": { "0": "3" }
                    }
                }
            }
        })
    );

    let a = render_one(&[1, 2, 3]);
    let b = render_two(&[1, 2]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "f": [
                        "<li disabled>",
                        "</li>"
                    ],
                    "b": {
                        "2": null
                    }
                }
            }
        })
    );
}

#[test]
fn diffing_removing_dynamic() {
    fn render_one(n: i32, m: i32) -> Html<()> {
        html! {
            { n }
            { m }
        }
    }

    fn render_two(n: i32) -> Html<()> {
        html! {
            { n }
        }
    }

    let a = render_one(1, 2);
    let b = render_two(1);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "f": [""],
            "d": {
                "1": null,
            }
        })
    );
}

#[test]
fn diffing_loop_conditional() {
    fn render(ns: &[i32]) -> Html<()> {
        html! {
            <ul>
                for n in ns {
                    <li>
                        if *n >= 10 {
                            <strong>"big number"</strong>
                        } else {
                            { n }
                        }
                    </li>
                }
            </ul>
        }
    }

    let a = render(&[1, 2, 3]);
    let b = render(&[1, 11, 3]);
    assert_json_diff::assert_json_eq!(
        pretty_print(a.diff(&b)),
        json!({
            "d": {
                "0": {
                    "b": {
                        "1": {
                            "0": {
                                "f": ["<strong>big number</strong>"],
                                "d": { "0": null }
                            }
                        },
                    }
                }
            }
        })
    );
}

#[test]
fn diffing_message() {
    fn render(msg: i32) -> Html<i32> {
        html! { <button axm-click={ msg }></button> }
    }

    let a = render(1);
    assert_json_diff::assert_json_eq!(a.diff(&a), json!(null));

    let b = render(2);
    assert_json_diff::assert_json_eq!(
        a.diff(&b),
        json!({
            "d": {
                "0": "2",
            }
        })
    );
}
