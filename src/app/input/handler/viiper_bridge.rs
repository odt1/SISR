use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use sdl3::event::EventSender;
use tracing::{info, trace, warn};
use viiper_client::devices::xbox360;
use viiper_client::{DeviceStream, ViiperClient};

use crate::app::input::device::Device;

/// Custom SDL event pushed when VIIPER server disconnects a device
pub enum ViiperEvent {
    Disconnect { device_id: u32 },
    Connect { device_id: u32, todo: () },
}

pub(super) struct ViiperBridge {
    client: Option<ViiperClient>,
    streams: Arc<Mutex<HashMap<u32, DeviceStream>>>,
    sdl_waker: Arc<Mutex<Option<EventSender>>>,
    async_handle: tokio::runtime::Handle,
}

impl ViiperBridge {
    pub fn new(
        viiper_address: Option<SocketAddr>,
        sdl_waker: Arc<Mutex<Option<EventSender>>>,
        async_handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            client: match viiper_address {
                Some(addr) => Some(ViiperClient::new(addr)),
                None => {
                    warn!("No VIIPER address provided; VIIPER integration disabled");
                    None
                }
            },
            streams: Arc::new(Mutex::new(HashMap::new())),
            sdl_waker,
            async_handle,
        }
    }

    pub fn create_device(&self, device: &mut Device, bus_id: &mut Option<u32>) -> Result<()> {
        let current_bus_id = self.ensure_bus(bus_id)?;

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("No VIIPER client available"))?;

        let response = client
            .bus_device_add(
                current_bus_id,
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

    pub fn connect_device(&mut self, device: &mut Device) -> Result<()> {
        let viiper_dev = device
            .viiper_device
            .as_ref()
            .ok_or_else(|| anyhow!("Device has no VIIPER device"))?;

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("No VIIPER client available"))?;

        let mut dev_stream = client
            .connect_device(viiper_dev.bus_id, &viiper_dev.dev_id)
            .map_err(|e| anyhow!("Failed to connect VIIPER device: {}", e))?;

        let device_id = device.id;
        let sdl_waker = self.sdl_waker.clone();

        dev_stream
            .on_disconnect(move || {
                info!("VIIPER server disconnected device {}", device_id);

                if let Ok(guard) = sdl_waker.lock()
                    && let Some(sender) = &*guard
                {
                    let _ = sender.push_custom_event(ViiperEvent::Disconnect { device_id });
                }
            })
            .map_err(|e| anyhow!("Failed to register disconnect callback: {}", e))?;

        dev_stream
            .on_output(|reader| {
                let mut buf = [0u8; xbox360::OUTPUT_SIZE];
                reader.read_exact(&mut buf)?;
                // TODO: Forward rumble to SDL haptic
                trace!("Rumble data: {:?}", buf);
                Ok(())
            })
            .map_err(|e| anyhow!("Failed to register output callback: {}", e))?;

        if let Ok(mut streams_guard) = self.streams.lock() {
            streams_guard.insert(device.id, dev_stream);
        }
        info!("Connected VIIPER device {:?}", device.viiper_device);
        Ok(())
    }

    fn ensure_bus(&self, bus_id: &mut Option<u32>) -> Result<u32> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("No VIIPER client available"))?;

        if let Some(id) = *bus_id {
            let buses = client
                .bus_list()
                .map_err(|e| anyhow!("Failed to list VIIPER buses: {}", e))?;

            if buses.buses.contains(&id) {
                return Ok(id);
            }
            warn!("Bus {} no longer exists, recreating...", id);
        }

        let response = client
            .bus_create(None)
            .map_err(|e| anyhow!("Failed to create VIIPER bus: {}", e))?;

        info!("Created VIIPER bus with ID {}", response.bus_id);
        *bus_id = Some(response.bus_id);
        Ok(response.bus_id)
    }

    pub fn remove_device(&mut self, which: u32) {
        if let Ok(mut streams_guard) = self.streams.lock() {
            if streams_guard.remove(&which).is_some() {
                info!("Disconnected VIIPER device with ID {}", which);
            } else {
                warn!("No VIIPER device to disconnect found with ID {}", which);
            }
        }
    }
}
