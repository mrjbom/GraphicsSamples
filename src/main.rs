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
    graphics_context: Option<GraphicsContext>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let graphics_context = GraphicsContext::new(event_loop);
        let graphics_context = match graphics_context {
            Ok(graphics_context) => graphics_context,
            Err(err) => {
                log::error!("Failed to create graphics context: {err:#}");
                event_loop.exit();
                return;
            }
        };
        self.graphics_context = Some(graphics_context);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.graphics_context.is_none() {
            return;
        }
        let _graphics_context = self.graphics_context.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
}

struct GraphicsContext {
    window: Arc<Window>,
}

impl GraphicsContext {
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let window = Arc::new(event_loop.create_window(Window::default_attributes())?);

        window.request_redraw();

        Ok(GraphicsContext { window })
    }
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
