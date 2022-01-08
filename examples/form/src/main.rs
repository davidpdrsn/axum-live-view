use axum::{
    async_trait,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_live_view::{
    event_data::EventData, html, live_view::Updated, Html, LiveView, LiveViewUpgrade,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, env, net::SocketAddr, path::PathBuf};
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/", get(root)).route(
        "/bundle.js",
        get_service(ServeFile::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist/bundle.js"),
        ))
        .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(live: LiveViewUpgrade) -> impl IntoResponse {
    let view = FormView::default();

    live.response(move |embed| {
        html! {
            <!DOCTYPE html>
            <html>
                <head>
                </head>
                <body>
                    { embed.embed(view) }
                    <script src="/bundle.js"></script>
                </body>
            </html>
        }
    })
}

#[derive(Default, Clone)]
struct FormView {
    text_input_value: String,
    textarea_value: String,
    errors: Vec<String>,
    values: Option<FormValues>,
}

#[async_trait]
impl LiveView for FormView {
    type Message = Msg;
    type Error = Infallible;

    async fn update(
        mut self,
        msg: Msg,
        data: Option<EventData>,
    ) -> Result<Updated<Self>, Infallible> {
        match msg {
            Msg::Validate => {
                let values: FormValues = data.unwrap().as_form().unwrap().deserialize().unwrap();
                self.perform_validations(&values);
            }
            Msg::Submit => {
                let values: FormValues = data.unwrap().as_form().unwrap().deserialize().unwrap();
                self.perform_validations(&values);

                if self.errors.is_empty() {
                    tracing::info!("submitting");
                } else {
                    tracing::info!("there are warnings, not submitting");
                }
                self.values = Some(values);
            }
            Msg::TextInputChanged => {
                self.text_input_value = data
                    .unwrap()
                    .as_input()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
            }
            Msg::TextAreaChanged => {
                self.textarea_value = data
                    .unwrap()
                    .as_input()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
            }
            Msg::Changed(msg) => {
                println!("change: {:?}", msg);
            }
        }

        Ok(Updated::new(self))
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <form
                axm-change={ Msg::Validate }
                axm-submit={ Msg::Submit }
            >
                <label>
                    <div>"Text input"</div>
                    <input type="text" name="input" axm-input={ Msg::TextInputChanged } axm-debounce="1000" />
                    if !self.text_input_value.is_empty() {
                        <div>
                            "Value: " { &self.text_input_value }
                        </div>
                    }
                </label>

                <label>
                    <div>"Textarea"</div>
                    <textarea name="textarea" axm-input={ Msg::TextAreaChanged }></textarea>
                    <div>
                        "Chars remaining: " { TEXTAREA_MAX_LEN - self.textarea_value.len() as i32 }
                    </div>
                </label>

                <label>
                    <div>"Select"</div>
                    <select name="number" axm-change={ Msg::Changed(Input::Select) }>
                        for n in 0..5 {
                            <option value={ n }>{ n }</option>
                        }
                    </select>
                </label>

                <label>
                    <div>"Multi select"</div>
                    <select name="numbers[]" size="6" multiple axm-change={ Msg::Changed(Input::MultiSelect) }>
                        for n in 0..5 {
                            <option value={ n }>{ n }</option>
                        }
                    </select>
                </label>

                <div>
                    <div>"Radio buttons"</div>
                    for n in 0..5 {
                        <div>
                            <label>
                                <input
                                    type="radio"
                                    name="radio"
                                    value={ n }
                                    axm-change={ Msg::Changed(Input::Radio(n)) }
                                />
                                { n }
                            </label>
                        </div>
                    }
                </div>

                <div>
                    <div>"Check boxes"</div>
                    for n in 0..5 {
                        <div>
                            <label>
                                <input
                                    type="checkbox"
                                    name={ format!("checkboxes[{}]", n) }
                                    value="true"
                                    axm-change={ Msg::Changed(Input::Checkbox(n)) }
                                />
                                { n }
                            </label>
                        </div>
                    }
                </div>


                if !self.errors.is_empty() {
                    <ul>
                        for error in &self.errors {
                            <li>{ error }</li>
                        }
                    </ul>
                }

                <input type="submit" value="Submit!" />

                if let Some(values) = &self.values {
                    <div>
                        <code><pre>{ format!("{:#?}", values) }</pre></code>
                    </div>
                }
            </form>
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Msg {
    Validate,
    Submit,
    TextInputChanged,
    TextAreaChanged,
    Changed(Input),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Input {
    Select,
    MultiSelect,
    Radio(u32),
    Checkbox(u32),
}

impl FormView {
    fn perform_validations(&mut self, values: &FormValues) {
        self.errors.clear();

        let FormValues {
            input,
            textarea,
            number,
            numbers,
            radio,
            checkboxes,
        } = values;

        if input.is_empty() {
            self.errors.push("`input` cannot be empty".to_owned());
        }

        if textarea.len() > TEXTAREA_MAX_LEN as _ {
            self.errors.push(format!(
                "textarea cannot be longer than {} characters",
                TEXTAREA_MAX_LEN
            ));
        }

        if number == "1" {
            self.errors.push("`number` cannot be 1".to_owned());
        }

        if numbers.len() > 3 {
            self.errors
                .push("cannot select more than 3 options".to_owned());
        }

        if radio.is_none() {
            self.errors.push("no radio option checked".to_owned());
        }

        if checkboxes.values().filter(|value| **value).count() > 3 {
            self.errors
                .push("cannot check more than 3 boxes".to_owned());
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ChangedInputValue {
    Select(String),
    MultiSelect(Vec<String>),
    RadioOrCheckbox(bool),
}

#[derive(Debug, Deserialize)]
struct ChangedInput {
    input: String,
}

#[derive(Debug, Deserialize, Clone)]
struct FormValues {
    input: String,
    textarea: String,
    number: String,
    #[serde(default)]
    numbers: Vec<String>,
    #[serde(default)]
    radio: Option<String>,
    #[serde(default)]
    checkboxes: HashMap<String, bool>,
}

const TEXTAREA_MAX_LEN: i32 = 10;
