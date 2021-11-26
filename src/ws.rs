use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};

pub(crate) fn routes<B>() -> Router<B>
where
    B: Send + 'static,
{
    Router::new().route("/live", get(ws))
}

async fn ws(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    todo!()
}
