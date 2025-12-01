use std::sync::{Arc, Mutex};

use crate::app::{
    App,
    gui::dispatcher::GuiDispatcher,
    input::{handler::EventHandler, sdl_hints},
    window::RunnerEvent,
};
use sdl3::sys::events;
use sdl3::{
    event::{Event, EventSender},
    gamepad::Gamepad,
};
use tracing::{Level, debug, error, span, trace, warn};
use winit::event_loop::EventLoopProxy;

//

pub fn get_gamepad_steam_handle(pad: &Gamepad) -> u64 {
    use sdl3::sys::gamepad::SDL_GetGamepadSteamHandle;
    let instance_id = pad.id().unwrap_or(0);
    if instance_id == 0 {
        trace!("Cannot get steam handle for device with invalid instance ID 0");
        return 0;
    }

    unsafe {
        // Extract the raw SDL_Gamepad pointer from the opened gamepad
        // sdl3-0.16.2\src\sdl3\gamepad.rs:745
        #[repr(C)]
        struct GamepadRaw {
            _subsystem: [u8; std::mem::size_of::<sdl3::GamepadSubsystem>()],
            raw: *mut sdl3::sys::gamepad::SDL_Gamepad,
        }

        let gamepad_raw: &GamepadRaw = std::mem::transmute(pad);
        if gamepad_raw.raw.is_null() {
            warn!(
                "Gamepad raw pointer is null for instance ID {}",
                instance_id
            );
            return 0;
        }

        SDL_GetGamepadSteamHandle(gamepad_raw.raw)
    }
}

macro_rules! event_which {
    ($event:expr) => {
        match $event {
            Event::JoyAxisMotion { which, .. }
            | Event::JoyBallMotion { which, .. }
            | Event::JoyHatMotion { which, .. }
            | Event::JoyButtonDown { which, .. }
            | Event::JoyButtonUp { which, .. }
            | Event::JoyDeviceAdded { which, .. }
            | Event::JoyDeviceRemoved { which, .. }
            // FUCK RUSTFMT
            | Event::ControllerAxisMotion { which, .. }
            | Event::ControllerButtonDown { which, .. }
            | Event::ControllerButtonUp { which, .. }
            | Event::ControllerDeviceAdded { which, .. }
            | Event::ControllerDeviceRemoved { which, .. }
            | Event::ControllerDeviceRemapped { which, .. }
            | Event::ControllerTouchpadDown { which, .. }
            | Event::ControllerTouchpadMotion { which, .. }
            | Event::ControllerTouchpadUp { which, .. }
            | Event::ControllerSensorUpdated { which, .. } => Some(*which),
            _ => None,
        }
    };
}

pub struct InputLoop {
    sdl_waker: Arc<Mutex<Option<EventSender>>>,
    winit_waker: Arc<Mutex<Option<EventLoopProxy<RunnerEvent>>>>,
    gui_dispatcher: Arc<Mutex<Option<GuiDispatcher>>>,
    somedummy: Arc<Mutex<SomeTodoDummyDebugState>>,
    async_handle: tokio::runtime::Handle,
}

#[derive(Default)]
struct SomeTodoDummyDebugState {
    counter: u64,
}

impl InputLoop {
    pub fn new(
        sdl_waker: Arc<Mutex<Option<EventSender>>>,
        winit_waker: Arc<Mutex<Option<EventLoopProxy<RunnerEvent>>>>,
        gui_dispatcher: Arc<Mutex<Option<GuiDispatcher>>>,
        async_handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            sdl_waker,
            winit_waker,
            gui_dispatcher,
            somedummy: Arc::new(Mutex::new(SomeTodoDummyDebugState::default())),
            async_handle,
        }
    }

    pub fn run(&mut self, viiper_address: Option<std::net::SocketAddr>) {
        trace!("SDL_Init");

        let sdl = match sdl3::init() {
            Ok(sdl) => sdl,
            Err(e) => {
                error!("Failed to initialize SDL: {}", e);
                return;
            }
        };
        for (hint_name, hint_value) in sdl_hints::SDL_HINTS {
            match sdl3::hint::set(hint_name, hint_value) {
                true => trace!("Set SDL hint {hint_name}={hint_value}"),
                false => warn!("Failed to set SDL hint {hint_name}={hint_value}"),
            }
        }

        let _ = sdl
            .joystick()
            .inspect_err(|e| error!("Failed to initialize SDL joystick subsystem: {e}"));
        let _ = sdl
            .gamepad()
            .inspect_err(|e| error!("Failed to initialize SDL gamepad subsystem: {e}"));

        match sdl.event() {
            Ok(event_subsystem) => {
                if let Err(e) =
                    event_subsystem.register_custom_event::<super::handler::ViiperEvent>()
                {
                    error!("Failed to register VIIPER disconnect event: {}", e);
                }

                match self.sdl_waker.lock() {
                    Ok(mut guard) => {
                        *guard = Some(event_subsystem.event_sender());
                    }
                    Err(e) => {
                        error!("Failed to set SDL event sender: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to get SDL event subsystem: {}", e);
            }
        }

        let mut event_pump = match sdl.event_pump() {
            Ok(pump) => pump,
            Err(e) => {
                error!("Failed to get SDL event pump: {}", e);
                return;
            }
        };

        if let Ok(dispatcher_guard) = self.gui_dispatcher.lock()
            && let Some(dispatcher) = &*dispatcher_guard
        {
            debug!("SDL loop GUI dispatcher initialized");
            let state = self.somedummy.clone();
            dispatcher.register_callback(move |ctx| {
                if let Ok(mut guard) = state.lock() {
                    let state = &mut *guard;
                    InputLoop::on_draw(state, ctx);
                }
            });
        }

        match self.run_loop(&mut event_pump, viiper_address) {
            Ok(_) => {}
            Err(_) => {
                error!("SDL loop exited with error");
            }
        }

        trace!("SDL loop exiting");
        App::shutdown(None, Some(&self.winit_waker));
    }

    fn run_loop(
        &mut self,
        event_pump: &mut sdl3::EventPump,
        viiper_address: Option<std::net::SocketAddr>,
    ) -> Result<(), ()> {
        let span = span!(Level::INFO, "sdl_loop");

        let mut pad_event_handler = EventHandler::new(
            self.sdl_waker.clone(),
            self.winit_waker.clone(),
            self.gui_dispatcher.clone(),
            viiper_address,
            self.async_handle.clone(),
        );
        trace!("SDL loop starting");
        loop {
            let mut redraw = false;
            let meh = event_pump.wait_event();

            for event in std::iter::once(meh).chain(event_pump.poll_iter()) {
                if let Ok(mut guard) = self.somedummy.lock() {
                    let state = &mut *guard;
                    state.counter += 1;
                }
                match event {
                    Event::Quit { .. } => {
                        tracing::event!(parent: &span, Level::INFO, event = ?event, "Quit event received");
                        return Ok(());
                    }
                    Event::JoyDeviceAdded { .. } | Event::ControllerDeviceAdded { .. } => {
                        pad_event_handler.on_pad_added(&event);
                        redraw = true;
                    }
                    Event::JoyDeviceRemoved { .. } | Event::ControllerDeviceRemoved { .. } => {
                        pad_event_handler.on_pad_removed(&event);
                        redraw = true;
                    }
                    Event::Unknown { type_, .. } => match events::SDL_EventType(type_) {
                        events::SDL_EVENT_GAMEPAD_STEAM_HANDLE_UPDATED => {
                            pad_event_handler.on_steam_handle_updated(&event);
                        }
                        events::SDL_EVENT_GAMEPAD_UPDATE_COMPLETE
                        | events::SDL_EVENT_JOYSTICK_UPDATE_COMPLETE => {
                            pad_event_handler.on_pad_event(&event);
                        }
                        _ => {
                            tracing::event!(parent: &span, Level::TRACE, event = ?event, "SDL event");
                        }
                    },
                    _ => {
                        if event.is_user_event()
                            && let Some(viiper_event) =
                                event.as_user_event_type::<super::handler::ViiperEvent>()
                        {
                            pad_event_handler.on_viiper_event(viiper_event);
                            continue;
                        }

                        if event.is_joy() || event.is_controller() {
                            pad_event_handler.on_pad_event(&event);
                        } else {
                            tracing::event!(parent: &span, Level::TRACE, event = ?event, "SDL event");
                        }
                    }
                }
            }

            if redraw {
                self.request_redraw();
            }
        }
    }

    fn request_redraw(&self) {
        if let Ok(guard) = self.winit_waker.lock()
            && let Some(proxy) = &*guard
        {
            match proxy.send_event(RunnerEvent::Redraw()) {
                Ok(_) => {}
                Err(e) => {
                    warn!("Failed to request GUI redraw: {}", e);
                }
            }
        }
    }

    fn on_draw(state: &mut SomeTodoDummyDebugState, ctx: &egui::Context) {
        egui::Window::new("SDL Input Loop").show(ctx, |ui| {
            ui.label("SDL Input Loop is running!");
            ui.label(format!("Event count: {}", state.counter));
        });
    }
}
