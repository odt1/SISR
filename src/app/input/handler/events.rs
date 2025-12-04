use sdl3::event::Event;
use tracing::{debug, error, info, trace, warn};

use crate::{
    app::input::{
        device::{Device, DeviceState, SDLDevice},
        handler::ViiperEvent,
        sdl::get_gamepad_steam_handle,
        sdl_device_info::SdlDeviceInfo,
    },
    event_which,
};

use super::EventHandler;

impl EventHandler {
    pub fn on_pad_added(&mut self, event: &Event) {
        let (which, is_joystick) = match event {
            Event::JoyDeviceAdded { which, .. } => (which, true),
            Event::ControllerDeviceAdded { which, .. } => (which, false),
            _ => {
                warn!("Unexpected event for pad addition: {:?}", event);
                return;
            }
        };

        trace!(
            "{} added with ID {}",
            if is_joystick { "Joystick" } else { "Gamepad" },
            which
        );

        let sdl_dev = if is_joystick {
            self.sdl_joystick.open(*which).ok().map(SDLDevice::Joystick)
        } else {
            self.sdl_gamepad.open(*which).ok().map(SDLDevice::Gamepad)
        };
        let Some(sdl_device) = sdl_dev else {
            warn!("Failed to open SDL device with ID {}", which);
            return;
        };

        let steam_handle = match &sdl_device {
            SDLDevice::Joystick(_) => 0,
            SDLDevice::Gamepad(p) => get_gamepad_steam_handle(p),
        };

        let sdl_device_info = SdlDeviceInfo::from(&sdl_device);
        self.sdl_devices.entry(*which).or_default().push(sdl_device);

        let Ok(mut guard) = self.state.lock() else {
            error!("Failed to lock state for adding device");
            return;
        };

        if let Some(&device_id) = self.sdl_id_to_device.get(which) {
            if let Some(existing_device) = guard.devices.iter_mut().find(|d| d.id == device_id) {
                existing_device.sdl_device_infos.push(sdl_device_info);
                debug!(
                    "Added extra SDL {} for device {}; SDL ID {}; Total SDL devices {}",
                    if is_joystick { "Joystick" } else { "Gamepad" },
                    device_id,
                    which,
                    existing_device.sdl_device_infos.len()
                );
                handle_existing_device_connect(
                    &mut self.viiper,
                    existing_device,
                    steam_handle,
                    *which,
                );
            }
        } else {
            let device_id = self.next_device_id;
            self.next_device_id += 1;
            self.sdl_id_to_device.insert(*which, device_id);

            handle_new_device(
                &mut self.viiper,
                &mut guard,
                device_id,
                *which,
                steam_handle,
                is_joystick,
                sdl_device_info,
            );
        }
    }
    pub fn on_pad_removed(&mut self, event: &Event) {
        match event {
            Event::JoyDeviceRemoved { which, .. }
            | Event::ControllerDeviceRemoved { which, .. } => {
                trace!(
                    "{} removed with ID {}",
                    if matches!(event, Event::JoyDeviceRemoved { .. }) {
                        "Joystick"
                    } else {
                        "Gamepad"
                    },
                    which
                );

                if let Some(devices) = self.sdl_devices.get_mut(which) {
                    _ = devices.pop();
                    if devices.is_empty() {
                        self.sdl_devices.remove(which);
                    }
                }

                if let Ok(mut guard) = self
                    .state
                    .lock()
                    .map_err(|e| error!("Failed to lock state for removing device: {}", e))
                {
                    let is_joystick = matches!(event, Event::JoyDeviceRemoved { .. });

                    let Some(&device_id) = self.sdl_id_to_device.get(which) else {
                        warn!("No device found for SDL ID {} in pad removal", which);
                        return;
                    };

                    let Some(device) = guard.devices.iter_mut().find(|d| d.id == device_id) else {
                        warn!(
                            "Device {} not found in state for SDL ID {}",
                            device_id, which
                        );
                        return;
                    };

                    let before_len = device.sdl_device_infos.len();

                    if let Some(idx) = device
                        .sdl_device_infos
                        .iter()
                        .position(|info| info.is_gamepad != is_joystick)
                    {
                        device.sdl_device_infos.remove(idx);
                        debug!(
                            "Removed SDL {} device info for device {}; SDL ID {}; Remaining SDL devices {}",
                            if is_joystick { "Joystick" } else { "Gamepad" },
                            device_id,
                            which,
                            device.sdl_device_infos.len()
                        );
                    } else if before_len > 0 {
                        warn!(
                            "Could not find matching SDL {} device info to remove for device {}",
                            if is_joystick { "Joystick" } else { "Gamepad" },
                            device_id
                        );
                    }

                    if device.sdl_device_infos.is_empty() {
                        device.sdl_ids.remove(which);

                        if device.sdl_ids.is_empty() {
                            self.viiper.remove_device(device_id);
                            self.sdl_id_to_device.remove(which);
                            guard.devices.retain(|d| d.id != device_id);
                            info!(
                                "Removed device {} (last SDL {} with ID {})",
                                device_id,
                                if is_joystick { "Joystick" } else { "Gamepad" },
                                which
                            );
                        }
                    }
                }
            }
            _ => {
                warn!("Unexpected event for pad removal: {:?}", event);
            }
        }
    }

    // The high-level sdl3-rs `Event::Unknown` doesn't expose the `which` field from
    // `SDL_GamepadDeviceEvent`. We work around this by refreshing all tracked pads.
    //
    // See: https://github.com/libsdl-org/SDL/blob/main/include/SDL3/SDL_events.h#L672-L677
    pub fn on_steam_handle_updated(&mut self, _event: &Event) {
        let Ok(mut guard) = self.state.lock() else {
            warn!("Failed to lock state for steam handle update");
            return;
        };
        // should only be one bus for all devices, this is fine for now!
        self.sdl_devices
            .values()
            .flat_map(|d| {
                d.iter().filter_map(|dev| match dev {
                    SDLDevice::Gamepad(pad) => Some(pad),
                    _ => None,
                })
            })
            .for_each(|pad| {
                let Ok(instance_id) = pad.id() else {
                    return;
                };
                let steam_handle = get_gamepad_steam_handle(pad);

                // Look up our device ID for this SDL instance ID
                let Some(&device_id) = self.sdl_id_to_device.get(&instance_id) else {
                    return;
                };

                if let Some(device) = guard.devices.iter_mut().find(|d| d.id == device_id) {
                    if device.steam_handle != steam_handle {
                        device.steam_handle = steam_handle;
                        info!(
                            "Updated steam handle for device {} (SDL ID {}) to {}",
                            device_id, instance_id, steam_handle
                        );
                    }

                    if device.viiper_device.is_none() {
                        info!(
                            "Connecting device {} upon steam handle update with steam handle {}",
                            device_id, steam_handle
                        );
                        self.viiper.create_device(device);
                    }
                }
            });
        self.request_redraw();
    }

    pub fn on_pad_event(&self, event: &Event) {
        match event {
            Event::Unknown { .. } => {
                // if nothing "outside" changed is
                // GAMEPAD_STATE_UPDATE_COMPLETE or JOYPAD_STATE_UPDATE_COMPLETE
                // Silently Ignore for now
                // Would need "supertrace" log level lol
                trace!("Unknown gamepad event: {:?}", event);
            }
            _ => {
                if event.is_joy() {
                    // Currently just drop lower level joystick events
                    return;
                }
                if !event.is_controller() {
                    warn!("Received non-gamepad event in on_pad_event: {:?}", event);
                    return;
                }
                // handle all other events and just "update gamepad"
                // instead of duplicating code for every shit"
                trace!("GamepadHandler: Pad event: {:?}", event);
                let Some(which) = event_which!(event) else {
                    warn!("Failed to get 'which' from gamepad event: {:?}", event);
                    return;
                };

                // Look up our device ID for this SDL instance ID
                let Some(&device_id) = self.sdl_id_to_device.get(&which) else {
                    warn!("No device found for SDL ID {} in pad event", which);
                    return;
                };

                if let Ok(mut guard) = self
                    .state
                    .lock()
                    .map_err(|e| error!("Failed to lock state for pad event: {}", e))
                    && let Some(device) = guard.devices.iter_mut().find(|d| d.id == device_id)
                {
                    let Some(gamepad) = self.sdl_devices.get(&which).and_then(|devs| {
                        devs.iter()
                            .find(|d| matches!(d, SDLDevice::Gamepad(_)))
                            .and_then(|d| match d {
                                SDLDevice::Gamepad(p) => Some(p),
                                _ => None,
                            })
                    }) else {
                        warn!("No SDL gamepad found for SDL ID {}", which);
                        return;
                    };

                    if device.steam_handle == 0 {
                        let handle = get_gamepad_steam_handle(gamepad);
                        if handle != 0 {
                            device.steam_handle = handle;
                            self.viiper.create_device(device);
                            return;
                        }
                        warn!(
                            "Device {} (SDL ID {}) has no steam handle in pad event",
                            device_id, which
                        );
                        return;
                    }

                    device.state.update_from_sdl_gamepad(gamepad);

                    self.viiper.update_device_state(device);
                } else {
                    warn!("Device {} not found in state for pad event", device_id);
                }
            }
        }
    }

    pub fn on_viiper_event(&mut self, event: ViiperEvent) {
        match event {
            ViiperEvent::ServerDisconnected { device_id } => {
                // Needs to be done here to avoid deadlock
                self.viiper.remove_device(device_id);

                let Ok(mut guard) = self.state.lock() else {
                    error!("Failed to lock state for VIIPER disconnect handling");
                    return;
                };
                if let Some(device) = guard.devices.iter_mut().find(|d| d.id == device_id) {
                    device.viiper_device = None;
                    debug!(
                        "Cleared VIIPER device for {} due to server disconnect",
                        device_id
                    );
                }
            }
            ViiperEvent::DeviceCreated {
                device_id,
                viiper_device,
            } => {
                let Ok(mut guard) = self.state.lock() else {
                    error!("Failed to lock state for VIIPER device created handling");
                    return;
                };
                let Some(device) = guard.devices.iter_mut().find(|d| d.id == device_id) else {
                    warn!("Received created event for unknown device ID {}", device_id);
                    return;
                };
                device.viiper_device = Some(viiper_device);
                self.viiper.connect_device(device);
                self.request_redraw();
            }
            ViiperEvent::DeviceConnected { device_id } => {
                let Ok(mut guard) = self.state.lock() else {
                    error!("Failed to lock state for VIIPER device connected handling");
                    return;
                };
                let Some(device) = guard.devices.iter_mut().find(|d| d.id == device_id) else {
                    warn!(
                        "Received connected event for unknown device ID {}",
                        device_id
                    );
                    return;
                };
                device.viiper_connected = true;
                self.request_redraw();
            }
            ViiperEvent::DeviceRumble { device_id, l, r } => {
                warn!("Received rumble for device {}, l={}, r={}", device_id, l, r);

                let Ok(guard) = self.state.lock() else {
                    error!("Failed to lock state for rumble");
                    return;
                };

                let Some(device) = guard.devices.iter().find(|d| d.id == device_id) else {
                    warn!("Device {} not found for rumble", device_id);
                    return;
                };

                // Get the first SDL ID associated with this device
                let Some(&sdl_id) = device.sdl_ids.iter().next() else {
                    warn!("Device {} has no SDL IDs for rumble", device_id);
                    return;
                };
                drop(guard); // Release lock before accessing sdl_devices

                let Some(devices) = self.sdl_devices.get_mut(&sdl_id) else {
                    warn!(
                        "No SDL devices found for SDL ID {} (device {}) in output event",
                        sdl_id, device_id
                    );
                    return;
                };

                let Some(gamepad) = devices.iter_mut().find_map(|d| match d {
                    SDLDevice::Gamepad(p) => Some(p),
                    _ => None,
                }) else {
                    warn!(
                        "No SDL gamepad found for SDL ID {} (device {}) in output event",
                        sdl_id, device_id
                    );
                    return;
                };
                if let Err(e) = gamepad.set_rumble(l as u16 * 257, r as u16 * 257, 10000) {
                    warn!("Failed to set rumble for device {}: {}", device_id, e);
                }
                self.request_redraw();
            }
            ViiperEvent::ErrorCreateDevice { device_id } => {
                error!("Failed to create VIIPER device {}", device_id);
                self.request_redraw();
            }
            ViiperEvent::ErrorConnectDevice { device_id } => {
                error!("Failed to connect VIIPER device {}", device_id);
                self.request_redraw();
            }
        }
    }
}

fn handle_existing_device_connect(
    viiper: &mut super::viiper_bridge::ViiperBridge,
    device: &mut Device,
    steam_handle: u64,
    sdl_id: u32,
) {
    if device.steam_handle == 0 && steam_handle != 0 {
        device.steam_handle = steam_handle;
        info!(
            "Updated steam handle for device {} (SDL ID {}) to {}",
            device.id, sdl_id, steam_handle
        );

        if device.viiper_device.is_some() {
            debug!(
                "Device {} already has a VIIPER device; skipping creation",
                device.id
            );
            return;
        }

        info!(
            "Connecting device {} upon connect with steam handle {}",
            device.id, steam_handle
        );
        viiper.create_device(device);
    }
}

fn handle_new_device(
    viiper: &mut super::viiper_bridge::ViiperBridge,
    guard: &mut std::sync::MutexGuard<'_, super::State>,
    device_id: u64,
    sdl_id: u32,
    steam_handle: u64,
    is_joystick: bool,
    sdl_device_info: SdlDeviceInfo,
) {
    let mut sdl_ids = std::collections::HashSet::new();
    sdl_ids.insert(sdl_id);

    let device = Device {
        id: device_id,
        sdl_ids,
        steam_handle,
        state: DeviceState::default(),
        sdl_device_infos: vec![sdl_device_info],
        ..Default::default()
    };
    if is_joystick {
        guard.devices.push(device);
        info!(
            "Added Joystick device {} (SDL ID {}); Steam Handle: {}",
            device_id, sdl_id, steam_handle
        );
        return;
    }

    if steam_handle != 0 {
        info!(
            "Connecting device {} (SDL ID {}) upon connect with steam handle {}",
            device_id, sdl_id, steam_handle
        );
        viiper.create_device(&device);
    }
    guard.devices.push(device);
}
