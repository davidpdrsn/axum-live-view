use axum::{
    async_trait,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_live_view::{
    html,
    live_view::{EmbedLiveView, EventData, LiveView, Subscriptions, Updated},
    middleware::LiveViewLayer,
    Html,
};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf};
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pubsub = axum_live_view::pubsub::InProcess::new();

    let app = Router::new()
        .route("/", get(root))
        .route(
            "/bundle.js",
            get_service(ServeFile::new(
                PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("dist/bundle.js"),
            ))
            .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .merge(axum_live_view::routes())
        .layer(LiveViewLayer::new(pubsub));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(embed_liveview: EmbedLiveView) -> impl IntoResponse {
    let counter = View::default();

    html! {
        <!DOCTYPE html>
        <html>
            <head>
                <script src="/bundle.js"></script>
            </head>
            <body>
                { embed_liveview.embed(counter) }
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

    fn init(&self, _subscriptions: &mut Subscriptions<Self>) {}

    async fn update(mut self, msg: Msg, _data: EventData) -> Updated<Self> {
        self.count += 1;
        self.prev = Some(msg);
        Updated::new(self)
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <div axm-window-keyup={ Msg::Key("window-keyup".to_owned()) } axm-key="escape">
                <div>
                    "Keydown"
                    <br />
                    <input type="text" axm-keydown={ Msg::Key("keydown".to_owned()) } />
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
