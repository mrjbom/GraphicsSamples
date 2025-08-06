pub mod camera;
pub mod graphics_context;

use crate::camera::Camera;
use crate::graphics_context::GraphicsContext;
use std::time::{Duration, Instant};
use wgpu::{DeviceDescriptor, TextureView};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop};
use winit::window::WindowId;

pub struct SampleApp<S: SampleTrait + Sized> {
    sample_name: &'static str,
    sample_requirements: SampleRequirements,
    event_loop: Option<EventLoop<()>>,
    graphics_context: Option<GraphicsContext>,
    sample_context: Option<S>,
    mouse_in_window: bool,
}

impl<S: SampleTrait + Sized> SampleApp<S> {
    pub fn new(sample_name: &'static str, sample_requirements: SampleRequirements) -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.listen_device_events(DeviceEvents::WhenFocused);

        Self {
            sample_name,
            sample_requirements,
            event_loop: Some(event_loop),
            graphics_context: None,
            sample_context: None,
            mouse_in_window: false,
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
                log::error!("Failed to create graphics context");
                for err in err.chain() {
                    log::error!("{err}");
                }
                event_loop.exit();
                return;
            }
        };

        let sample_context = S::new(&graphics_context);
        let sample_context = match sample_context {
            Ok(sample_context) => sample_context,
            Err(err) => {
                log::error!("Failed to create sample context");
                for err in err.chain() {
                    log::error!("{err}");
                }
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
                let sample_context = self.sample_context.as_mut().unwrap();

                let now = Instant::now();
                let frame_time_delta = now - graphics_context.last_frame_time;
                graphics_context.last_frame_time = now;

                let (surface_texture, surface_texture_view) =
                    graphics_context.surface_data.acquire();

                sample_context.render(graphics_context, surface_texture_view, frame_time_delta);
                graphics_context.window.pre_present_notify();
                surface_texture.present();

                let graphics_context = self.graphics_context.as_ref().unwrap();
                graphics_context.window.request_redraw();
            }

            WindowEvent::Resized(new_size) => {
                let graphics_context = self.graphics_context.as_mut().unwrap();

                graphics_context
                    .surface_data
                    .configure(new_size.width, new_size.height);
                graphics_context.window.request_redraw();
            }

            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let Some(sample_context) = self.sample_context.as_mut() {
                    if let Some(camera) = sample_context.process_camera_input() {
                        camera.process_keyboard(event.physical_key, event.state);
                    }
                }
            }
            WindowEvent::CursorEntered { device_id: _ } => {
                self.mouse_in_window = true;
            }

            WindowEvent::CursorLeft { device_id: _ } => {
                self.mouse_in_window = false;
            }

            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if self.mouse_in_window {
                    if let Some(sample_context) = self.sample_context.as_mut() {
                        if let Some(camera) = sample_context.process_camera_input() {
                            camera.process_mouse_input(button, state);
                        }
                    }
                }
            }

            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion {
                delta: (delta_x, delta_y),
            } => {
                if self.mouse_in_window {
                    if let Some(sample_context) = self.sample_context.as_mut() {
                        if let Some(camera) = sample_context.process_camera_input() {
                            camera.process_mouse_motion(delta_x, delta_y);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub trait SampleTrait: Sized {
    fn new(graphics_context: &GraphicsContext) -> anyhow::Result<Self>;

    fn render(
        &mut self,
        graphics_context: &GraphicsContext,
        surface_texture_view: TextureView,
        frame_time_delta: Duration,
    );

    fn process_camera_input(&mut self) -> Option<&mut Camera> {
        None
    }
}

#[derive(Default)]
pub struct SampleRequirements {
    pub device_descriptor: Option<DeviceDescriptor<'static>>,
}
