use axum::{async_trait, response::IntoResponse, routing::get, Router};
use axum_liveview::{html, EventContext, Html, LiveView, LiveViewManager, Subscriptions};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

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
    let form = View::default();

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
struct View {
    count: u64,
    prev: Option<Msg>,
}

#[async_trait]
impl LiveView for View {
    type Message = Msg;

    fn init(&self, subscriptions: &mut Subscriptions<Self>) {}

    async fn update(mut self, msg: Msg, ctx: EventContext) -> Self {
        self.count += 1;
        self.prev = Some(msg);
        self
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <div axm-window-keyup={ Msg::Key("window-keyup".to_owned()) } axm-key="escape">
                <div>
                    "Keydown"
                    <br />
                    <input type="text" axm-window-keydown={ Msg::Key("keydown".to_owned()) } />
                </div>

                <div>
                    "Keydown (w debounce)"
                    <br />
                    <input
                        type="text"
                        axm-keydown={ Msg::Key("keydown-w-debounce".to_owned()) }
                        axm-debounce="500"
                    />
                </div>

                <div>
                    "Keyup"
                    <br />
                    <input type="text" axm-keyup={ Msg::Key("keyup".to_owned()) }/>
                </div>

                <hr />

                if let Some(event) = &self.prev {
                    <div>"Event count: " { self.count }</div>
                    <pre>
                        <code>
                            { format!("{:#?}", event) }
                        </code>
                    </pre>
                } else {
                    <div>
                        "No keys pressed yet"
                    </div>
                }
            </div>
        }
    }
}

#[derive(Deserialize, Debug)]
struct Data {
    id: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
enum Msg {
    Key(String),
}
