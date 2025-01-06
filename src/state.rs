use std::{sync::Arc, time::Instant};
use wgpu::{core::instance, util::DeviceExt};
use winit::window::Window;

pub struct WgpuState<'window> {
    pub surface: wgpu::Surface<'window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub instance_buffer: wgpu::Buffer,
    pub start_time: std::time::Instant
}

#[repr(C)]
#[derive(Debug, Copy, Clone,bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2]
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    position: [f32; 2],
    scale: f32,
    initial_rotation: f32,
    speed: [f32; 2],
    rotation_speed: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
}
impl<'window> WgpuState<'window> {
    const STAR_INSTANCE_COUNT: u32 = 500;
    pub fn new(window: Arc<Window>) -> WgpuState<'window> {
        pollster::block_on(WgpuState::new_async(window))
    }
    pub async fn new_async(window: Arc<Window>) -> Self {
        let size = window
            .inner_size();
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            }
        );

        let surface = instance.create_surface(window).unwrap();
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false
            }
        ).await.unwrap();
        let (device,queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                ..Default::default()
            },
            None
        ).await.unwrap();
        device.on_uncaptured_error(Box::new(|error| {
            panic!("Device error: {:?}", error);
        }));

        let surface_caps  = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device,&config);

        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("./shader.wgsl").into()
                )
            }
        );

        let uniform_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: None,
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }]
            }
        );

        let uniform_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding()
                }]
            }
        );

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[]
            }
        );

        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertexMain"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0 => Float32x2]
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![
                                2 => Float32x2,
                                3 => Float32,
                                4 => Float32,
                                5 => Float32x2,
                                6 => Float32
                            ],
                        }
                    ]
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragmentMain"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL
                    })],
                    compilation_options: Default::default()
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None
            }
        );

        let (vertices,indices) = Self::create_star_vertices();
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX
            }
        );

        let instances = Self::create_star_instances();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            num_vertices: vertices.len() as u32,
            index_buffer,
            num_indices: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            start_time: std::time::Instant::now(),
            instance_buffer
        }

    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn create_star_vertices() -> (Vec<Vertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let num_points = 5;
        let radius = 1.0;
        
        // 中心点を最初に追加
        vertices.push(Vertex { position: [0.0, 0.0] });
        
        // 外側の頂点を計算
        for i in 0..num_points {
            let angle = (i as f32 * 2.0 * std::f32::consts::PI / num_points as f32) 
                - std::f32::consts::FRAC_PI_2;
            vertices.push(Vertex {
                position: [
                    radius * angle.cos(),
                    radius * angle.sin(),
                ]
            });
        }
    
        // 五芒星を形成するインデックス
        // 頂点0は中心点、頂点1-5は外周の点
        let mut indices = Vec::new();
        
        // 五芒星の三角形を形成
        for i in 0..num_points {
            let current = 1 + i;
            let next = 1 + ((i + 2) % num_points); // 2つ先の頂点と接続
            
            // 三角形を追加（中心点と2つの外周点で1つの三角形を形成）
            indices.extend_from_slice(&[
                0,                    // 中心点
                current as u16,       // 現在の頂点
                next as u16,         // 2つ先の頂点
            ]);
        }
    
        (vertices, indices)
    }

    pub fn render(&mut self) -> Result<(),wgpu::SurfaceError> {
        let output = self.surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let now = Instant::now();
        let time = now.duration_since(self.start_time).as_secs_f32();

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms { time }])
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: None
            }
        );
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store
                            }
                        })
                    ],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None
                }
            );

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices,0,0..Self::STAR_INSTANCE_COUNT);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn create_star_instances() -> Vec<Instance> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut instances = Vec::new();
        
        for _ in 0..Self::STAR_INSTANCE_COUNT {
            instances.push(Instance {
                position: [
                    rng.gen_range(-0.9..0.9),
                    rng.gen_range(-0.9..0.9),
                ],
                scale: rng.gen_range(0.02..0.05),  // スケールを少し大きく
                initial_rotation: rng.gen_range(0.0..std::f32::consts::PI * 2.0),
                speed: [
                    rng.gen_range(-0.3..0.3),      // 移動速度を調整
                    rng.gen_range(-0.3..0.3),
                ],
                rotation_speed: rng.gen_range(0.5..2.0),  // 回転速度を調整
            });
        }
        instances
    }
}