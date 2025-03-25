mod surface_data;

use crate::graphics_context::surface_data::SurfaceData;
use std::sync::Arc;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, MemoryHints,
    PowerPreference, Queue, RequestAdapterOptions, TextureUsages,
};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub struct GraphicsContext {
    pub window: Arc<Window>,
    instance: Instance,
    adapter: Adapter,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface_data: SurfaceData,
}

impl GraphicsContext {
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let window = Arc::new(event_loop.create_window(Window::default_attributes())?);
        window.set_min_inner_size(Some(LogicalSize::new(1, 1)));

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
        println!(
            "Selected adapter: {}, {}",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        // Device and Queue
        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                memory_hints: MemoryHints::MemoryUsage,
                ..Default::default()
            },
            None,
        ))?;
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
        })
    }
}
