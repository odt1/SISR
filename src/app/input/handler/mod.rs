mod events;
mod gui;
mod viiper;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tracing::{debug, error, warn};
use viiper_client::{DeviceStream, ViiperClient};
use winit::event_loop::EventLoopProxy;

use crate::app::{
    gui::dispatcher::GuiDispatcher,
    input::device::{Device, SDLDevice},
    window::RunnerEvent,
};

pub struct EventHandler {
    winit_waker: Arc<Mutex<Option<EventLoopProxy<RunnerEvent>>>>,
    gui_dispatcher: Arc<Mutex<Option<GuiDispatcher>>>,
    sdl_joystick: sdl3::JoystickSubsystem,
    sdl_gamepad: sdl3::GamepadSubsystem,
    sdl_devices: HashMap<u32, Vec<SDLDevice>>,
    viiper_streams: HashMap<u32, DeviceStream>,
    viiper_client: Option<ViiperClient>,
    state: Arc<Mutex<State>>,
}

pub(super) struct State {
    devices: Vec<Device>,
    viiper_address: Option<SocketAddr>,
    viiper_bus: Option<u32>,
}

impl EventHandler {
    pub fn new(
        winit_waker: Arc<Mutex<Option<EventLoopProxy<RunnerEvent>>>>,
        gui_dispatcher: Arc<Mutex<Option<GuiDispatcher>>>,
        viiper_address: Option<SocketAddr>,
    ) -> Self {
        let sdl = sdl3::init()
            .inspect_err(|e| error!("failed to get handle on SDL: {}", e))
            .unwrap();
        let state = Arc::new(Mutex::new(State {
            devices: Vec::new(),
            viiper_address,
            viiper_bus: None,
        }));
        let res = Self {
            winit_waker,
            gui_dispatcher,
            sdl_joystick: sdl.joystick().unwrap(),
            sdl_gamepad: sdl.gamepad().unwrap(),
            sdl_devices: HashMap::new(),
            state: state.clone(),
            viiper_streams: HashMap::new(),
            viiper_client: match viiper_address {
                Some(addr) => Some(ViiperClient::new(addr)),
                None => {
                    warn!("No VIIPER address provided; VIIPER integration disabled");
                    None
                }
            },
        };
        if let Ok(dispatcher_guard) = res.gui_dispatcher.lock()
            && let Some(dispatcher) = &*dispatcher_guard
        {
            debug!("SDL loop GUI dispatcher initialized");
            dispatcher.register_callback(move |ctx| {
                if let Ok(mut guard) = state.lock() {
                    let state = &mut *guard;
                    EventHandler::on_draw(state, ctx);
                }
            });
        }
        res
    }
}
