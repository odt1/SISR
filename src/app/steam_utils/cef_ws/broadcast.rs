use serde::Serialize;
use std::sync::Arc;
use tracing::trace;

use super::server::WebSocketServer;

static SERVER: std::sync::OnceLock<Arc<WebSocketServer>> = std::sync::OnceLock::new();

pub(crate) fn set_server(server: Arc<WebSocketServer>) {
    let _ = SERVER.set(server);
}

pub fn broadcast_ws<T: Serialize>(message: T) {
    match SERVER.get() {
        Some(server) => server.broadcast(message),
        None => trace!("No WebSocket server available for broadcast"),
    }
}
