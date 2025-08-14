

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@group(1) @binding(0)
var<uniform> mouse_position: vec4<f32>;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position * vec3<f32>(1.0, 1.0, -1.0), 1.0);
    out.clip_position = vec4<f32>(out.clip_position.xy * 0.2 + (mouse_position.xy * 2 - 1) * vec2<f32>(1, -1), out.clip_position.zw);
    out.normal = model.normal * vec3<f32>(1.0, 1.0, -1.0);
    return out;
}


@group(0) @binding(0)
var t: texture_2d<f32>;
@group(0) @binding(1)
var s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t, s, (in.normal.xy * vec2<f32>(1, 1) + 1.0) * 0.5);
}

