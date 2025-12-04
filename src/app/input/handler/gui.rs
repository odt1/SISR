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
                    ui.label(format!("Device ID: {}", device.id));
                    ui.label(format!(
                        "SDL IDs: {}",
                        device
                            .sdl_ids
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    ui.label(format!("Steam Handle: {}", device.steam_handle));
                    ui.label(format!(
                        "SDL Device Count: {}",
                        device.sdl_device_infos.len()
                    ));

                    for (idx, info) in device.sdl_device_infos.iter().enumerate() {
                        ui.collapsing(
                            format!(
                                "SDL {} #{}",
                                if info.is_gamepad {
                                    "Gamepad"
                                } else {
                                    "Joystick"
                                },
                                idx
                            ),
                            |ui| {
                                for (key, value) in &info.properties {
                                    ui.label(format!("  {}: {}", key, value));
                                }
                            },
                        );
                    }

                    ui.group(|ui| {
                        ui.label("VIIPER Device:");
                        match &device.viiper_device {
                            Some(viiper_dev) => {
                                ui.label(format!("  Connected: {}", device.viiper_connected));
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

            let busses = state
                .devices
                .iter()
                .filter_map(|d| d.viiper_device.as_ref().map(|v| v.bus_id))
                .collect::<Vec<u32>>()
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>();
            ui.label(format!(
                "Bus IDs: {}",
                if busses.is_empty() {
                    "None".to_string()
                } else {
                    busses.join(", ")
                }
            ));
        });
    }
}
