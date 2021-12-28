use axum::{async_trait, response::IntoResponse, routing::get, Json, Router};
use axum_liveview::{html, liveview::EventContext, Html, LiveView, LiveViewManager, Setup};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = axum_liveview::pubsub::InProcess::new();

    let app = Router::new()
        .route("/", get(root))
        .merge(axum_liveview::routes())
        .layer(axum_liveview::layer(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr, _>())
        .await
        .unwrap();
}

async fn root(live: LiveViewManager) -> impl IntoResponse {
    let form = FormView::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                { axum_liveview::assets() }
            </head>
            <body>
                { live.embed(form) }
                <script>
                    r#"
                        const liveView = new LiveView({ host: 'localhost', port: 4000 })
                        liveView.connect()
                    "#
                </script>
            </body>
        </html>
    }
}

#[derive(Default)]
struct FormView {
    text_input_value: String,
    textarea_value: String,
    errors: Vec<String>,
    values: Option<FormValues>,
}

#[async_trait]
impl LiveView for FormView {
    type Message = Msg;

    fn setup(&self, setup: &mut Setup<Self>) {}

    async fn update(self, msg: Msg, ctx: EventContext) -> Self {
        self
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <form axm-change={ Msg::Validate } axm-submit={ Msg::Submit } axm-throttle="1000">
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
                    <select name="numbers" size="6" multiple axm-change={ Msg::Changed(Input::MultiSelect) }>
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
                                    name="checkboxes"
                                    value={ n }
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
    // #[tracing::instrument(skip(self))]
    // async fn text_input_changed(mut self, event: FormEvent) -> Self {
    //     self.text_input_value = event.into_value();
    //     self
    // }

    // #[tracing::instrument(skip(self))]
    // async fn changed(self, _event: FormEvent<ChangedInputValue, ChangedInput>) -> Self {
    //     self
    // }

    // #[tracing::instrument(skip(self))]
    // async fn textarea_changed(mut self, event: FormEvent) -> Self {
    //     self.textarea_value = event.into_value();
    //     self
    // }

    // #[tracing::instrument(skip(self))]
    // async fn validate(mut self, event: FormEvent<FormValues>) -> Self {
    //     self.perform_validations(event.value());
    //     self
    // }

    // #[tracing::instrument(skip(self))]
    // async fn submit(mut self, event: FormEvent<FormValues>) -> Self {
    //     self.perform_validations(event.value());
    //     if self.errors.is_empty() {
    //         tracing::info!("submitting");
    //     } else {
    //         tracing::info!("there are warnings, not submitting");
    //     }
    //     self.values = Some(event.into_value());
    //     self
    // }

    // fn perform_validations(&mut self, values: &FormValues) {
    //     self.errors.clear();

    //     let FormValues {
    //         input,
    //         textarea,
    //         number,
    //         numbers,
    //         radio,
    //         checkboxes,
    //     } = values;

    //     if input.is_empty() {
    //         self.errors.push("`input` cannot be empty".to_owned());
    //     }

    //     if textarea.len() > TEXTAREA_MAX_LEN as _ {
    //         self.errors.push(format!(
    //             "textarea cannot be longer than {} characters",
    //             TEXTAREA_MAX_LEN
    //         ));
    //     }

    //     if number == "1" {
    //         self.errors.push("`number` cannot be 1".to_owned());
    //     }

    //     if numbers.len() > 3 {
    //         self.errors
    //             .push("cannot select more than 3 options".to_owned());
    //     }

    //     if radio.is_none() {
    //         self.errors.push("no radio option checked".to_owned());
    //     }

    //     if checkboxes.values().filter(|value| **value).count() > 3 {
    //         self.errors
    //             .push("cannot check more than 3 boxes".to_owned());
    //     }
    // }
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

#[derive(Debug, Deserialize)]
struct FormValues {
    input: String,
    textarea: String,
    number: String,
    numbers: Vec<String>,
    radio: Option<String>,
    checkboxes: HashMap<String, bool>,
}

const TEXTAREA_MAX_LEN: i32 = 10;
