struct ViewUniform {
    viewport: vec2<f32>,
    scale: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> view: ViewUniform;

struct VertexInput {
    @location(0) rect: vec4<f32>,
    @location(1) fill: vec4<f32>,
    @location(2) border: vec4<f32>,
    @location(3) clip: vec4<f32>,
    @location(4) radius_and_border: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) fill: vec4<f32>,
    @location(3) border: vec4<f32>,
    @location(4) clip: vec4<f32>,
    @location(5) radius_and_border: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];
    let logical_position = input.rect.xy + corner * input.rect.zw;
    let physical_position = logical_position * view.scale;

    var output: VertexOutput;
    output.position = vec4<f32>(
        physical_position.x / view.viewport.x * 2.0 - 1.0,
        1.0 - physical_position.y / view.viewport.y * 2.0,
        0.0,
        1.0,
    );
    output.local_position = corner * input.rect.zw;
    output.size = input.rect.zw;
    output.fill = input.fill;
    output.border = input.border;
    output.clip = input.clip * view.scale;
    output.radius_and_border = input.radius_and_border;
    return output;
}

fn rounded_box_distance(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let safe_radius = clamp(radius, 0.0, min(half_size.x, half_size.y));
    let q = abs(point - half_size) - half_size + vec2<f32>(safe_radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - safe_radius;
}

fn coverage(distance: f32) -> f32 {
    let antialias_width = max(fwidth(distance), 0.0001);
    return 1.0 - smoothstep(-antialias_width, antialias_width, distance);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = input.position.xy;
    if pixel.x < input.clip.x || pixel.y < input.clip.y ||
       pixel.x >= input.clip.z || pixel.y >= input.clip.w {
        discard;
    }

    let outer = coverage(rounded_box_distance(
        input.local_position,
        input.size,
        input.radius_and_border.x,
    ));
    let border_width = min(
        max(input.radius_and_border.y, 0.0),
        min(input.size.x, input.size.y) * 0.5,
    );

    var inner = outer;
    if border_width > 0.0 {
        inner = coverage(rounded_box_distance(
            input.local_position - vec2<f32>(border_width),
            max(input.size - vec2<f32>(border_width * 2.0), vec2<f32>(0.0)),
            max(input.radius_and_border.x - border_width, 0.0),
        ));
    }

    let fill_coverage = inner;
    let border_coverage = max(outer - inner, 0.0);
    let fill_alpha = input.fill.a * fill_coverage;
    let border_alpha = input.border.a * border_coverage;
    let alpha = fill_alpha + border_alpha;
    let rgb = input.fill.rgb * fill_alpha + input.border.rgb * border_alpha;
    return vec4<f32>(rgb, alpha);
}

