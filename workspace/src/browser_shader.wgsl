struct Uniforms {
    time: f32,
    padding: vec3<f32>
}

struct InstanceInput {
    @location(2) position: vec2<f32>,
    @location(3) scale: f32,
    @location(4) initialRotation: f32,
    @location(5) speed: vec2<f32>,
    @location(6) rotationSpeed: f32,
}

@binding(0) @group(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertexMain(
    @location(0) position: vec2<f32>,
    @builtin(instance_index) instanceIdx: u32,
    instance: InstanceInput,
) -> @builtin(position) vec4<f32> {
    // アニメーションの計算
    let rotation = instance.initialRotation + uniforms.time * instance.rotationSpeed;
    var pos = instance.position + instance.speed * uniforms.time;
    
    // 画面端でのラップ処理
    pos = vec2<f32>(
        fract((pos.x + 1.0) / 2.0) * 2.0 - 1.0,
        fract((pos.y + 1.0) / 2.0) * 2.0 - 1.0
    );

    // 回転行列の作成
    let c = cos(rotation);
    let s = sin(rotation);
    let rotMatrix = mat2x2<f32>(
        c, -s,
        s, c
    );

    // 頂点の変換
    let scaledPos = position * instance.scale;
    let rotatedPos = rotMatrix * scaledPos;
    let finalPos = rotatedPos + pos;

    return vec4<f32>(finalPos, 0.0, 1.0);
}

@fragment
fn fragmentMain() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}