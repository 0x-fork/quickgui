# QuickGUI

QuickGUI is a damage-driven, GPU-accelerated GUI foundation for Rust desktop apps. It combines a GPUI-style fluent view API, Taffy flexbox layout, a purpose-built WGPU renderer, cached Unicode text shaping, native accessibility, and bounded virtual scrolling.

The hot path is intentionally small:

- windows use `ControlFlow::Wait` and do no application rendering while clean;
- wheel events are coalesced at the OS frame boundary;
- retained element trees can repaint hover and scroll state without rebuilding the view or rerunning flex layout;
- rectangles, borders, rounded corners, and analytic drop/inset shadows share one instance upload and split only when overlap ordering requires it;
- visible images share one instance upload, while consecutive uses of the same texture share a draw;
- SVG icons are parsed once, rasterized only for unseen physical sizes, and recolored from one-channel GPU masks;
- retained fill/stroke paths are tessellated once, uploaded through bounded triple buffers, and batched by overlap order;
- animated images wake at their exact frame deadlines and stop completely while offscreen or occluded;
- text shaping and rasterization are retained by stable element IDs;
- one reusable spatial order tree preserves web sibling paint order across shapes, paths, images, SVGs, and text without sorting disjoint content;
- dynamic instance buffers are triple-buffered rather than overwritten while the GPU may still read them;
- `VirtualList` computes the mounted range in O(1), independent of item count.

The current milestone is a runnable framework core, not a claim that every production widget already exists. The macOS runtime and 100,000-row demo have live visual acceptance. Windows and Linux backends are configured through Winit/WGPU but still need CI and runtime acceptance on those operating systems.

## View API

Views use regular Rust with JSX-like composition, composable Tailwind-style spacing, inherited typography, flexbox, stable identities, and view-local listeners:

```rust
use quickgui::{
    App, Color, EventContext, IntoElement, View, ViewContext, div, text,
};

struct Counter {
    count: usize,
}

impl View for Counter {
    fn render(
        &mut self,
        cx: &mut ViewContext<'_, Self>,
    ) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx: &mut EventContext| {
            this.count += 1;
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(Color::rgb8(18, 18, 20))
            .text_color(Color::rgb8(240, 241, 244))
            .child(text(format!("Count: {}", self.count)).text_2xl().font_semibold())
            .child(
                div()
                    .on_click(increment)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(45, 105, 180))
                    .hover(|style| style.bg(Color::rgb8(56, 122, 204)))
                    .active(|style| style.bg(Color::rgb8(37, 87, 151)))
                    .child("Increment"),
            )
    }
}

fn main() -> Result<(), quickgui::AppError> {
    App::new(Counter { count: 0 })
        .title("Counter")
        .size(480.0, 320.0)
        .run()
}
```

`Scene`, `Quad`, `Shadow`, `PathPrimitive`, `ImagePrimitive`, and `SvgPrimitive` remain public as the lower-level escape hatch for specialized widgets. Ordinary application views should use `Element` builders or the scoped `canvas(...)` painter.

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
use the same ordered instanced shape draw as quads: there are no per-shadow textures, CPU blur,
cache entries, or draw calls. See `cargo run --release --example shadows`.

Ordinary text wraps at word boundaries by default and contributes its wrapped height to flex layout; use `.no_wrap()` where a single line is intentional. Controlled native text input uses the same listener pattern:

```rust
let edit_name = cx.input_listener("name", |this, value, cx| {
    this.name = value.into();
    cx.invalidate();
});

text_input(self.name.clone())
    .on_input(edit_name)
    .placeholder("Type a name…")
    .accessibility_label("Name")
    .w_full()
```

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
wakes the owning window exactly once when decoding completes. The fixed two-thread worker pool is
created only on first use, its channel is bounded at 64 jobs, and each window's CPU-side resource
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

Commands use typed actions and a keymap that is independent from rendering. Handlers live on the
focused element path, so editor commands naturally take precedence over pane and workspace
fallbacks:

```rust
quickgui::actions!(editor, [Save, SplitLeft]);

let editor_focus = cx.focus_handle("editor");
let save = cx.action_listener("editor", |this, _: &Save, cx| {
    this.save();
    cx.invalidate();
});

div()
    .track_focus(editor_focus)
    .key_context("Editor mode=insert")
    .on_action(save)
```

```rust
App::new(view).bind_keys([
    KeyBinding::new("platform-s", Save, Some("Editor")),
    KeyBinding::new("platform-k left", SplitLeft, Some("Workspace > Editor")),
]);
```

Actions consume by default; call `cx.propagate()` to continue bubbling. Buttons, command palettes,
and native menus can call `cx.dispatch_action(Save)` and share the same handlers.
Context predicates support `!`, `&&`, `||`, `>`, `==`, and `!=`. Incomplete multi-stroke bindings
sleep until the next key or one timeout wake-up, then replay unmatched input. See
`cargo run --release --example actions_keymap`.

Application menus use GPUI-shaped declarations and derive their key equivalents from the same
contextual keymap:

```rust
let menus = [
    Menu::new("File").items([
        MenuItem::action("Save", Save),
        MenuItem::separator(),
        MenuItem::submenu(Menu::new("Layout").items([
            MenuItem::action("Split Left", SplitLeft),
        ])),
    ]),
];

App::new(view).menus(menus);
```

On macOS these are real `NSMenu` trees with native keyboard navigation, nested submenus, checked
and disabled state, and focus-aware validation. `EventContext::set_menus` replaces labels or state
after an application mutation without introducing an idle update loop. `MenuItem::os_action`
first follows AppKit's responder chain for Cut/Copy/Paste/Select All/Undo/Redo, then falls back to
the same typed action and QuickGUI text input. This lets one Edit menu serve both GPU controls and
embedded `NSView` children. Opening a menu also cancels an incomplete multi-stroke prefix.

Overlays use a document-independent GPU plane, so they escape ancestor clipping and can be
anchored to any stable element ID:

```rust
overlay()
    .anchor_to(trigger.id(), AnchorPlacement::BottomEnd)
    .anchor_gap(8.0)
    .viewport_margin(12.0)
    .on_dismiss(dismiss)
    .restore_focus_to(trigger)
    .accessibility_role(AccessibilityRole::Menu)
    .child("Popover content")
```

Placement follows web popover behavior: use the preferred side when it fits, flip to the opposite
side when it has more room, try alternate alignment, then shift inside the viewport. A dismissible
overlay blocks pointer input behind it, dismisses on outside press or Escape, and restores its
trigger's focus.

On macOS, any retained AppKit `NSView` subclass can participate in the same layout:

```rust
let field: Retained<NSTextField> = /* construct on the main thread */;

native_view(field.as_ref())
    .id("native-field")
    .w_full()
    .h(44.0)
    .rounded_lg()
```

QuickGUI switches that window to three-plane composition: WGPU base content, native child views,
then a transparent WGPU overlay. The second full-window swapchain is created only when the first
overlay opens, then retained for low-latency reuse. Native views are retained by Objective-C
identity, clipped and resized in logical points, reordered by `z_index`, and removed when their
elements disappear. AppKit and QuickGUI first-responder focus are synchronized without an idle
polling loop. A background-colored launch shield stays above every plane until WGPU successfully
presents and completes the first frame, so an AppKit child cannot flash over an otherwise empty
window. Layout, text shaping, native reconciliation, and buffer uploads run while the window is
still hidden. QuickGUI briefly detaches the retained content view from that hidden `NSWindow`, which
allows WGPU to present the actual Metal layer without ordering any window onscreen. It reattaches
the same view at the unchanged frame only after GPU completion, preventing both black intermediate
frames and cross-monitor movement in unoptimized builds.
See `cargo run --release --example native_view` and
`cargo run --release --example overlays`.

## Run the stress test

```console
cargo run --release --example stress_scroll
```

The demo renders a selectable 100,000-row list while mounting only the visible rows plus two rows of overscan. It supports trackpad/wheel scrolling, a captured draggable scrollbar, hover and pressed feedback, click listeners, arrow/page navigation, Home/End, and in-window CPU/render telemetry.

Run validation and the CPU-side list benchmarks with:

```console
cargo test --all-targets
cargo bench --bench virtual_list
cargo clippy --all-targets --all-features -- -D warnings
```

## Design status

Implemented now:

- macOS/Windows/Linux backend selection through Winit 0.30 and WGPU 30;
- sleeping single-window runtime with resize, DPI, pointer, wheel, keyboard, full IME preedit/commit routing, and focus events;
- declarative elements, Taffy flexbox, absolute positioning, clipping, inherited text styles, and Tailwind-like helpers;
- ordered `z_index` stacking layers plus portal-style overlays with anchor flip/shift, pointer blocking, outside/Escape dismissal, and focus restoration;
- macOS `NSView` children composed between the base and transparent overlay WGPU surfaces, with atomic first-frame reveal, keyed lifetime, clipping, sizing, first-responder handoff, and merged AccessKit/AppKit accessibility routing;
- keyed hover/active/focus/click state, captured pointer gestures, Tab traversal, keyboard button activation, and type-safe `ViewContext` listeners;
- typed actions, non-focusable focus scopes, focused-path bubbling, contextual keymaps, programmatic command dispatch, and replay-safe multi-stroke bindings;
- native macOS application menus with nested/system menus, contextual key equivalents, focused command validation, dynamic replacement, checked/disabled items, and AppKit responder-chain actions;
- controlled single-line text input with grapheme-safe movement/deletion, mouse caret and drag selection, horizontal scrolling, copy/cut/paste, and bounded undo/redo history;
- AccessKit trees with semantic roles, labels, disabled/selected state, native focus/click actions, and editable value/selection actions;
- linear-light colors, premultiplied blending, analytic rounded rectangles, borders, CSS-ordered drop/inset shadows, and HiDPI rendering;
- static, asynchronous, and animated PNG/JPEG/WebP/GIF/RGBA images with intrinsic layout, all web `object-fit` modes, rounded clipping, GPU grayscale, delayed loading/error fallbacks, per-element playback, Reduce Motion, stable identity, and hard-bounded CPU/GPU caches;
- retained SVG/SVGZ icons and tessellated fill/stroke paths with intrinsic layout, web `object-fit`, transforms, dashes, arcs, two-stop gradients, analytic boundary antialiasing, and scoped custom canvas painting;
- Cosmic Text/Glyphon shaping, fallback, rasterization, atlas reuse, and bounded text-layout retention;
- fixed-height virtualization, clamped scrolling, scrollbar math, and performance telemetry.

Still required before calling it a production-complete general GUI framework:

- rich and multiline text editing, word navigation, and input validation hooks;
- command-palette widgets, drag/drop, multi-window APIs, and Windows/Linux native menu projection;
- Windows/Linux runtime and visual CI, plus a portable benchmark matrix.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the renderer and ownership model.

## Why this renderer

Most desktop UI pixels are rectangles, glyphs, icons, images, and modest vector paths. Dedicated data-driven pipelines minimize CPU preparation and draw calls for that workload. QuickGUI uses retained Lyon tessellation plus a narrow WGPU path pipeline instead of making a general compute vector renderer part of every frame; a broader custom-shader or vector backend can remain opt-in.

The design is informed by the same proven ideas documented by [Zed's GPU UI architecture](https://zed.dev/blog/videogame), while changing the scheduling and ownership choices that matter for low CPU: no permanent redraw loop, frame-boundary input coalescing, retained flex layout between view changes, and bounded text/list caches. Sublime Text 4 likewise documents GPU compositing as the route to fluid high-DPI UI with lower power use in its [GPU rendering notes](https://www.sublimetext.com/docs/gpu_rendering.html).

## License

MIT.
