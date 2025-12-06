use tracing::{error, info, warn};

use super::EventHandler;

impl EventHandler {
    pub fn ignore_device(&mut self, device_id: u64) {
        let Ok(mut guard) = self.state.lock() else {
            error!(
                "Failed to acquire event handler state lock to ignore device {}",
                device_id
            );
            return;
        };
        if let Some(device) = guard.devices.get(&device_id) {
            info!("Ignoring device {}: {}", device_id, device.id);
            guard.devices.remove(&device_id);
        } else {
            warn!("Tried to ignore unknown device ID {}", device_id);
        }
    }

    pub fn connect_viiper_device(&mut self, device_id: u64) {
        let Ok(mut guard) = self.state.lock() else {
            error!(
                "Failed to acquire event handler state lock to connect VIIPER device {}",
                device_id
            );
            return;
        };
        let Some(device) = guard.devices.get_mut(&device_id) else {
            warn!(
                "Tried to connect VIIPER for unknown device ID {}",
                device_id
            );
            return;
        };
        if device.viiper_connected || device.viiper_device.is_some() {
            warn!("Device ID {} is already connected to VIIPER", device_id);
            return;
        }
        if device.steam_handle == 0 {
            warn!(
                "Device ID {} does not have a valid Steam handle; cannot connect to VIIPER",
                device_id
            );
            return;
        }
        self.viiper.create_device(device);
        self.request_redraw();
    }

    pub fn disconnect_viiper_device(&mut self, device_id: u64) {
        let Ok(mut guard) = self.state.lock() else {
            error!(
                "Failed to acquire event handler state lock to disconnect VIIPER device {}",
                device_id
            );
            return;
        };
        let Some(device) = guard.devices.get_mut(&device_id) else {
            warn!(
                "Tried to disconnect VIIPER for unknown device ID {}",
                device_id
            );
            return;
        };
        if !device.viiper_connected || device.viiper_device.is_none() {
            warn!("Device ID {} is not connected to VIIPER", device_id);
            return;
        }
        self.viiper.remove_device(device_id);
    }
}
