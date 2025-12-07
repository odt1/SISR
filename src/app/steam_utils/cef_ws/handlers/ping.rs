use serde::Serialize;
use std::sync::{Arc, Mutex};
use tracing::debug;
use winit::event_loop::EventLoopProxy;

use crate::app::steam_utils::cef_ws::response_writer::ResponseWriter;
use crate::app::steam_utils::cef_ws::{CefMessage, broadcast_ws};
use crate::app::window::RunnerEvent;

#[derive(Serialize)]
struct PongResponse {
    pong: bool,
    timestamp: u64,
}

pub fn handle(
    _message: &CefMessage,
    _winit_waker: &Arc<Mutex<Option<EventLoopProxy<RunnerEvent>>>>,
    _sdl_waker: &Arc<Mutex<Option<sdl3::event::EventSender>>>,
    writer: &ResponseWriter,
) {
    debug!("CEF Debug WebSocket: Received ping");

    let response = PongResponse {
        pong: true,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    broadcast_ws("meh");

    let _ = writer.write(response);
}
