mod surface_data;

use crate::SampleRequirements;
use crate::graphics_context::surface_data::SurfaceData;
use anyhow::Context;
use std::sync::Arc;
use std::time::Instant;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, PowerPreference,
    Queue, RequestAdapterOptions, TextureUsages,
};
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub struct GraphicsContext {
    pub window: Arc<Window>,
    #[allow(unused)]
    instance: Instance,
    #[allow(unused)]
    adapter: Adapter,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface_data: SurfaceData,
    pub last_frame_time: Instant,
}

impl GraphicsContext {
    pub fn new(
        event_loop: &ActiveEventLoop,
        window_title: &str,
        sample_requirements: &SampleRequirements,
    ) -> anyhow::Result<Self> {
        let mut window_attributes = Window::default_attributes()
            .with_title(window_title)
            .with_min_inner_size(LogicalSize::new(1, 1));

        if let Some(primary_monitor) = event_loop.primary_monitor() {
            let monitor_size = primary_monitor.size();
            let window_size = PhysicalSize::new(monitor_size.width / 2, monitor_size.height / 2);
            let mut window_position = PhysicalPosition::new(
                (monitor_size.width - window_size.width) / 2,
                (monitor_size.height - window_size.height) / 2,
            );
            window_position.y -= (window_size.height as f32 * 0.1) as u32;
            window_attributes = window_attributes.with_inner_size(window_size);
            window_attributes = window_attributes.with_position(window_position);
        }

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .context("Failed to create window")?,
        );

        // Instance
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        // Surface
        let surface = instance
            .create_surface(window.clone())
            .context("Failed to create instance")?;

        // Adapter
        let adapter =
            futures::executor::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .context("Failed to get adapter")?;

        println!(
            "Selected adapter: {}, {}",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        // Device and Queue
        let (device, queue) = futures::executor::block_on(
            adapter.request_device(
                sample_requirements
                    .device_descriptor
                    .as_ref()
                    .unwrap_or(&DeviceDescriptor::default()),
            ),
        )
        .context("Failed to request device")?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let mut surface_data = SurfaceData::new(
            window.clone(),
            surface,
            &adapter,
            device.clone(),
            TextureUsages::RENDER_ATTACHMENT,
        );
        surface_data.configure(
            window.inner_size().width.max(1),
            window.inner_size().height.max(1),
        );

        window.request_redraw();
        Ok(GraphicsContext {
            window,
            instance,
            adapter,
            device,
            queue,
            surface_data,
            last_frame_time: Instant::now(),
        })
    }

    pub fn window_aspect(&self) -> f32 {
        self.window.inner_size().width as f32 / self.window.inner_size().height as f32
    }
}
