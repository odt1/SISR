use tracing::warn;

use crate::app::window::RunnerEvent;

use super::{EventHandler, State};

impl EventHandler {
    pub(super) fn request_redraw(&self) {
        _ = self
            // the legend of zelda: the
            .winit_waker
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|p| p.send_event(RunnerEvent::Redraw())))
            .map(|r| r.inspect_err(|e| warn!("Failed to request GUI redraw: {}", e)));
    }

    pub(super) fn on_draw(state: &mut State, ctx: &egui::Context) {
        egui::Window::new("🎮 GamePads").show(ctx, |ui| {
            ui.label("Connected GamePads:");
            for device in &state.devices {
                ui.group(|ui| {
                    ui.label(format!("Pad ID: {}", device.id));
                    ui.label(format!("Steam Handle: {}", device.steam_handle));
                    ui.label(format!("SDL Device Count: {}", device.sdl_device_count));
                    ui.group(|ui| {
                        ui.label("VIIPER Device:");
                        match &device.viiper_device {
                            Some(viiper_dev) => {
                                ui.label(format!("  Bus ID: {}", viiper_dev.bus_id));
                                ui.label(format!("  Device ID: {}", viiper_dev.dev_id));
                                ui.label(format!("  Type: {}", viiper_dev.r#type));
                                ui.label(format!("  Vendor ID: {:?}", viiper_dev.vid));
                                ui.label(format!("  Product ID: {:?}", viiper_dev.pid));
                            }
                            None => {
                                ui.label("  Not connected");
                            }
                        }
                    });
                });
            }
        });
        egui::Window::new("🐍 VIIPER").show(ctx, |ui| {
            ui.label(format!(
                "VIIPER Address: {}",
                state
                    .viiper_address
                    .map(|addr| addr.to_string())
                    .unwrap_or("None".to_string())
            ));
            ui.label(format!(
                "Bus ID: {}",
                state
                    .viiper_bus
                    .map(|id| id.to_string())
                    .unwrap_or("Not created".to_string())
            ));
        });
    }
}
