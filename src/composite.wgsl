// Vertex shader

struct ScreenQuad {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: ScreenQuad,
) -> MeshFragment {
    var out: MeshFragment;
    out.clip_position = vec4(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

// Fragment shader

// TODO:
//      Make this one group with 3 texture buffers
//      and 1 sample buffer
@group(0) @binding(0)
var normal_buffer: texture_2d<f32>;
@group(0) @binding(1)
var normal_sample: sampler;
@group(1) @binding(0)
var albedo_buffer: texture_2d<f32>;
@group(1) @binding(1)
var albedo_sample: sampler;
@group(2) @binding(0)
var position_buffer: texture_2d<f32>;
@group(2) @binding(1)
var position_sample: sampler;

struct MeshFragment {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct PointLight {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) constant: f32,
    @location(3) linear: f32,
    @location(4) quadratic: f32
}

fn to_vec3(v: vec4<f32>) -> vec3<f32>{
    return vec3(v[0], v[1], v[2]);
}

@fragment
fn fs_main(in: MeshFragment) -> @location(0) vec4<f32> {
    // Light Calculations

    let normal_sample = textureSample(normal_buffer, normal_sample, in.tex_coords);
    let albedo_sample = textureSample(albedo_buffer, albedo_sample, in.tex_coords);
    let position_sample = textureSample(position_buffer, position_sample, in.tex_coords);

    let normal = vec3(normal_sample.x, normal_sample.y, normal_sample.z);
    let color = to_vec3(albedo_sample);
    let position = to_vec3(position_sample);
    let specular = albedo_sample[2];

    var light: PointLight;
    light.position = vec3(0.0, 0.0, 0.0);
    light.color = vec3(1.0, 1.0, 1.0);

    light.constant = 1.0;
    light.linear = 0.22;
    light.quadratic = 0.20;

    let to_light = position - light.position;
    let light_dir = normalize(to_light);
    let light_dist = distance(position, light.position);

    // magic formula from Davis' class
    let atten = 1.0/(light.constant + light.linear * light_dist + light.quadratic * light_dist * light_dist);

    return vec4(color * atten, 0.0);
}
