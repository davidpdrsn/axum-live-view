use std::collections::BTreeMap;

use serde::Serialize;

use super::{DynamicFragment, Html};

impl<T> Html<T> {
    pub(crate) fn render(&self) -> String
    where
        T: Serialize,
    {
        let mut out = String::new();
        let _ = render_to(self.fixed, &self.dynamic, &mut out);
        out
    }
}

fn render_to<T>(
    fixed: &'static [&'static str],
    dynamic: &BTreeMap<usize, DynamicFragment<T>>,
    out: &mut String,
) -> Result<(), ()>
where
    T: Serialize,
{
    let mut dynamic_iter = dynamic.iter();

    for f in fixed {
        out.push_str(f);

        match dynamic_iter.next() {
            Some((_, DynamicFragment::Html(html))) => {
                let _ = render_to(html.fixed, &html.dynamic, out);
            }
            Some((_, DynamicFragment::String(s))) => {
                out.push_str(&*s);
            }
            Some((_, DynamicFragment::Message(msg))) => {
                let encoded_msg = serde_json::to_string(msg).unwrap();
                out.push_str(&encoded_msg);
            }
            Some((
                _,
                DynamicFragment::DedupLoop {
                    fixed: loop_fixed,
                    dynamic,
                },
            )) => {
                for d in dynamic {
                    let _ = render_to(loop_fixed, d, out);
                }
            }
            None => {}
        }
    }

    Ok(())
}
