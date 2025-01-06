use std::{sync::Arc, time::Instant};
use gloo::render;
use rand::distributions::uniform;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub struct WgpuState<'window> {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'window>,
    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub num_vertices: Option<u32>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_indices: Option<u32>,
    pub uniform_buffer: Option<wgpu::Buffer>,
    pub uniform_bind_group: Option<wgpu::BindGroup>,
    pub instance_buffer: Option<wgpu::Buffer>,
    pub start_time: Option<std::time::Instant>
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
    pub fn new_wasm(window: Arc<Window>) -> WgpuState<'window> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            }
        );
        let surface = instance.create_surface(window).unwrap();

        Self {
            instance,
            surface,
            device: None,
            queue: None,
            config: None,
            size,
            render_pipeline: None,
            vertex_buffer: None,
            num_vertices: None,
            index_buffer: None,
            num_indices: None,
            uniform_buffer: None,
            uniform_bind_group: None,
            instance_buffer: None,
            start_time: None
        }
    }

    pub async fn wasm_runtime_setup(&mut self) {
        let adapter = self.instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&self.surface),
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

        let surface_caps  = self.surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: self.size.width.max(1),
            height: self.size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        self.surface.configure(&device,&config);

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

        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.render_pipeline = Some(render_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.num_vertices = Some(vertices.len() as u32);
        self.index_buffer = Some(index_buffer);
        self.num_indices = Some(indices.len() as u32);
        self.uniform_buffer = Some(uniform_buffer);
        self.uniform_bind_group = Some(uniform_bind_group);
        self.instance_buffer = Some(instance_buffer);
        self.start_time = Some(Instant::now());
    
    }   

    pub fn native_new(window: Arc<Window>) -> WgpuState<'window> {
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
            instance,
            surface,
            device: Some(device),
            queue: Some(queue),
            config: Some(config),
            size,
            render_pipeline: Some(render_pipeline),
            vertex_buffer: Some(vertex_buffer),
            num_vertices: Some(vertices.len() as u32),
            index_buffer: Some(index_buffer),
            num_indices: Some(indices.len() as u32),
            uniform_buffer: Some(uniform_buffer),
            uniform_bind_group: Some(uniform_bind_group),
            instance_buffer: Some(instance_buffer),
            start_time: Some(Instant::now())
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            if let Some(config) = &mut self.config {
                config.width = new_size.width;
                config.height = new_size.height;
            }
            self.surface.configure(&self.device.as_ref().unwrap(), &self.config.as_ref().unwrap());
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
        let time = now.duration_since(self.start_time.clone().unwrap()).as_secs_f32();
        if let (
            Some(queue),
            Some(device),
            Some(uniform_buffer),
            Some(render_pipeline),
            Some(uniform_bind_group),
            Some(vertex_buffer),
            Some(index_buffer),
            Some(instance_buffer),
            Some(num_indices),
        ) = (
            &mut self.queue,
            &self.device,
            &self.uniform_buffer,
            &self.render_pipeline,
            &self.uniform_bind_group,
            &self.vertex_buffer,
            &self.index_buffer,
            &self.instance_buffer,
            self.num_indices
        ) {
            queue.write_buffer(
                uniform_buffer,
                0,
                bytemuck::cast_slice(&[Uniforms { time }])
            );
            let mut encoder = device.create_command_encoder(
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
                                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                    store: wgpu::StoreOp::Store
                                }
                            })
                        ],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None
                    }
                );
    
                render_pass.set_pipeline(render_pipeline);
                render_pass.set_bind_group(0, uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..),wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..num_indices,0,0..Self::STAR_INSTANCE_COUNT);
            }
            queue.submit(std::iter::once(encoder.finish()));
        }

        output.present();

        Ok(())
    }

    fn create_star_instances() -> Vec<Instance> {
        use rand::Rng;

        let mut rng: Box<dyn rand::RngCore> = if cfg!(target_arch = "wasm32") {
            // wasm32の場合はrandが使えないので、乱数を固定値にする
            use rand::SeedableRng;
            Box::new(rand::rngs::SmallRng::seed_from_u64(0))
        } else {
            // デスクトップの場合は乱数を初期化
            Box::new(rand::thread_rng())
        };
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