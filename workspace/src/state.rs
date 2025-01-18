use wgpu::util::DeviceExt;
use winit::{event::WindowEvent, window::Window};
use core::f64;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;


struct FrameStats {
    min_time: f64, // ナノ秒単位で記録
    max_time: f64,
    total_time: f64,
    frame_count: u64,
}

impl FrameStats {
    fn new() -> Self {
        Self {
            min_time: f64::MAX,
            max_time: 0.0,
            total_time: 0.0,
            frame_count: 0,
        }
    }

    fn update(&mut self, frame_time: f64) {
        if frame_time < self.min_time {
            self.min_time = frame_time;
        }
        if frame_time > self.max_time {
            self.max_time = frame_time;
        }
        self.total_time += frame_time;
        self.frame_count += 1;
    }

    fn average_time(&self) -> f64 {
        if self.frame_count == 0 {
            0.0
        } else {
            self.total_time / self.frame_count as f64
        }
    }

    fn display_stats(&self) {
        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                println!("Min Time: {} sec", self.min_time);
                println!("Max Time: {} sec", self.max_time);
                println!("Average Time: {} sec", self.average_time());
                println!("Total Frames: {}", self.frame_count);
                print!("----------------------------------\n");
            } else {
                use wasm_bindgen::JsValue;
                web_sys::console::log_1(
                    &JsValue::from(
                        format!(
                            "Min Time: {} sec\nMax Time: {} sec\nAverage Time: {} sec\nTotal Frames: {}\n----------------------------------",
                            self.min_time,
                            self.max_time,
                            self.average_time(),
                            self.frame_count
                        ).as_str()
                    )
                );
            }
        }
        
    }
}


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
    pub start_time: Option<Instant>,
    pub frame_stats: FrameStats,

    pub window: &'window Window,
}




impl<'window> WgpuState<'window> {
    pub const STAR_INSTANCE_COUNT: u32 = 500;
    pub async fn new(window: &'window Window) -> WgpuState<'window> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor {
                // WebAssemblyのときはWebGL、そうでないならWebGPU
                #[cfg(target_arch = "wasm32")]
                backends: wgpu::Backends::SECONDARY, // BROWSER_WEBGPUは動く。
                #[cfg(not(target_arch = "wasm32"))]
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

        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                println!("Adapter: {:?}", adapter.get_info());
                // Limitations of the adapter
            } else {
                use wasm_bindgen::JsValue;
                web_sys::console::log_1(&JsValue::from_str(format!("Adapter: {:?}", adapter.get_info()).as_str()));
            }
        }
        let device_result = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(), // デフォルトの制限を使用
                ..Default::default()
            },
            None,
        ).await;

        let (device, queue) = match device_result {
            Ok(result) => result,
            Err(err) => {
                panic!("Device error: {:?}", err);
            }
        };
        
        
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

        // シェーダーを読み込む。ビルド先によってファイルを変える
        

        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                #[cfg(not(target_arch = "wasm32"))]
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("./shader.wgsl").into()
                ),
                #[cfg(target_arch = "wasm32")]
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("./browser_shader.wgsl").into()
                )
            }
        );

        let uniform_buffer = crate::uniform::Uniforms::get_uniform_buffer(&device);

        let (uniform_bind_group_layout,uniform_bind_group) = crate::uniform::Uniforms::get_uniform_bind_groups(&device,&uniform_buffer);

        let render_pipeline= crate::uniform::Uniforms::get_render_setting(&device,&uniform_bind_group_layout,&shader,&config);  

        let (vertices,indices) = Self::create_star_vertices();
        let vertex_buffer = crate::vertex::Vertex::get_vertex_buffer(&device,&vertices);

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX
            }
        );

        let instances = crate::instance::create_star_instances();
        let instance_buffer = crate::instance::get_instance_buffer(&device, &instances);
        let mut stats = FrameStats::new();

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
            start_time: Some(Instant::now()),
            frame_stats: stats,
            window: window
        }
    }


    pub fn native_new(window: &'window Window) -> WgpuState<'window> {
        pollster::block_on(Self::new(window))
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

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {}

    fn create_star_vertices() -> (Vec<crate::vertex::Vertex>, Vec<u16>) {
        let num_points = 5;
        let vertices = crate::vertex::Vertex::get_vertices();
    
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
        let render_before_time = Instant::now();
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
                bytemuck::cast_slice(&[crate::uniform::Uniforms::new(time)])
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
        let render_after_time = Instant::now();
        if cfg!(not(target_arch = "wasm32")) {
            // レンダリングにかかった時間を出力
            let render_time = render_after_time.duration_since(render_before_time).as_secs_f64();
            self.frame_stats.update(render_time);
            if self.frame_stats.frame_count % 60 == 0 {
                self.frame_stats.display_stats();
            }
        } else {
            // かなり細かい精度で出力する
            let render_time = render_after_time.duration_since(render_before_time).as_secs_f64();
            self.frame_stats.update(render_time);
            if self.frame_stats.frame_count % 60 == 0 {
                self.frame_stats.display_stats();
            }
        }
        Ok(())
    }
}