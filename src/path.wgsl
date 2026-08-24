struct ViewUniform {
    viewport: vec2<f32>,
    scale: f32,
    _padding: f32,
};

struct PathPaint {
    transform: vec4<f32>,
    clip: vec4<f32>,
    gradient_line: vec4<f32>,
    color_0: vec4<f32>,
    color_1: vec4<f32>,
    stops_mode: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view: ViewUniform;
@group(1) @binding(0) var<storage, read> paints: array<PathPaint>;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) barycentric: vec3<f32>,
    @location(2) edge_mask: vec3<f32>,
    @location(3) paint_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) logical_position: vec2<f32>,
    @location(1) barycentric: vec3<f32>,
    @location(2) @interpolate(flat) edge_mask: vec3<f32>,
    @location(3) @interpolate(flat) paint_index: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let paint = paints[input.paint_index];
    let logical = input.position * paint.transform.xy + paint.transform.zw;
    let physical = logical * view.scale;
    let clip_position = vec2<f32>(
        physical.x / view.viewport.x * 2.0 - 1.0,
        1.0 - physical.y / view.viewport.y * 2.0,
    );
    var output: VertexOutput;
    output.position = vec4<f32>(clip_position, 0.0, 1.0);
    output.logical_position = logical;
    output.barycentric = input.barycentric;
    output.edge_mask = input.edge_mask;
    output.paint_index = input.paint_index;
    return output;
}

fn linear_to_srgb_component(value: f32) -> f32 {
    let clamped = clamp(value, 0.0, 1.0);
    if clamped <= 0.0031308 {
        return clamped * 12.92;
    }
    return 1.055 * pow(clamped, 1.0 / 2.4) - 0.055;
}

fn srgb_to_linear_component(value: f32) -> f32 {
    let clamped = clamp(value, 0.0, 1.0);
    if clamped <= 0.04045 {
        return clamped / 12.92;
    }
    return pow((clamped + 0.055) / 1.055, 2.4);
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb_component(color.r),
        linear_to_srgb_component(color.g),
        linear_to_srgb_component(color.b),
    );
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_to_linear_component(color.r),
        srgb_to_linear_component(color.g),
        srgb_to_linear_component(color.b),
    );
}

fn linear_to_oklab(color: vec3<f32>) -> vec3<f32> {
    let l = 0.4122214708 * color.r + 0.5363325363 * color.g + 0.0514459929 * color.b;
    let m = 0.2119034982 * color.r + 0.6806995451 * color.g + 0.1073969566 * color.b;
    let s = 0.0883024619 * color.r + 0.2817188376 * color.g + 0.6299787005 * color.b;
    let l_root = sign(l) * pow(abs(l), 1.0 / 3.0);
    let m_root = sign(m) * pow(abs(m), 1.0 / 3.0);
    let s_root = sign(s) * pow(abs(s), 1.0 / 3.0);
    return vec3<f32>(
        0.2104542553 * l_root + 0.7936177850 * m_root - 0.0040720468 * s_root,
        1.9779984951 * l_root - 2.4285922050 * m_root + 0.4505937099 * s_root,
        0.0259040371 * l_root + 0.7827717662 * m_root - 0.8086757660 * s_root,
    );
}

fn oklab_to_linear(color: vec3<f32>) -> vec3<f32> {
    let l_root = color.x + 0.3963377774 * color.y + 0.2158037573 * color.z;
    let m_root = color.x - 0.1055613458 * color.y - 0.0638541728 * color.z;
    let s_root = color.x - 0.0894841775 * color.y - 1.2914855480 * color.z;
    let l = l_root * l_root * l_root;
    let m = m_root * m_root * m_root;
    let s = s_root * s_root * s_root;
    return vec3<f32>(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    );
}

fn interpolate_color(first: vec4<f32>, second: vec4<f32>, amount: f32, space: f32) -> vec4<f32> {
    var first_coordinates: vec3<f32>;
    var second_coordinates: vec3<f32>;
    if space > 1.5 {
        first_coordinates = linear_to_oklab(first.rgb);
        second_coordinates = linear_to_oklab(second.rgb);
    } else if space > 0.5 {
        first_coordinates = linear_to_srgb(first.rgb);
        second_coordinates = linear_to_srgb(second.rgb);
    } else {
        first_coordinates = first.rgb;
        second_coordinates = second.rgb;
    }
    let alpha = mix(first.a, second.a, amount);
    let premultiplied = mix(first_coordinates * first.a, second_coordinates * second.a, amount);
    var coordinates = vec3<f32>(0.0);
    if alpha > 0.000001 {
        coordinates = premultiplied / alpha;
    }
    var rgb: vec3<f32>;
    if space > 1.5 {
        rgb = oklab_to_linear(coordinates);
    } else if space > 0.5 {
        rgb = srgb_to_linear(coordinates);
    } else {
        rgb = coordinates;
    }
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}

fn path_color(paint: PathPaint, position: vec2<f32>) -> vec4<f32> {
    if paint.stops_mode.z < 0.5 {
        return paint.color_0;
    }
    let start = paint.gradient_line.xy;
    let direction = paint.gradient_line.zw - start;
    let denominator = dot(direction, direction);
    var position_on_line = 0.0;
    if denominator > 0.000001 {
        position_on_line = clamp(dot(position - start, direction) / denominator, 0.0, 1.0);
    }
    let first_stop = paint.stops_mode.x;
    let second_stop = paint.stops_mode.y;
    var amount = 0.0;
    if position_on_line >= second_stop {
        amount = 1.0;
    } else if position_on_line > first_stop && second_stop > first_stop {
        amount = (position_on_line - first_stop) / (second_stop - first_stop);
    }
    return interpolate_color(paint.color_0, paint.color_1, amount, paint.stops_mode.w);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let paint = paints[input.paint_index];
    if input.logical_position.x < paint.clip.x || input.logical_position.y < paint.clip.y ||
       input.logical_position.x >= paint.clip.z || input.logical_position.y >= paint.clip.w {
        discard;
    }

    let derivative = max(fwidth(input.barycentric), vec3<f32>(0.000001));
    let edge_distance = input.barycentric / derivative;
    let masked_distance = select(vec3<f32>(1000000.0), edge_distance, input.edge_mask > vec3<f32>(0.5));
    let coverage = smoothstep(0.0, 1.0, min(masked_distance.x, min(masked_distance.y, masked_distance.z)));
    let color = path_color(paint, input.logical_position);
    let alpha = color.a * coverage;
    return vec4<f32>(color.rgb * alpha, alpha);
}
