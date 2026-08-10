// GUI / HUD shader — orthographic, no lighting

struct PushConstants {
    screen_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> pc: PushConstants;
@group(1) @binding(0) var t_gui: texture_2d<f32>;
@group(1) @binding(1) var s_gui: sampler;

struct VertexIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv:  vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    // convert pixel coords to NDC
    let ndc = (in.pos / pc.screen_size) * 2.0 - vec2<f32>(1.0, 1.0);
    out.clip_pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_gui, s_gui, in.uv);
    if color.a < 0.05 { discard; }
    return color;
}
