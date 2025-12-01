use std::sync::MutexGuard;

use super::{EventHandler, State};
use crate::app::input::device::Device;
use anyhow::{Result, anyhow};
use serde::de;
use tracing::{info, warn};

impl EventHandler {
    pub(super) fn create_viiper_device(&mut self, device: &mut Device) -> Result<()> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| anyhow!("Failed to lock state: {}", e))?;

        self.create_viiper_device_with_guard(device, &mut guard)
    }

    pub(super) fn create_viiper_device_with_guard(
        &self,
        device: &mut Device,
        guard: &mut MutexGuard<'_, State>,
    ) -> Result<()> {
        let bus_id = match guard.viiper_bus {
            Some(id) => id,
            None => self.create_viiper_bus(guard)?,
        };

        let client = self
            .viiper_client
            .as_ref()
            .ok_or_else(|| anyhow!("No VIIPER client available"))?;

        let response = client
            .bus_device_add(
                bus_id,
                &viiper_client::types::DeviceCreateRequest {
                    r#type: Some(device.viiper_type.clone()),
                    id_vendor: None,
                    id_product: None,
                },
            )
            .map_err(|e| anyhow!("Failed to create VIIPER device: {}", e))?;

        info!("Created VIIPER device with {:?}", response);
        device.viiper_device = Some(response);
        Ok(())
    }

    pub(super) fn connect_viiper_device(&mut self, device: &mut Device) -> Result<()> {
        let client = self
            .viiper_client
            .as_mut()
            .ok_or_else(|| anyhow!("No VIIPER client available"))?;

        connect_viiper_device_with_impl(device, client, &mut self.viiper_streams)
    }

    pub(super) fn create_viiper_bus(&self, guard: &mut MutexGuard<'_, State>) -> Result<u32> {
        if guard.viiper_bus.is_some() {
            warn!("VIIPER bus already created; Recreating");
        }
        let client = self
            .viiper_client
            .as_ref()
            .ok_or_else(|| anyhow!("No VIIPER client available"))?;

        let response = client
            .bus_create(None)
            .map_err(|e| anyhow!("Failed to create VIIPER bus: {}", e))?;
        info!("Created VIIPER bus with ID {}", response.bus_id);
        guard.viiper_bus = Some(response.bus_id);
        Ok(response.bus_id)
    }
}

fn connect_viiper_device_with_impl(
    device: &mut Device,
    viiper_client: &mut viiper_client::ViiperClient,
    viiper_streams: &mut std::collections::HashMap<u32, viiper_client::DeviceStream>,
) -> Result<()> {
    let viiper_dev = device
        .viiper_device
        .as_ref()
        .ok_or_else(|| anyhow!("Device has no VIIPER device"))?;

    let dev_stream = viiper_client
        .connect_device(viiper_dev.bus_id, &viiper_dev.dev_id)
        .map_err(|e| anyhow!("Failed to connect VIIPER device: {}", e))?;

    viiper_streams.insert(device.id, dev_stream);
    info!("Connected VIIPER device {:?}", device.viiper_device);
    Ok(())
}
