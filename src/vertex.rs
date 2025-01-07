#[repr(C)]
#[derive(Debug, Copy, Clone,bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2]
}

impl Vertex {
    pub fn get_vertices() -> Vec<Vertex> {
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
        vertices
    }
    
    pub fn get_vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    format: wgpu::VertexFormat::Float32x2,
                    shader_location: 0,
                }
            ]
        }
    }

    pub fn get_vertex_buffer(device: &wgpu::Device,vertices: &Vec<Self>) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        vertex_buffer
    }
}

