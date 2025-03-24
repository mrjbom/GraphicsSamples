use std::sync::Arc;
use wgpu::{
    Adapter, CompositeAlphaMode, Device, PresentMode, Surface, SurfaceCapabilities,
    SurfaceConfiguration, SurfaceError, SurfaceTexture, TextureAspect, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};
use winit::window::Window;

pub struct SurfaceData {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Arc<Device>,
    capabilities: SurfaceCapabilities,
    pub surface_configuration: SurfaceConfiguration,
    suboptimal: bool,
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
            let preferences = vec![PresentMode::FifoRelaxed, PresentMode::Fifo];
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
            surface_configuration,
            suboptimal: false,
        }
    }

    pub fn configure(&mut self, width: u32, height: u32) {
        self.surface_configuration.width = width;
        self.surface_configuration.height = height;

        self.surface
            .configure(&self.device, &self.surface_configuration);
    }

    pub fn acquire(&mut self) -> (SurfaceTexture, TextureView) {
        if self.suboptimal {
            self.configure(
                self.window.inner_size().width.max(1),
                self.window.inner_size().height.max(1),
            );
        }
        self.suboptimal = false;

        let surface_texture = self.surface.get_current_texture();
        let surface_texture = match surface_texture {
            Ok(frame) => frame,
            // If we timed out, just try again
            Err(SurfaceError::Timeout) => self.surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture"),
            Err(
                // If the surface is outdated, or was lost, reconfigure it.
                SurfaceError::Outdated
                | SurfaceError::Lost
                | SurfaceError::Other
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try
                | SurfaceError::OutOfMemory,
            ) => {
                self.configure(self.window.inner_size().width.max(1), self.window.inner_size().height.max(1));
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture")
            }
        };
        self.suboptimal = surface_texture.suboptimal;

        let texture_view = surface_texture.texture.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(self.surface_configuration.view_formats[0]),
            dimension: Some(TextureViewDimension::D2),
            usage: Some(self.surface_configuration.usage),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        (surface_texture, texture_view)
    }
}
