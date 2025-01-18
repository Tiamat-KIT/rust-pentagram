use crate::state::WgpuState;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    position: [f32; 2],
    scale: f32,
    initial_rotation: f32,
    speed: [f32; 2],
    rotation_speed: f32,
}

pub fn create_star_instances() -> Vec<Instance> {
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
    
    for _ in 0..WgpuState::STAR_INSTANCE_COUNT {
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

pub fn get_instance_buffer(device: &wgpu::Device,instances: &Vec<Instance>) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX
        }
    );
}

pub fn get_instance_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
    static ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        2 => Float32x2,
        3 => Float32,
        4 => Float32,
        5 => Float32x2,
        6 => Float32
    ];

    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}