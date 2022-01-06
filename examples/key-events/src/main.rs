use axum::{
    async_trait,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_live_view::{html, EventData, Html, LiveView, LiveViewUpgrade, Updated};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, env, net::SocketAddr, path::PathBuf};
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
    let view = View::default();

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
struct View {
    count: u64,
    prev: Option<Msg>,
}

#[async_trait]
impl LiveView for View {
    type Message = Msg;
    type Error = Infallible;

    async fn update(
        mut self,
        msg: Msg,
        _data: Option<EventData>,
    ) -> Result<Updated<Self>, Self::Error> {
        self.count += 1;
        self.prev = Some(msg);
        Ok(Updated::new(self))
    }

    fn render(&self) -> Html<Self::Message> {
        html! {
            <div axm-window-keyup={ Msg::Key("window-keyup".to_owned()) } axm-key="escape" >
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
enum Msg {
    Key(String),
}
