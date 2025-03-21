use log::LevelFilter;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

fn main() {
    logger_init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).expect("Failed to run app");
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .expect("Failed to create window"),
        );

        self.state = Some(State {
            window: window.clone(),
        });

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
}

struct State {
    window: Arc<Window>,
}

fn logger_init() {
    let mut builder = env_logger::builder();
    // Levels
    if cfg!(debug_assertions) {
        // Enable all message levels
        builder.filter_level(LevelFilter::Trace);
    } else {
        // Enable error, warn, info levels
        // debug and trace disabled
        builder.filter_level(LevelFilter::Info);
    }
    builder.format_timestamp(None);
    builder.format_target(false);
    builder.init();
}
