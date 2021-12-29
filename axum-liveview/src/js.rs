use std::time::Duration;

use axum::http::Uri;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsCommand {
    pub(crate) kind: JsCommandKind,
    pub(crate) delay_ms: Option<u64>,
}

impl JsCommand {
    pub fn delay(mut self, duration: Duration) -> Self {
        self.delay_ms = Some(duration.as_millis() as _);
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum JsCommandKind {
    NavigateTo { uri: String },
    AddClass { selector: String, class: String },
    RemoveClass { selector: String, class: String },
    ToggleClass { selector: String, class: String },
    ClearValue { selector: String },
    SetTitle { title: String },
}

fn command(kind: JsCommandKind) -> JsCommand {
    JsCommand {
        kind,
        delay_ms: None,
    }
}

pub fn navigate_to(uri: Uri) -> JsCommand {
    command(JsCommandKind::NavigateTo {
        uri: uri.to_string(),
    })
}

pub fn add_class(selector: impl Into<String>, class: impl Into<String>) -> JsCommand {
    command(JsCommandKind::AddClass {
        selector: selector.into(),
        class: class.into(),
    })
}

pub fn remove_class(selector: impl Into<String>, class: impl Into<String>) -> JsCommand {
    command(JsCommandKind::RemoveClass {
        selector: selector.into(),
        class: class.into(),
    })
}

pub fn toggle_class(selector: impl Into<String>, class: impl Into<String>) -> JsCommand {
    command(JsCommandKind::ToggleClass {
        selector: selector.into(),
        class: class.into(),
    })
}

pub fn clear_value(selector: impl Into<String>) -> JsCommand {
    command(JsCommandKind::ClearValue {
        selector: selector.into(),
    })
}

pub fn set_title(title: impl Into<String>) -> JsCommand {
    command(JsCommandKind::SetTitle {
        title: title.into(),
    })
}
