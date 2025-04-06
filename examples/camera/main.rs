use bytemuck::{Pod, Zeroable};
use graphics_samples::camera::Camera;
use graphics_samples::graphics_context::GraphicsContext;
use graphics_samples::{SampleApp, SampleRequirements, SampleTrait};
use nalgebra::Matrix4;
use std::borrow::Cow;
use std::time::Duration;
use wgpu::naga::ShaderStage;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, DeviceDescriptor, Features, FragmentState, FrontFace, Limits, LoadOp,
    Maintain, Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    PushConstantRange, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    SurfaceTexture, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let sample_requirements = SampleRequirements {
        device_descriptor: Some(DeviceDescriptor {
            required_features: Features::PUSH_CONSTANTS,
            required_limits: Limits {
                // Matrix needs 64 bytes
                max_push_constant_size: 64,
                ..Default::default()
            },
            ..Default::default()
        }),
    };
    let mut sample_app = SampleApp::<SampleContext>::new("Camera", sample_requirements);

    sample_app.run();
}

struct SampleContext {
    camera: Camera,
    vertex_buffer: Buffer,
    render_pipeline: RenderPipeline,
}

impl SampleTrait for SampleContext {
    fn new(graphics_context: &GraphicsContext) -> anyhow::Result<Self> {
        let camera = Camera::new(
            [0.0, 0.0, -1.0],
            [0.0, 0.0, 1.0],
            1.0,
            1.0,
            graphics_context.window.current_monitor(),
        );

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
                    layout(push_constant) uniform PushConstants {
                        mat4 mvp_matrix;
                    } p_c;
                    out vec4 out_Color;

                    void main() {
                        gl_Position = p_c.mvp_matrix * vec4(in_Position, 1.0);
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
                position: [0.0, 0.5, 0.25],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.25],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.25],
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
                    layout: Some(&graphics_context.device.create_pipeline_layout(
                        &PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &[],
                            push_constant_ranges: &[PushConstantRange {
                                stages: ShaderStages::VERTEX,
                                range: 0..64,
                            }],
                        },
                    )),
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
            camera,
            vertex_buffer,
            render_pipeline,
        })
    }

    fn render(
        &mut self,
        graphics_context: &GraphicsContext,
        surface_texture: SurfaceTexture,
        surface_texture_view: TextureView,
        frame_time_delta: Duration,
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

            // Camera
            // nalgebra creates a projection matrix for OpenGL, but it is not suitable for wgpu because:
            // 1. Incorrect Z-axis direction
            // 2. Incorrect depth clip space
            // OpenGL: [-1,1], wgpu: [0,1]
            #[rustfmt::skip]
            let projection_correction = Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, -0.5, 0.5,
                0.0, 0.0, -1.0, 0.0,
            );
            let projection_matrix = Matrix4::new_perspective(
                graphics_context.window_aspect(),
                45.0_f32.to_radians(),
                0.1,
                100.0,
            );
            let projection_matrix = projection_correction * projection_matrix;
            let view_matrix = self.camera.calculate_view_matrix(frame_time_delta);
            let model_matrix = Matrix4::<f32>::identity();
            let mvp_matrix = projection_matrix * view_matrix * model_matrix;
            render_pass.set_push_constants(
                ShaderStages::VERTEX,
                0,
                bytemuck::bytes_of(&mvp_matrix),
            );

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

    fn process_camera_input(&mut self) -> Option<&mut Camera> {
        Some(&mut self.camera)
    }
}
