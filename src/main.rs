use std::cell::RefCell;
use std::sync::Arc;
use wgpu::{
    Adapter, Backends, CompositeAlphaMode, Device, DeviceDescriptor, Instance, InstanceDescriptor,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceCapabilities,
    SurfaceConfiguration, TextureFormat, TextureUsages,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

fn main() {
    env_logger::builder().format_timestamp(None).init();

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
        if self.graphics_context.is_some() {
            return;
        }

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
        let graphics_context = self.graphics_context.as_mut().unwrap();

        match event {
            WindowEvent::RedrawRequested => {}
            WindowEvent::Resized(_new_size) => {
                graphics_context.surface_data.configure();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

struct GraphicsContext {
    window: Arc<Window>,
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Queue,
    surface_data: SurfaceData,
}

impl GraphicsContext {
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let window = Arc::new(event_loop.create_window(Window::default_attributes())?);

        // Instance
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        // Surface
        let surface = instance.create_surface(window.clone())?;

        // Adapter
        let adapter =
            futures::executor::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }));
        if adapter.is_none() {
            anyhow::bail!("Failed to get adapter");
        }
        let adapter = adapter.unwrap();
        log::info!(
            "Selected adapter: {}, {}",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        // Device and Queue
        let (device, queue) = futures::executor::block_on(
            adapter.request_device(&DeviceDescriptor::default(), None),
        )?;
        let device = Arc::new(device);

        let surface_data = SurfaceData::new(
            window.clone(),
            surface,
            &adapter,
            device.clone(),
            TextureUsages::RENDER_ATTACHMENT,
        );
        surface_data.configure();

        window.request_redraw();
        Ok(GraphicsContext {
            window,
            instance,
            adapter,
            device,
            queue,
            surface_data,
        })
    }
}

struct SurfaceData {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Arc<Device>,
    capabilities: SurfaceCapabilities,
    // Configuration data
    surface_configuration: RefCell<SurfaceConfiguration>,
}

impl SurfaceData {
    pub fn new(
        window: Arc<Window>,
        surface: Surface<'static>,
        adapter: &Adapter,
        device: Arc<Device>,
        usage: TextureUsages,
    ) -> Self {
        let capabilities = surface.get_capabilities(adapter);
        assert!(adapter.is_surface_supported(&surface));

        // [0] - preferred
        let format = capabilities.formats[0];

        let present_mode = 'present_mode: {
            let preferences = vec![
                PresentMode::Mailbox,
                PresentMode::FifoRelaxed,
                PresentMode::Fifo,
            ];
            for preferred_present_mode in preferences.iter() {
                if capabilities.present_modes.contains(preferred_present_mode) {
                    break 'present_mode *preferred_present_mode;
                }
            }
            PresentMode::default()
        };

        // Hint, will always be clamped to the supported range
        let desired_maximum_frame_latency = 3;

        let alpha_mode: CompositeAlphaMode = CompositeAlphaMode::Auto;

        // View formats of the same format as the texture are always allowed
        let view_formats = vec![format];

        // SurfaceConfiguration
        let surface_configuration = SurfaceConfiguration {
            usage,
            format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode,
            desired_maximum_frame_latency,
            alpha_mode,
            view_formats,
        };

        Self {
            window,
            surface,
            device,
            capabilities,
            surface_configuration: RefCell::new(surface_configuration),
        }
    }

    pub fn configure(&self) {
        self.surface_configuration.borrow_mut().width = self.window.inner_size().width;
        self.surface_configuration.borrow_mut().height = self.window.inner_size().height;

        self.surface
            .configure(&self.device, &self.surface_configuration.borrow());
    }
}
