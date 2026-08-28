struct ViewUniform {
    viewport: vec2<f32>,
    scale: f32,
    padding: f32,
}

@group(0) @binding(0)
var<uniform> view: ViewUniform;

@group(1) @binding(0)
var image_texture: texture_2d<f32>;

@group(1) @binding(1)
var image_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) clip: vec4<f32>,
    @location(3) mask: vec4<f32>,
    @location(4) radius_grayscale_opacity_padding: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) logical_position: vec2<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) mask: vec4<f32>,
    @location(4) @interpolate(flat) radius_grayscale_opacity_padding: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[input.vertex_index];
    let logical_position = input.rect.xy + corner * input.rect.zw;
    let physical_position = logical_position * view.scale;
    let ndc = vec2<f32>(
        physical_position.x / view.viewport.x * 2.0 - 1.0,
        1.0 - physical_position.y / view.viewport.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.uv = input.uv.xy + corner * input.uv.zw;
    output.logical_position = logical_position;
    output.clip = input.clip;
    output.mask = input.mask;
    output.radius_grayscale_opacity_padding = input.radius_grayscale_opacity_padding;
    return output;
}

fn rounded_rect_distance(position: vec2<f32>, size: vec2<f32>, radius_value: f32) -> f32 {
    let radius = min(max(radius_value, 0.0), min(size.x, size.y) * 0.5);
    let centered = abs(position - size * 0.5) - (size * 0.5 - vec2<f32>(radius));
    return length(max(centered, vec2<f32>(0.0)))
        + min(max(centered.x, centered.y), 0.0)
        - radius;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.logical_position.x < input.clip.x
        || input.logical_position.y < input.clip.y
        || input.logical_position.x >= input.clip.z
        || input.logical_position.y >= input.clip.w
    {
        discard;
    }

    let mask_position = input.logical_position - input.mask.xy;
    let distance = rounded_rect_distance(
        mask_position,
        input.mask.zw,
        input.radius_grayscale_opacity_padding.x,
    );
    let antialias = max(fwidth(distance), 0.001);
    let coverage = clamp(0.5 - distance / antialias, 0.0, 1.0);
    if coverage <= 0.0 {
        discard;
    }

    let sampled = textureSample(image_texture, image_sampler, input.uv);
    let luminance = dot(sampled.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let color = mix(
        sampled.rgb,
        vec3<f32>(luminance),
        clamp(input.radius_grayscale_opacity_padding.y, 0.0, 1.0),
    );
    let opacity = clamp(input.radius_grayscale_opacity_padding.z, 0.0, 1.0);
    return vec4<f32>(color * sampled.a, sampled.a) * coverage * opacity;
}
