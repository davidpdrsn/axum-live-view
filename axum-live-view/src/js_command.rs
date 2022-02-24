//! JavaScript commands for performing additional kinds of actions directly in the browser.

use axum::http::Uri;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A JavaScript command that can be sent along with view updates to perform actions
/// [`LiveView::render`] can't otherwise do.
///
/// [`LiveView::render`]: crate::live_view::LiveView::render
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsCommand {
    pub(crate) kind: JsCommandKind,
    pub(crate) delay_ms: Option<u64>,
}

impl JsCommand {
    /// Delay the execution of the command by some duration.
    ///
    /// The duration will be rounded the nearest millisecond.
    ///
    /// Uses [`setTimeout`] in the browser.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// axum_live_view::js_command::add_class(".thing", "hidden").delay(Duration::from_secs(2));
    /// ```
    ///
    /// [`setTimeout`]: https://developer.mozilla.org/en-US/docs/Web/API/setTimeout
    pub fn delay(mut self, duration: Duration) -> Self {
        self.delay_ms = Some(duration.as_millis() as _);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "t")]
pub(crate) enum JsCommandKind {
    NavigateTo { uri: String },
    AddClass { selector: String, klass: String },
    RemoveClass { selector: String, klass: String },
    ToggleClass { selector: String, klass: String },
    ClearValue { selector: String },
    SetTitle { title: String },
    HistoryPushState { uri: String },
}

impl From<JsCommandKind> for JsCommand {
    fn from(kind: JsCommandKind) -> Self {
        JsCommand {
            kind,
            delay_ms: None,
        }
    }
}

/// Navigate to another URL.
///
/// This sets `window.location`.
///
/// Use [`history_push_state`] to change the location without reloading the page.
///
/// # Example
///
/// ```
/// axum_live_view::js_command::navigate_to("/some/other/page".parse().unwrap());
/// ```
pub fn navigate_to(uri: Uri) -> JsCommand {
    JsCommandKind::NavigateTo {
        uri: uri.to_string(),
    }
    .into()
}

/// Add a class to elements matching a CSS selector.
///
/// # Example
///
/// ```
/// axum_live_view::js_command::add_class(".thing", "hidden");
/// ```
pub fn add_class(selector: impl Into<String>, klass: impl Into<String>) -> JsCommand {
    JsCommandKind::AddClass {
        selector: selector.into(),
        klass: klass.into(),
    }
    .into()
}

/// Remove a class from elements matching a CSS selector.
///
/// # Example
///
/// ```
/// axum_live_view::js_command::remove_class(".thing", "hidden");
/// ```
pub fn remove_class(selector: impl Into<String>, klass: impl Into<String>) -> JsCommand {
    JsCommandKind::RemoveClass {
        selector: selector.into(),
        klass: klass.into(),
    }
    .into()
}

/// Toggle a class from elements matching a CSS selector.
///
/// # Example
///
/// ```
/// axum_live_view::js_command::toggle_class(".thing", "hidden");
/// ```
pub fn toggle_class(selector: impl Into<String>, klass: impl Into<String>) -> JsCommand {
    JsCommandKind::ToggleClass {
        selector: selector.into(),
        klass: klass.into(),
    }
    .into()
}

/// Clear the value of input fields matching a CSS selector.
///
/// # Example
///
/// ```
/// axum_live_view::js_command::clear_value(".search-query");
/// ```
pub fn clear_value(selector: impl Into<String>) -> JsCommand {
    JsCommandKind::ClearValue {
        selector: selector.into(),
    }
    .into()
}

/// Change the `<title>`.
///
/// # Example
///
/// ```
/// axum_live_view::js_command::set_title("My page - (2 notifications)");
/// ```
pub fn set_title(title: impl Into<String>) -> JsCommand {
    JsCommandKind::SetTitle {
        title: title.into(),
    }
    .into()
}

/// Change the location without reloading the page.
///
/// This calls [`History.pushState`].
///
/// # Example
///
/// ```
/// axum_live_view::js_command::history_push_state("/some/other/page".parse().unwrap());
/// ```
///
/// [`History.pushState`]: https://developer.mozilla.org/en-US/docs/Web/API/History/pushState
pub fn history_push_state(uri: Uri) -> JsCommand {
    JsCommandKind::HistoryPushState {
        uri: uri.to_string(),
    }
    .into()
}
