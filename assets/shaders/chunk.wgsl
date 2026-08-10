// Chunk vertex+fragment shader
// Each vertex: position (vec3f), uv (vec2f), normal (vec3f), ao (f32)

struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var t_terrain: texture_2d<f32>;
@group(1) @binding(1) var s_terrain: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) normal:   vec3<f32>,
    @location(3) ao:       f32,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:      vec2<f32>,
    @location(1) light:   f32,
    @location(2) ao:      f32,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip_pos = cam.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    // simple directional lighting
    let sun = normalize(vec3<f32>(0.6, 1.0, 0.4));
    out.light = clamp(dot(in.normal, sun), 0.3, 1.0);
    out.ao = in.ao;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_terrain, s_terrain, in.uv);
    if tex.a < 0.1 { discard; }
    let brightness = in.light * (0.5 + 0.5 * in.ao);
    return vec4<f32>(tex.rgb * brightness, tex.a);
}
