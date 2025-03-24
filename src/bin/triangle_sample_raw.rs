use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use std::sync::Arc;
use wgpu::naga::ShaderStage;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Adapter, Backends, Buffer, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, FragmentState,
    FrontFace, Instance, InstanceDescriptor, LoadOp, MemoryHints, Operations, PowerPreference,
    PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface, SurfaceCapabilities,
    SurfaceConfiguration, SurfaceError, SurfaceTexture, TextureAspect, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
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
    app_context: Option<AppContext>,
}

impl App {
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_texture: SurfaceTexture,
        surface_texture_view: TextureView,
    ) {
        let graphics_context = self.graphics_context.as_mut().unwrap();
        let app_context = self.app_context.as_mut().unwrap();

        let mut command_encoder =
            device.create_command_encoder(&CommandEncoderDescriptor::default());
        {
            let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_vertex_buffer(0, app_context.vertex_buffer.slice(..));
            render_pass.set_pipeline(&app_context.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }
        let command_buffer = command_encoder.finish();
        queue.submit([command_buffer]);
        graphics_context.window.pre_present_notify();
        surface_texture.present();
    }
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

        let app_context = AppContext::new(&graphics_context);
        let app_context = match app_context {
            Ok(app_context) => app_context,
            Err(err) => {
                log::error!("Failed to create app context: {err:#}");
                event_loop.exit();
                return;
            }
        };

        self.graphics_context = Some(graphics_context);
        self.app_context = Some(app_context);
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
                let device = graphics_context.device.clone();
                let queue = graphics_context.queue.clone();
                let (surface_texture, surface_texture_view) =
                    graphics_context.surface_data.acquire();

                self.render(&device, &queue, surface_texture, surface_texture_view);
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

struct GraphicsContext {
    window: Arc<Window>,
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface_data: SurfaceData,
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
        log::info!(
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

struct SurfaceData {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Arc<Device>,
    capabilities: SurfaceCapabilities,
    surface_configuration: SurfaceConfiguration,
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

struct AppContext {
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
    vertex_buffer: Buffer,
    render_pipeline: RenderPipeline,
}

impl AppContext {
    pub fn new(graphics_context: &GraphicsContext) -> anyhow::Result<Self> {
        // Shaders
        let vertex_shader = graphics_context
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Glsl {
                    shader: Cow::Borrowed(
                        r#"
                    #version 460

                    layout(location = 0) in vec3 in_Position;
                    layout(location = 1) in vec4 in_Color;
                    out vec4 out_Color;

                    void main() {
                        gl_Position = vec4(in_Position, 1.0);
                        out_Color = in_Color;
                    }
                "#,
                    ),
                    stage: ShaderStage::Vertex,
                    defines: Default::default(),
                },
            });
        let fragment_shader =
            graphics_context
                .device
                .create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: ShaderSource::Glsl {
                        shader: Cow::Borrowed(
                            r#"
                    #version 460

                    in vec4 out_Color;
                    out vec4 frag_Color;

                    void main() {
                        frag_Color = out_Color;
                    }
                "#,
                        ),
                        stage: ShaderStage::Fragment,
                        defines: Default::default(),
                    },
                });

        // Vertex buffer
        #[repr(C)]
        #[derive(Pod, Zeroable, Clone, Copy)]
        struct Vertex {
            position: [f32; 3],
            color: [f32; 3],
        }

        let vertexes = vec![
            Vertex {
                position: [0.0, 0.5, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                color: [0.0, 0.0, 1.0],
            },
        ];
        let vertex_buffer = graphics_context
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertexes),
                usage: BufferUsages::VERTEX,
            });

        // Render Pipeline
        let render_pipeline =
            graphics_context
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: None,
                    layout: None,
                    vertex: VertexState {
                        module: &vertex_shader,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        buffers: &[VertexBufferLayout {
                            array_stride: size_of::<Vertex>() as BufferAddress,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &[
                                VertexAttribute {
                                    format: VertexFormat::Float32x3,
                                    offset: 0,
                                    shader_location: 0,
                                },
                                VertexAttribute {
                                    format: VertexFormat::Float32x3,
                                    offset: 4 * 3,
                                    shader_location: 1,
                                },
                            ],
                        }],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Cw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: Default::default(),
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(FragmentState {
                        module: &fragment_shader,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        targets: &[Some(ColorTargetState {
                            format: graphics_context
                                .surface_data
                                .surface_configuration
                                .view_formats[0],
                            blend: None,
                            write_mask: ColorWrites::all(),
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        Ok(Self {
            vertex_shader,
            fragment_shader,
            vertex_buffer,
            render_pipeline,
        })
    }
}
