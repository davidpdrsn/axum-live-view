use self::input::Input;
use crate::input::{greater_than, less_than, not_empty, present, BoxedValidation, Validation};
use axum::{extract::Form, response::IntoResponse, routing::get, Router};
use axum_liveview::{html, messages::InputEvent, Html, LiveView, LiveViewManager, Setup};
use serde::Deserialize;
use std::net::SocketAddr;

mod input;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = axum_liveview::pubsub::InProcess::new();

    let app = Router::new()
        .route("/", get(root).post(post_form))
        .merge(axum_liveview::routes())
        .layer(axum_liveview::layer(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr, _>())
        .await
        .unwrap();
}

async fn root(live: LiveViewManager) -> impl IntoResponse {
    let name_input = Input::new("name")
        .type_("text")
        .validation(not_empty().and(present()).boxed());

    let age_input = Input::new("age")
        .type_("number")
        .validation(greater_than(9).and(less_than(21)).and(present()).boxed());

    let form = FormView {
        name_input,
        name_focus: false,
        age_input,
        age_focus: false,
    };

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                { axum_liveview::assets() }
                <link rel="stylesheet" href="https://cdn.simplecss.org/simple.min.css" />
                <style>
                    r#"
                    .focus input {
                        background: rgb(60, 60, 60);
                    }
                    "#
                </style>
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

struct FormView {
    name_input: Input<String, BoxedValidation<String>>,
    name_focus: bool,
    age_input: Input<u32, BoxedValidation<u32>>,
    age_focus: bool,
}

impl LiveView for FormView {
    fn setup(&self, setup: &mut Setup<Self>) {
        setup.on(
            &self.name_input.changed_topic(),
            |mut this: Self, event| async move {
                this.name_input.update_value(event);
                this
            },
        );
        setup.on(
            &self.name_input.blur_topic(),
            |mut this: Self, event| async move {
                this.name_input.update_validations(event);
                this.name_focus = false;
                this
            },
        );
        setup.on(
            &self.name_input.focus_topic(),
            |mut this: Self, _event: InputEvent| async move {
                this.name_focus = true;
                this
            },
        );

        setup.on(
            &self.age_input.changed_topic(),
            |mut this: Self, event| async move {
                this.age_input.update_value(event);
                this
            },
        );
        setup.on(
            &self.age_input.blur_topic(),
            |mut this: Self, event| async move {
                this.age_input.update_validations(event);
                this.age_focus = false;
                this
            },
        );
        setup.on(
            &self.age_input.focus_topic(),
            |mut this: Self, _event: InputEvent| async move {
                this.age_focus = true;
                this
            },
        );
    }

    fn render(&self) -> Html {
        html! {
            <form method="POST" action="/">
                <div class={ if self.name_focus { "focus" } else { "" } }>
                    { self.name_input.render() }
                </div>

                <div class={ if self.age_focus { "focus" } else { "" } }>
                    { self.age_input.render() }
                </div>

                <input type="submit" value="Submit" />
            </form>
        }
    }
}

#[derive(Debug, Deserialize)]
struct Payload {
    name: String,
    age: u32,
}

async fn post_form(Form(payload): Form<Payload>) -> impl IntoResponse {
    format!("{:#?}", payload)
}
