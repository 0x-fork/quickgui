struct ViewUniform {
    viewport: vec2<f32>,
    scale: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> view: ViewUniform;

struct VertexInput {
    @location(0) geometry: vec4<f32>,
    @location(1) primary: vec4<f32>,
    @location(2) secondary: vec4<f32>,
    @location(3) clip: vec4<f32>,
    // mode, subject radius, border width / element radius, blur radius
    @location(4) params: vec4<f32>,
    @location(5) subject: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) logical_position: vec2<f32>,
    @location(1) geometry: vec4<f32>,
    @location(2) primary: vec4<f32>,
    @location(3) secondary: vec4<f32>,
    @location(4) clip: vec4<f32>,
    @location(5) params: vec4<f32>,
    @location(6) subject: vec4<f32>,
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
    let logical_position = input.geometry.xy + corner * input.geometry.zw;
    let physical_position = logical_position * view.scale;

    var output: VertexOutput;
    output.position = vec4<f32>(
        physical_position.x / view.viewport.x * 2.0 - 1.0,
        1.0 - physical_position.y / view.viewport.y * 2.0,
        0.0,
        1.0,
    );
    output.logical_position = logical_position;
    output.geometry = input.geometry;
    output.primary = input.primary;
    output.secondary = input.secondary;
    // Keep clipping in the same logical coordinate space as the primitive. During a macOS live
    // resize the Metal drawable may intentionally remain larger than the current viewport; the
    // fragment builtin position is then expressed in drawable pixels and cannot be compared to
    // current-viewport pixels without clipping the primitive by the resize ratio a second time.
    output.clip = input.clip;
    output.params = input.params;
    output.subject = input.subject;
    return output;
}

fn rounded_box_distance(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let safe_radius = clamp(radius, 0.0, min(half_size.x, half_size.y));
    let q = abs(point - half_size) - half_size + vec2<f32>(safe_radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - safe_radius;
}

fn rounded_rect_distance(point: vec2<f32>, rect: vec4<f32>, radius: f32) -> f32 {
    return rounded_box_distance(point - rect.xy, rect.zw, radius);
}

fn coverage(distance: f32) -> f32 {
    let antialias_width = max(fwidth(distance), 0.0001);
    return 1.0 - smoothstep(-antialias_width, antialias_width, distance);
}

// Maximum absolute error is about 1.5e-7, which is far below an 8-bit alpha step.
fn erf_approx(value: f32) -> f32 {
    let x = abs(value);
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let polynomial = (((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t
        - 0.284496736) * t + 0.254829592) * t);
    let result = 1.0 - polynomial * exp(-x * x);
    return select(-result, result, value >= 0.0);
}

fn blurred_coverage(distance: f32, blur_radius: f32) -> f32 {
    let crisp = coverage(distance);
    if blur_radius <= 0.001 {
        return crisp;
    }
    // CSS blur radii map closely to a Gaussian whose standard deviation is half the radius.
    let sigma = max(blur_radius * 0.5, 0.0001);
    let blurred = 0.5 * (1.0 - erf_approx(distance / (1.41421356237 * sigma)));
    return clamp(blurred, 0.0, 1.0);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let logical_position = input.logical_position;
    if logical_position.x < input.clip.x || logical_position.y < input.clip.y ||
       logical_position.x >= input.clip.z || logical_position.y >= input.clip.w {
        discard;
    }

    // A complete underline span is one instance. The fragment shader evaluates its wave
    // analytically, so long diagnostics do not create CPU-side path vertices or extra draws.
    if input.params.x > 2.5 {
        let thickness = max(input.params.z, 0.0001);
        let wavelength = max(input.params.w, 0.0001);
        let amplitude = max(input.subject.x, 0.0);
        let wave_y = input.params.y + amplitude * sin(
            6.28318530718 * input.logical_position.x / wavelength,
        );
        let distance = abs(input.logical_position.y - wave_y) - thickness * 0.5;
        let wave_coverage = coverage(distance);
        let alpha = input.primary.a * wave_coverage;
        return vec4<f32>(input.primary.rgb * alpha, alpha);
    }

    // A square, borderless quad already has exact coverage from its two triangles. Running an
    // SDF over that geometry antialiases every internal edge independently, which exposes seams
    // between adjacent terminal-cell backgrounds at fractional physical coordinates.
    if input.params.x < 0.5 && input.params.y <= 0.0 && input.params.z <= 0.0 {
        let alpha = input.primary.a;
        return vec4<f32>(input.primary.rgb * alpha, alpha);
    }

    let subject_distance = rounded_rect_distance(
        input.logical_position,
        input.subject,
        input.params.y,
    );
    let subject_coverage = blurred_coverage(subject_distance, input.params.w);
    // Regular rounded rectangle with an inside border.
    if input.params.x < 0.5 {
        let border_width = min(
            max(input.params.z, 0.0),
            min(input.subject.z, input.subject.w) * 0.5,
        );
        let inner_rect = vec4<f32>(
            input.subject.xy + vec2<f32>(border_width),
            max(input.subject.zw - vec2<f32>(border_width * 2.0), vec2<f32>(0.0)),
        );
        let inner = coverage(rounded_rect_distance(
            input.logical_position,
            inner_rect,
            max(input.params.y - border_width, 0.0),
        ));
        let fill_alpha = input.primary.a * inner;
        let border_alpha = input.secondary.a * max(subject_coverage - inner, 0.0);
        let alpha = fill_alpha + border_alpha;
        let rgb = input.primary.rgb * fill_alpha + input.secondary.rgb * border_alpha;
        return vec4<f32>(rgb, alpha);
    }

    // Drop shadow. Its element is painted by a later instance in the same ordered draw.
    if input.params.x < 1.5 {
        let alpha = input.primary.a * subject_coverage;
        return vec4<f32>(input.primary.rgb * alpha, alpha);
    }

    // Inset shadow: the element is the mask and the translated subject is its clear hole.
    let geometry_coverage = coverage(rounded_rect_distance(
        input.logical_position,
        input.geometry,
        input.params.z,
    ));
    let inset_coverage = geometry_coverage * (1.0 - subject_coverage);
    let alpha = input.primary.a * inset_coverage;
    return vec4<f32>(input.primary.rgb * alpha, alpha);
}
