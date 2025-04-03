pub mod camera;
pub mod graphics_context;

use crate::graphics_context::GraphicsContext;
use wgpu::{DeviceDescriptor, SurfaceTexture, TextureView};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

pub struct SampleApp<S: SampleTrait + Sized> {
    sample_name: &'static str,
    sample_requirements: SampleRequirements,
    event_loop: Option<EventLoop<()>>,
    graphics_context: Option<GraphicsContext>,
    sample_context: Option<S>,
}

impl<S: SampleTrait + Sized> SampleApp<S> {
    pub fn new(sample_name: &'static str, sample_requirements: SampleRequirements) -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Poll);

        Self {
            sample_name,
            sample_requirements,
            event_loop: Some(event_loop),
            graphics_context: None,
            sample_context: None,
        }
    }

    pub fn run(&mut self) {
        self.event_loop
            .take()
            .unwrap()
            .run_app(self)
            .expect("Failed to run sample app");
    }
}

impl<S: SampleTrait> ApplicationHandler for SampleApp<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics_context.is_some() {
            return;
        }

        let graphics_context =
            GraphicsContext::new(event_loop, self.sample_name, &self.sample_requirements);
        let graphics_context = match graphics_context {
            Ok(graphics_context) => graphics_context,
            Err(err) => {
                log::error!("Failed to create graphics context: {err:#}");
                event_loop.exit();
                return;
            }
        };

        let sample_context = S::new(&graphics_context);
        let sample_context = match sample_context {
            Ok(sample_context) => sample_context,
            Err(err) => {
                log::error!("Failed to create sample context: {err:#}");
                event_loop.exit();
                return;
            }
        };

        self.graphics_context = Some(graphics_context);
        self.sample_context = Some(sample_context);
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

        match event {
            WindowEvent::RedrawRequested => {
                let graphics_context = self.graphics_context.as_mut().unwrap();
                let (surface_texture, surface_texture_view) =
                    graphics_context.surface_data.acquire();

                let sample_context = self.sample_context.as_mut().unwrap();
                sample_context.render(graphics_context, surface_texture, surface_texture_view);

                let graphics_context = self.graphics_context.as_ref().unwrap();
                graphics_context.window.request_redraw();
            }

            WindowEvent::Resized(new_size) => {
                let graphics_context = self.graphics_context.as_mut().unwrap();
                graphics_context
                    .surface_data
                    .configure(new_size.width.max(1), new_size.height.max(1));
                graphics_context.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

pub trait SampleTrait: Sized {
    fn new(graphics_context: &GraphicsContext) -> anyhow::Result<Self>;

    fn render(
        &mut self,
        graphics_context: &GraphicsContext,
        surface_texture: SurfaceTexture,
        surface_texture_view: TextureView,
    );
}

#[derive(Default)]
pub struct SampleRequirements {
    pub device_descriptor: Option<DeviceDescriptor<'static>>,
}
