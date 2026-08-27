# Graphics and media

[Documentation index](README.md)

Box shadows follow web paint order, support offset, blur, positive or negative spread, and inset
rendering, and stay paint-only in hover/active/focus states:

```rust
use quickgui::{BoxShadow, Color, div};

div()
    .rounded_xl()
    .bg(Color::WHITE)
    .shadows([
        BoxShadow::new(0.0, 10.0, Color::rgba8(15, 23, 42, 38))
            .blur_radius(24.0)
            .spread_radius(-6.0),
        BoxShadow::new(0.0, 1.0, Color::rgba8(15, 23, 42, 24))
            .blur_radius(3.0),
    ])
    .hover(|style| style.shadow_lg())
```

The first declared shadow is topmost, matching CSS. `.shadow_sm()`, `.shadow_md()`,
`.shadow_lg()`, `.shadow_xl()`, and `.shadow_2xl()` provide Tailwind-shaped elevations. Shadows
are capped at eight declarations per element or state override and use the same ordered instanced
shape draw as quads: there are no per-shadow textures, CPU blur, cache entries, or draw calls.
Opted-in [style transitions](animations.md) interpolate compatible lists without per-frame heap
allocation. See `cargo run --release --example shadows`.

`.opacity(value)` is one scoped paint multiplier rather than an offscreen layer. It composes across
shapes, images, SVG masks, solid or gradient paths, text decorations, rich glyph colors, canvas
commands, and custom WGSL. This preserves batching and bounded memory; overlapping descendants use
GPUI-style per-primitive alpha rather than CSS group-isolation compositing.

Static images use the same flexbox sizing API. With neither axis specified, the decoded pixel size
is the intrinsic logical size; setting one axis preserves the aspect ratio. The default is
`object-fit: contain`:

```rust
let logo = Image::decode(include_bytes!("logo.webp"))?;

img(&logo)
    .w_full()
    .h(240.0)
    .object_fit(ObjectFit::Cover)
    .rounded_lg()
    .accessibility_label("Project logo")
```

`Image::decode` accepts PNG, JPEG, WebP, and the first GIF frame; `Image::from_rgba` accepts
generated straight-alpha RGBA8 pixels. Decoding is explicit and synchronous so applications can
move it off latency-sensitive input handling. A decoded image is limited to 4096 px per axis and
64 MiB. Clones share immutable pixels and GPU identity. Each renderer retains at most 256 textures
and 128 MiB of decoded texture data, evicting least-recently-used off-screen entries. See
`cargo run --release --example images` for all five `ObjectFit` modes and GPU grayscale.

SVG icons use the same intrinsic flexbox sizing and `ObjectFit` API. Their color inherits from
typography, including hover, active, and focus overrides, while transforms remain paint-only:

```rust
use quickgui::{Color, Svg, SvgTransform, button, svg};

let icon = Svg::from_bytes(include_bytes!("icons/spark.svg"))?;

button()
    .text_color(Color::rgb8(56, 189, 248))
    .hover(|style| style.text_color(Color::WHITE))
    .child(
        svg(&icon)
            .size(24.0, 24.0)
            .svg_transform(SvgTransform::new().rotate(0.15)),
    )
```

`Svg` retains one parsed `resvg` tree and stable identity. The first visible use of an identity at a
new physical size is rasterized synchronously, then uploaded as an `R8Unorm` alpha mask; tint and
translation are per-instance GPU data and do not invalidate that mask. Retain `Svg` values instead
of parsing them inside `View::render`. Source and expanded SVGZ data are capped at 4 MiB, raster
entries at 4096 px per axis and 16 million pixels, and each renderer at 512 entries or 32 MiB of
one-channel masks. System fonts initialize only for SVGs that may contain text. All SVG `<image>`
references are ignored, so vector icons cannot hide an unbounded external or embedded bitmap
decode. See `cargo run --release --example svg`.

Vector paths use the same intrinsic layout and `ObjectFit` behavior as images and SVGs. Build and
retain them outside `View::render`; fill/stroke tessellation, joins, caps, curves, arcs, polygons,
dashes, and transforms are resolved once:

```rust
use quickgui::{Color, PathBuilder, Point, canvas, path};

let mut builder = PathBuilder::fill();
builder.move_to(Point::new(50.0, 0.0));
builder.line_to(Point::new(100.0, 100.0));
builder.line_to(Point::new(0.0, 100.0));
builder.close();
let triangle = builder.build()?;

let icon = path(&triangle)
    .size(96.0, 96.0)
    .path_background(Color::rgb8(56, 189, 248));

let custom = canvas(move |bounds, surface| {
    surface.fill_rect(bounds, Color::rgb8(15, 23, 42));
    surface.paint_path_at(&triangle, Point::new(12.0, 12.0), Color::WHITE);
});
```

`path(...)` inherits text color unless `.path_background(...)` supplies a solid or two-stop linear
gradient. Gradients support linear-sRGB, encoded sRGB, and Oklab interpolation. `canvas(...)` uses
local coordinates, clips every command to its element and ancestors, and calls its painter only on
an actual repaint. GPU edge coverage is analytic and applies only to true outline edges, so internal
tessellation edges do not create seams and the window does not carry a permanent multisample target.
One retained path is capped at 65,536 commands, 196,605 vertices, and 4 MiB. Each frame admits at
most 262,144 vertices and 16,384 paints in source order, skipping later paths instead of allocating
without bound. See `cargo run --release --example paths`.

Specialized effects can retain a validated application fragment shader without creating another
WGPU device, texture, or readback path:

```rust
use quickgui::{CustomShader, ShaderParameters, custom_shader};

let aurora = CustomShader::new(r#"
fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {
    let tint = input.params[0].xyz;
    return vec4<f32>(mix(tint, vec3<f32>(input.uv, 1.0), 0.5), 1.0);
}
"#)?;

let effect = custom_shader(aurora)
    .shader_parameters(ShaderParameters::new().vector(0, [0.1, 0.8, 0.9, 0.0]))
    .size_full();
```

QuickGUI owns the vertex stage, physical clipping, and straight-to-premultiplied-alpha wrapper.
Each instance receives normalized `uv`, logical `position` and `size`, plus four sanitized
`vec4<f32>` parameter slots. WGSL is parsed and validated at construction, then compiled lazily and
cached per window. Source is capped at 64 KiB; a window retains at most 32 application pipelines
and submits at most 4,096 visible shader rectangles per frame through one triple-buffered instanced
upload. There is deliberately no clock uniform or implicit animation: changing parameters and
invalidating the view submits one damage-driven frame, then the window sleeps again. Shader source
is trusted application code; structural validation cannot make an intentionally non-terminating
GPU program safe. See `cargo run --release --example custom_shader`.

Filesystem image paths and retained custom loaders use the asynchronous resource path. Loading and fallback content
is ordinary QuickGUI layout, so it supports the same flexbox, typography, and wrapping API:

```rust
let avatar = ImageResource::from_path("assets/avatar.webp");

img(&avatar)
    .size(160.0, 160.0)
    .object_fit(ObjectFit::Cover)
    .with_loading(|| div().child("Loading…"))
    .with_fallback(|| div().child("Avatar unavailable"))
```

Fast resources never flash loading content; it becomes eligible after 200 ms. A Winit user event
wakes the owning window exactly once when decoding completes. One application-wide two-thread worker pool is
created only on first use, its shared channel is bounded at 64 jobs, and each window's CPU-side resource
cache is capped at 256 entries and 128 MiB. Path files are also rejected above 64 MiB before
decoding. Retain and clone `ImageResource::custom(...)` handles rather than constructing them in
every render. See `cargo run --release --example image_resources` for fast, delayed, and failing
resources.

Animated GIF and WebP paths are detected automatically. Applications can also build retained
animations directly; each mounted image element preserves its own playback phase across ordinary
view invalidations:

```rust
let animation = AnimatedImage::with_repeat(
    frames.into_iter().map(|(image, delay)| AnimatedImageFrame::new(image, delay)),
    AnimationRepeat::Finite(2),
)?;

img(&animation).size(320.0, 180.0).object_fit(ObjectFit::Cover)
```

Playback schedules only the next frame deadline instead of enabling a display-rate frame loop.
Offscreen and occluded animations pause without catching up when they return; finite animations
stop on their last frame. macOS Reduce Motion is respected automatically, and
`App::reduce_motion(true)` provides an application override. One animation retains at most 256
frames and 64 MiB of unique decoded pixels, while delays faster than 60 Hz are clamped. See
`cargo run --release --example animated_images` for direct, finite, and asynchronously loaded
animations.
