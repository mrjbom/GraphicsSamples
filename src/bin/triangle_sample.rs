use bytemuck::{Pod, Zeroable};
use graphics_samples::graphics_context::GraphicsContext;
use graphics_samples::{SampleApp, SampleTrait};
use std::borrow::Cow;
use wgpu::naga::ShaderStage;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, FragmentState, FrontFace, LoadOp, Maintain, Operations,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    StoreOp, SurfaceTexture, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let mut sample_app = SampleApp::<SampleContext>::new();

    sample_app.run();
}

struct SampleContext {
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
    vertex_buffer: Buffer,
    render_pipeline: RenderPipeline,
}

impl SampleTrait for SampleContext {
    fn new(graphics_context: &GraphicsContext) -> anyhow::Result<Self> {
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

    fn render(
        &mut self,
        graphics_context: &GraphicsContext,
        surface_texture: SurfaceTexture,
        surface_texture_view: TextureView,
    ) {
        let mut command_encoder = graphics_context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
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
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }
        let command_buffer = command_encoder.finish();
        let submission_index = graphics_context.queue.submit([command_buffer]);
        graphics_context
            .device
            .poll(Maintain::WaitForSubmissionIndex(submission_index));
        graphics_context.window.pre_present_notify();
        surface_texture.present();
    }
}
