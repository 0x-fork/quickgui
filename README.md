# QuickGUI

QuickGUI is a damage-driven, GPU-accelerated GUI foundation for Rust desktop apps. It combines a GPUI-style fluent view API, Taffy Flexbox and CSS Grid layout, a purpose-built WGPU renderer, cached Unicode text shaping, native accessibility, and bounded virtual scrolling.

The hot path is intentionally small:

- every window uses `ControlFlow::Wait` and does no application rendering while clean;
- wheel events are coalesced at the OS frame boundary;
- retained element trees can repaint hover and scroll state without rebuilding the view or rerunning layout;
- overlay scrollbars reveal on motion, expand on hover/capture, and use one exact hide deadline instead of an animation loop;
- rectangles, borders, rounded corners, and analytic drop/inset shadows share one instance upload and split only when overlap ordering requires it;
- visible images share one instance upload, while consecutive uses of the same texture share a draw;
- SVG icons are parsed once, rasterized only for unseen physical sizes, and recolored from one-channel GPU masks;
- retained fill/stroke paths are tessellated once, uploaded through bounded triple buffers, and batched by overlap order;
- animated images wake at their exact frame deadlines and stop completely while offscreen or occluded;
- typed drag previews move in a detached overlay tree without rebuilding the application view;
- outbound macOS typed drags lazily arm one boundary monitor only for the active source gesture;
- delayed tooltips use one exact wake-up and retain a pointer-passive detached GPU tree while visible;
- picker matching runs only when its bounded query or registry changes and mounts visible results only;
- text shaping and rasterization are retained by stable element IDs;
- one reusable spatial order tree preserves web sibling paint order across shapes, paths, images, SVGs, and text without sorting disjoint content;
- dynamic instance buffers are triple-buffered rather than overwritten while the GPU may still read them;
- compatible windows share one WGPU instance, adapter, device, and queue; image decoding and application work use separate bounded two-thread pools;
- shared `Entity<T>` state invalidates only windows that observed it during their latest retained render;
- application-global typed state delivers coalesced changes only to observing windows, with no polling or idle frame;
- `VirtualList` computes the mounted range in O(1), independent of item count; binding it with
  `.virtual_scroll(&list)` reuses the framework scrollbar without per-hover view rebuilds.

The current milestone is a runnable framework core, not a claim that every production widget already exists. The macOS runtime and 100,000-row demo have live visual acceptance. Windows and Linux backends are configured through Winit/WGPU but still need CI and runtime acceptance on those operating systems.

## View API

Views use regular Rust with JSX-like composition, composable Tailwind-style spacing, inherited typography, Flexbox and CSS Grid, stable identities, and view-local listeners:

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

`Scene`, `Quad`, `Shadow`, `PathPrimitive`, `ImagePrimitive`, `SvgPrimitive`, and
`CustomShaderPrimitive` remain public as the lower-level escape hatch for specialized widgets.
Ordinary application views should use `Element` builders or the scoped `canvas(...)` painter.

## CSS Grid

The GPUI-shaped equal-track and placement helpers work directly on ordinary elements:

```rust
div()
    .size_full()
    .grid()
    .grid_cols(5)
    .grid_rows(5)
    .gap_2()
    .child(header.col_span_full().row_span(1))
    .child(sidebar.col_span(1).row_span(3))
    .child(content.col_span(3).row_span(3))
    .child(inspector.col_span(1).row_span(3))
    .child(footer.col_span_full().row_span(1))
```

For app-shell and dashboard layouts, explicit web-style tracks support fixed pixels, percentages,
fractions, intrinsic sizing, fit-content, and the common `minmax(px, fr)` form:

```rust
use quickgui::GridTrack;

div()
    .grid()
    .grid_template_columns([
        GridTrack::px(180.0),
        GridTrack::minmax_px_fr(240.0, 1.0),
        GridTrack::fit_content_px(160.0),
    ])
    .grid_template_rows([GridTrack::px(64.0), GridTrack::fr(1.0)])
```

`grid_cols[_min_content|_max_content]`, matching row helpers, explicit line start/end, full and
numeric spans, and sparse/dense row/column auto-flow are available. Track lists and spans are
hard-capped at 1,024 per axis; equal tracks remain one compact Taffy `repeat()` component. Grid is
retained CPU layout only: paint-only interaction and scrolling reuse it, and it creates no GPU
resources or idle frames. Run `cargo run --release --example grid_layout` and resize across 700 pt
to compare the grid with its stacked Flexbox fallback.

## Multiple windows

An event handler can mount a different concrete `View` type in another native window and keep its
stable handle. Closing one window does not stop the application until the final window closes:

```rust
let open = cx.listener("open-inspector", |this, cx| {
    this.inspector = Some(cx.open_window(
        Inspector::new(),
        WindowOptions::new("Inspector")
            .size(520.0, 360.0)
            .background(Color::rgb8(20, 22, 27)),
    ));
    cx.invalidate();
});

let focus = cx.listener("focus-inspector", |this, cx| {
    if let Some(inspector) = this.inspector {
        cx.focus_window(inspector);
        cx.invalidate_window(inspector);
        // cx.close_window_handle(inspector);
    }
});
```

State shared by unrelated view types uses a main-thread entity instead of a lock or manual window
handle fan-out:

```rust
use quickgui::{Entity, EventEmitter, Subscription};

struct WorkspacePublished {
    revision: usize,
}
impl EventEmitter<WorkspacePublished> for WorkspaceState {}

let workspace = Entity::new(WorkspaceState::default());

// During either window's render, this both reads and retains the observation.
let revision = cx.observe(&self.workspace, |state| state.revision);

// During an event callback, every observing window receives one coalesced invalidation.
self.workspace.update(cx, |state, _cx| {
    state.revision += 1;
});

// Store this `Subscription` on the view; dropping it cancels delivery.
if self.workspace_events.is_none() {
    self.workspace_events = Some(cx.subscribe(
        &self.workspace,
        |this, _source, event: &WorkspacePublished, cx| {
            this.last_published_revision = event.revision;
            cx.invalidate();
        },
    ));
}

// Typed events are delivered after this callback releases its view/entity borrows.
self.workspace.emit(cx, WorkspacePublished { revision: 1 });
```

`Entity<T>` is deliberately main-thread-owned and lock-free; background work returns through the
existing UI-thread completion callback before updating it. Conditional observations disappear on
the next declarative rebuild, and `WeakEntity<T>` breaks ownership cycles. A window retains at most
4,096 entity identities. One callback retains at most 1,024 distinct notifications, then falls back
to one bounded all-window invalidation rather than losing a change or growing memory without bound.
Typed events are not coalesced: they retain FIFO emission order, then visit windows in creation order
and subscriptions in registration order. `Subscription` follows GPUI's RAII lifetime contract;
`detach()` keeps it until the subscribing window closes. Each window retains at most 4,096 active
subscriptions, one callback emits at most 1,024 events, one effect cycle queues at most 4,096 events,
and recursive delivery stops at 65,536 callback invocations. There is no polling, subscription task,
or idle frame.

Application-wide settings and services use one exact Rust type instead of an entity handle shared
through every view constructor:

```rust
use quickgui::{App, Global, Subscription};

#[derive(Default)]
struct Appearance {
    warm: bool,
}
impl Global for Appearance {}

App::new(Launcher::default())
    .global(Appearance::default())
    .run()?;

// During render, this read conditionally watches the type for this window.
let warm = cx.watch_global::<Appearance, _>(|appearance| appearance.warm);

// During an event callback, mutation is synchronous and notification is deferred until
// the callback releases its borrows.
cx.update_global::<Appearance, _>(|appearance| appearance.warm = !appearance.warm);

// Store the RAII subscription on the view; dropping it cancels callback delivery.
let subscription: Subscription = cx.observe_global::<Appearance>(|this, cx| {
    this.last_theme_change = cx.global::<Appearance>().warm;
    cx.invalidate();
});
```

`observe_global` follows GPUI's callback shape; `subscribe_global` is an explicit alias, while
`watch_global` is QuickGUI's conditional declarative read. `has_global`, `global`, and `try_global`
provide unobserved reads. `global_mut`,
`default_global`, `set_global`, `update_global`, and `remove_global` are available in event
callbacks and coalesce changes by concrete `TypeId`. The application retains at most 1,024 global
types; a window may observe 1,024 types and retain 1,024 RAII global subscriptions. One callback
tracks 256 exact changed types before conservatively notifying all global observers, one deferred
turn queues at most 1,024 types, and recursive callback delivery stops at 65,536 invocations.
Globals are main-thread-owned, so ordinary access is lock-free and adds no thread, timer, polling
pass, GPU resource, or idle frame. See `cargo run --release --example multi_window` for exact
cross-window observation and cancellation behavior.

Each window independently owns its retained view, element/UI tree, listener registry, frame
scheduler, focus, pointer capture, IME/key state, scene, surfaces, metrics, and bounded render
caches. Compatible windows with the same `PerformanceProfile` share the heavyweight WGPU device
and queue. `Event::CloseRequested` is delivered before destruction; a view can call
`cx.prevent_close()`, while `cx.close_window()` explicitly closes the current window. Async image
completions are tagged with the owning `WindowHandle`, so identical per-window request IDs cannot
wake or mutate another cache. On macOS, arbitrary typed drag values can cross these independent
windows without serialization; the example includes a bidirectional process-local card transfer.
See `cargo run --release --example multi_window`.

Window creation also retains a native role, restore geometry, initial visibility, and runtime
capabilities:

```rust
use quickgui::{Rect, WindowBounds, WindowKind, WindowOptions};

let dialog = cx.open_window(
    ConfirmDelete::new(),
    WindowOptions::new("Confirm delete")
        .window_kind(WindowKind::Dialog)
        .window_bounds(WindowBounds::Windowed(Rect::new(220.0, 140.0, 560.0, 360.0)))
        .resizable(false),
);

cx.set_window_bounds(WindowBounds::windowed(240.0, 160.0, 720.0, 520.0))
    .expect("valid bounds");
cx.toggle_fullscreen().expect("native window");
cx.hide_window().expect("native window");
cx.show_window_handle(dialog).expect("queued child handle");
```

Event callbacks enqueue those mutations through `EventContext`. A render pass observes future
native state changes through `ViewContext`:

```rust
let state = cx.window_state();
```

Every `open_window` call makes the delivering window the new window's retained parent. Closing a
parent closes its descendants child-first and cancels their foreground tasks. On macOS, `Dialog`
is an AppKit sheet, `Floating` uses the floating level, and `PopUp` uses transient popup-menu level
and space behavior. `PopUp` is currently a Winit-created `NSWindow`, not a nonactivating `NSPanel`;
true anchored native panels remain a platform gap.

Title, bounds, move, resize, minimize, restore, zoom, fullscreen, visibility, movability,
resizability, minimizability, focus, attention, and close commands can target the current window or
a stable `WindowHandle`. Mutations are validated before retention, capped at 256 per event and
1,024 per effect cycle, and applied after the application callback releases its borrows. Calling
`cx.window_state()` declaratively observes native changes; it does not install a timer or polling
frame. See `cargo run --release --example window_controls`.

## macOS platform services

Event callbacks can start window-owned AppKit prompts and file panels, then await the result on the
existing main-thread foreground executor:

```rust
let response = cx.prompt(
    PromptLevel::Warning,
    "Save changes?",
    Some("Unsaved edits will be lost."),
    &[
        PromptButton::ok("Save"),
        PromptButton::new("Don't Save"),
        PromptButton::cancel("Cancel"),
    ],
).expect("valid native prompt");

cx.spawn(|async_cx: AsyncViewContext<MyView>| async move {
    let answer = response.await.ok();
    async_cx.update(move |view, cx| {
        view.answer = answer;
        cx.invalidate();
    }).await
}).expect("foreground capacity").detach();
```

System notifications are application-wide and use stable tags for replacement and dismissal:

```rust
let notification = SystemNotification::new(
    "background-build",
    "Build finished",
    "All checks passed.",
)
.action(SystemNotificationAction::new("open", "Open"));

cx.show_system_notification(notification)?;
// cx.dismiss_system_notification("background-build")?;
```

Native application events are configured once on `App`, outside any particular window:

```rust
App::new(view)
    .on_open_urls(|urls, cx| update_workspace(urls, cx))
    .on_reopen(|had_visible_windows, cx| reopen(had_visible_windows, cx))
    .on_system_wake(|cx| reconnect(cx))
    .on_system_notification_response(|response, cx| activate(response, cx))
    .run()?;
```

Those callbacks receive an application-wide `EventContext` with no current window. They can
update globals or entities and open new root windows; window-owned prompts and foreground tasks
still require a window callback. Native observers are installed only for callbacks the
application registered.

`prompt_for_paths(PathPromptOptions)` and `prompt_for_new_path(SavePathOptions)` return the same
single-use future shape; cancellation is `Ok(None)`. `open_url`, `open_path`, and `reveal_path`
project through `NSWorkspace`. A dropped response cancels a sheet that has already started, window
teardown cancels both the response task and native panel, related parent/child windows cannot stack
conflicting sheets, and unrelated windows remain independent. Requests, active dialogs, selected
path counts, and returned path bytes all have public hard limits. Unbound Cmd-W now goes through
AppKit's ordinary close path and still delivers `Event::CloseRequested`.

On macOS, notifications require a real `.app` bundle with a `CFBundleIdentifier`; an unbundled
`cargo run` process cannot use `UNUserNotificationCenter`. The first post makes one contextual
authorization request, waits for its result, and coalesces replacement tags in a 64-entry bounded
queue. Dismissal also removes a matching authorization-pending post. Action category retention is
capped at 64 distinct action sets; later notifications still post without actions instead of
growing native state. Open callbacks retain at most 256 URLs and 1 MiB total. None of these
services installs a polling timer or idle frame. See
`cargo run --release --example platform_services`.

## macOS window chrome

`HiddenInset` extends GPU content through a transparent titlebar while retaining native traffic
lights. Its implicit AppKit drag area is disabled; layout boxes explicitly opt into web-style
`drag` and `no-drag` regions:

```rust
App::new(view)
    .title_bar_style(TitleBarStyle::HiddenInset)
    .traffic_light_position(16.0, 13.0)
    .run()?;

div()
    .h(64.0)
    .app_region_drag()
    .child(button().app_region_no_drag().child("Refresh"))
```

The topmost declared app region wins, so interactive descendants use `.app_region_no_drag()`.
Drag regions do not receive element clicks or captured pointer gestures. Built-in overlay
scrollbars take input precedence over an ancestor drag region.

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

Ordinary text wraps at word boundaries by default and contributes its wrapped height to flex layout; use `.no_wrap()` where a single line is intentional. Controlled text fields and text areas use the same listener pattern:

Immutable rich text uses GPUI-shaped byte-range highlights while retaining one shaped buffer:

```rust
use quickgui::{Color, HighlightStyle, styled_text};

let source = "GPU cached · deprecated";
let rich = styled_text(source).with_highlights([
    (0..10, HighlightStyle::default()
        .font_semibold()
        .color(Color::rgb8(94, 234, 212))
        .background(Color::rgba8(13, 148, 136, 48))),
    (13..23, HighlightStyle::default()
        .color(Color::rgb8(248, 113, 113))
        .strikethrough()),
]);

div().w_full().text_lg().child(rich)
```

Ranges are sorted, non-overlapping UTF-8 byte offsets checked at the API boundary. Foreground,
font family/weight/italic, background, single or double underline, and strikethrough can vary per
range. Wrapping, bidirectional shaping, measurement, glyph painting, backgrounds, and decorations
all reuse one bounded Cosmic Text entry; decoration rectangles are generated only for visible
lines and use the existing ordered instanced-shape draw. Clones share the string and immutable run
table, each element accepts at most 4,096 non-empty highlights, and a background-color-only change
does not invalidate glyph shaping. See `cargo run --release --example styled_text`.

Plain and styled immutable text is selectable by default outside controls. Drag selection crosses
sibling text leaves in document order, double-click selects Unicode words, triple-click selects a
logical line, Shift extends the existing range, Cmd/Ctrl-C copies, and Cmd/Ctrl-A selects all
mounted immutable text. Buttons, pointer listeners, and drag sources inherit web-like
`user-select: none`; use `.user_select_text()` (or `.selectable()`) to opt a control subtree back in,
and `.user_select_none()` to suppress selection explicitly. Selection geometry reuses the retained
Cosmic Text buffer and does not enter its layout key. Cross-leaf clipboard text is joined by visual
line and capped at 8 MiB without splitting UTF-8.

Controlled text fields and text areas use the same listener pattern:

```rust
let edit_name = cx.input_listener("name", |this, value, cx| {
    this.name = value.into();
    cx.invalidate();
});
let submit_name = cx.submit_listener("name", |this, value, cx| {
    this.save_name(value);
    cx.invalidate();
});
let edit_notes = cx.input_listener("notes", |this, value, cx| {
    this.notes = value.into();
    cx.invalidate();
});

text_input(self.name.clone())
    .on_input(edit_name)
    .on_submit(submit_name)
    .max_length(32)
    .input_filter(|value| !value.contains('\t'))
    .invalid(self.name.trim().is_empty())
    .validation_message("Name is required")
    .placeholder("Type a name…")
    .accessibility_label("Name")
    .w_full()

text_area(self.notes.clone())
    .on_input(edit_notes)
    .placeholder("Write multiline notes…")
    .accessibility_label("Notes")
    .size(480.0, 180.0)
```

Input constraints run against the proposed complete value before retained text or undo history is
changed. Maximum length counts Unicode grapheme clusters and truncates paste and IME commits only
at grapheme boundaries. A rejected edit leaves the value, selection, composition backup, history,
and `on_input` listener untouched. Invalidity remains controlled application state: it adds a
paint-only invalid style and native accessibility metadata, and suppresses `.on_submit(...)`
without moving focus. Single-line Return submits the committed value once; Return in a text area
continues to insert a newline.

For browser-style forms, attach valid and invalid listeners to a semantic `form()`. Return in any
descendant single-line input and a `submit_button()` validate the nearest form:

```rust
let save = cx.form_submit_listener("profile", |this, event, cx| {
    this.save(event.value("name").unwrap_or_default());
    cx.invalidate();
});
let report = cx.form_invalid_listener("profile", |this, report, cx| {
    this.status = report.first().and_then(|issue| issue.message()).unwrap_or("Invalid").into();
    cx.invalidate();
});

form()
    .on_form_submit(save)
    .on_form_invalid(report)
    .child(
        text_input(self.name.clone())
            .id("name")
            .invalid(self.name.trim().is_empty())
            .validation_message("Name is required"),
    )
    .child(submit_button().child("Save"))
```

Valid submissions expose up to 256 document-ordered controlled values without copying their text.
Invalid reports retain up to 256 issues; each message is truncated safely at 4 KiB, disabled and
nested-form controls are excluded, and focus moves to the first focusable invalid control. Every
failed attempt replaces one assertive AccessKit live node, so an identical retry is announced
again. This costs one action-driven redraw and retains one announcement—there is no validation
polling or idle frame loop. Custom controls can request the same path with
`EventContext::submit_form(...)`.

Text areas preserve normalized newlines, wrap and hit-test visual lines, keep the caret visible in
both axes, and support Up/Down/Page navigation, macOS word/line/document shortcuts, drag
selection, IME composition, and the same native-style auto-hiding scrollbar as other scroll
containers. The renderer reuses one retained Cosmic Text layout for those operations; scrolling
does not start an idle frame loop.

The same bounded `StyledText` value can drive a controlled attributed input without a parallel
editor widget:

```rust
let edit_code = cx.input_listener("code", |this, value, cx| {
    this.code = value.into();
    cx.invalidate();
});

let value = styled_text(self.code.clone()).with_highlights(syntax_runs(&self.code));

styled_text_area(value)
    .on_input(edit_code)
    .font_family(FontFamily::Monospace)
    .no_wrap()
    .size(640.0, 240.0)
```

Attributed fields and areas use the run table for shaping, measurement, caret placement, pointer
hit testing, selection, IME geometry, glyphs, backgrounds, and decorations. Accepted local edits
shift, split, and merge runs until the controlled view supplies its next table; undo and redo
restore the corresponding styles. Run metadata and named-family bytes count toward each input's
existing 512 KiB history budget, the live table remains capped at 4,096 runs, and highlighting
runs only after application value changes—never on idle frames.

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

Non-blocking application futures use a window-owned foreground executor. The future starts on the
next event-loop turn; dropping its `Task` handle cancels it, while `detach()` keeps it alive until
completion or window close. View access after an `await` is explicitly fallible and deferred until
the current future poll has released executor state:

```rust
let task = cx.spawn(|task_cx: AsyncViewContext<MyView>| async move {
    task_cx.sleep(Duration::from_millis(500)).await?;
    task_cx.update(|view, cx| {
        view.ready = true;
        cx.invalidate();
    }).await
})?;
```

`sleep` contributes one exact `ControlFlow::WaitUntil` deadline; it does not create a timer thread,
polling frame, or idle wakeup. The runtime caps live tasks at 1,024 per window and 4,096 per
application, polls at most 256 ready futures per event-loop turn, and independently bounds queued
updates and timers. See `cargo run --release --example foreground_tasks`.

Blocking or CPU-heavy application work uses the separate bounded background pool. Completion
returns to the owning window once through the event loop, so a clean UI does not poll:

```rust
cx.spawn_background(load_data, |this, result, cx| {
    this.data = result.ok();
    cx.invalidate();
})?;
```

For a one-off paint transition such as hiding an overlay, `cx.request_repaint_at(deadline)` adds
one exact event-loop deadline; it does not enable display-rate rendering.

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

Reusable command palettes use `PickerState<T>`. The state owns bounded fuzzy matches, highlighted
UTF-8 ranges, disabled-aware selection, and a `VirtualList`; the application keeps command behavior
as ordinary typed values:

```rust
let palette = PickerState::new([
    PickerItem::new("Save File", AnyAction::new(Save)).shortcut("⌘S"),
    PickerItem::new("Toggle Sidebar", AnyAction::new(ToggleSidebar))
        .keywords("view explorer panel"),
])?;

App::new(view).bind_keys(picker_key_bindings());

// Inside View::render:
self.palette.element(
    cx,
    "command-palette",
    |view| &mut view.palette,
    |view, action, cx| {
        view.palette_open = false;
        cx.focus(view.editor_focus);
        cx.dispatch_any_action(action);
        cx.invalidate();
    },
)
```

The default contextual bindings cover Up/Down, Page Up/Down, Cmd/Ctrl-Up/Down, and Return without
stealing those keys from ordinary inputs. Escape uses the normal dismissible-overlay path and
restores focus. One picker accepts at most 65,536 items, 16 MiB of searchable metadata, 128 query
graphemes and 4 KiB of query text, and 2,048 ranked results; only the configured visible rows are
declared. Matching runs
only after `set_query` or `set_items`, uses a bounded top-result partition, and schedules no timer or
idle frame. Run `cargo run --release --example command_palette`; benchmark the 25,000-item query
path with `cargo bench --bench picker`.

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

Tooltips are ordinary bounded QuickGUI element trees owned by the framework:

```rust
button()
    .tooltip("Save the current file")
    .child("Save");

div().tooltip(
    Tooltip::new(custom_tooltip_content())
        .placement(AnchorPlacement::Right),
);
```

A tooltip waits for one exact 500 ms deadline by default, paints without rebuilding the application
view, stays visible for keyboard focus as well as pointer hover, and schedules nothing while
stationary. Each tooltip is capped at 256 elements and each window indexes at most 4,096 tooltip
declarations. Text tooltips also become the trigger's native accessibility description.

Web-style context menus use a targeted secondary-click listener and the same edge-aware placement
engine at the original logical pointer position:

```rust
let context = cx.context_menu_listener("row", |this, event, cx| {
    this.menu_position = Some(event.position);
    cx.invalidate();
});

div().id("row").on_context_menu(context);

overlay()
    .anchor_at(menu_position, AnchorPlacement::BottomStart)
    .on_dismiss(dismiss)
    .accessibility_role(AccessibilityRole::Menu)
    .child(menu_items)
```

Nested targets bubble to their nearest retained ancestor listener, while overlay blockers prevent
click-through. Dismissible menus retain outside-click/Escape dismissal and focus restoration. Run
`cargo run --example tooltips_context_menu` for the combined macOS example.

Typed drags and native files, text, and URLs use the same exact-type listener path:

```rust
let source = cx.drag_listener("source", |_this, _event, _cx| {
    Drag::new(Card { id: 7 }).preview(card_preview())
});
let cards = cx.drop_listener("target", |this, card: &Card, event, cx| {
    this.accept(card, event.position);
    cx.invalidate();
});
let files = cx.drop_listener("target", |this, files: &DroppedFiles, _event, cx| {
    this.open(files.paths());
    cx.invalidate();
});
let native_text = cx.drop_listener("target", |this, value: &DroppedText, _event, cx| {
    this.insert(value.as_str());
    cx.invalidate();
});
let native_url = cx.drop_listener("target", |this, value: &DroppedUrl, _event, cx| {
    this.open_url(value.as_str());
    cx.invalidate();
});

div().on_drag(source).dragging(|style| style.bg(dragging_color));
div()
    .on_drop(cards)
    .on_drop(files)
    .on_drop(native_text)
    .on_drop(native_url)
    .can_drop::<Card>(|card| !card.locked)
    .drag_over(|style| style.border(2.0, accent));
```

On macOS, the original arbitrary Rust value automatically remains available when a drag crosses
into another QuickGUI window. Add a public representation only when native applications should
also see existing files/directories, plain text, or an absolute URL:

```rust
use quickgui::{ExternalDragUrl, FileDragPaths};

let files = FileDragPaths::files([manifest_path]);
let project = ExternalDragUrl::new("https://github.com/egoist/quickgui")?;

let source = cx.drag_listener("source", move |_this, _event, _cx| {
    Drag::new(Card { id: 7 })
        .preview(card_preview())
        .external_files(files.clone())
});

let text_source = cx.drag_listener("text", move |_this, _event, _cx| {
    Drag::new(Card { id: 8 }).external_text("QuickGUI native text drag")
});

let url_source = cx.drag_listener("url", move |_this, _event, _cx| {
    Drag::new(Card { id: 9 }).external_url(project.clone())
});
```

A primary-button gesture crosses a two-point threshold before becoming a drag, which preserves
ordinary clicks. Only one payload and one preview tree are retained per window. Compatible targets
are selected by Rust `TypeId`; overlapping incompatible targets are skipped without violating
pointer blockers. Preview motion repaints the retained scene without rerunning `View::render` or
Flexbox, and stopping the pointer schedules no work. Escape, focus loss, or button release tears
the session down immediately.

Leaving a macOS window promotes the original `Arc<dyn Any>` into an application-wide registry
capped at 256 simultaneous sessions. AppKit receives only a random UUID under QuickGUI's private
pasteboard type; the Rust value is neither serialized nor copied. A destination resolves it into
the same exact-type listener path and reports `DragOrigin::CrossWindow { window, source }`. If a
writer also exposes text, URL, or file formats, selection is one topmost-first pass: the highest
compatible element wins before its preferred offered type, and a live-tree check still precedes
delivery. Snapshot storage is capped at 8,192 exact listener acceptances and 4,096 relevant target
or blocker regions, failing closed past either bound.

Finder drops are grouped into one `DroppedFiles` value, capped at
4,096 paths, and their enter notification is coalesced at the event-loop boundary. An outbound
drag stays on the internal GPU path until the pointer exits the viewport, then starts one AppKit
session and reports its result through `Event::ExternalDragEnded`. Files become `NSURL`
pasteboard writers; the application supplies each path's directory bit, avoiding synchronous
filesystem metadata, while construction caps the payload at 4,096 paths, 16 KiB per encoded path,
and 8 MiB total path storage. Plain text becomes an `NSString`, retains at most 1 MiB, and truncates
only at a UTF-8 boundary. URLs become `NSURL` writers, reject missing/invalid absolute schemes,
whitespace, controls, and values above 16 KiB before promotion. The source advertises copy only, so
QuickGUI never implicitly relocates or mutates application-owned data. Inbound text and URLs use
the aliases `DroppedText` and `DroppedUrl`, so the same exact type can make a round trip through a
native app. AppKit registers those pasteboard formats only while matching listeners exist. Its
synchronous cursor decision uses a topmost-first snapshot capped at 4,096 relevant targets and
pointer blockers; `can_drop` gates both the native cursor and eventual live-tree delivery. Hover
motion is coalesced to one pending event-loop wake, and UTF-8 extraction retains at most 1 MiB of
text or rejects URLs above 16 KiB. Winit continues to own and receive every Finder file selector.
There is no polling, animation loop, or deadline while the pointer is stationary. See
`cargo run --release --example drag_drop` for public formats and
`cargo run --release --example multi_window` for arbitrary cross-window values.

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

The live Hacker News example combines bounded background fetching, full wrapped comments,
independent draggable overlay scrollbars, custom hidden-inset chrome, and explicit app regions:

```console
cargo run --example hacker_news
```

## Run the stress test

```console
cargo run --release --example stress_scroll
```

The demo renders a selectable 100,000-row list while mounting only the visible rows plus two rows
of overscan. It supports trackpad/wheel scrolling, a captured overlay scrollbar that expands on
hover and auto-hides, hover and pressed feedback, click listeners, arrow/page navigation, Home/End,
and in-window CPU/render telemetry.

Run validation and the CPU-side list benchmarks with:

```console
cargo test --all-targets
cargo bench --bench virtual_list
cargo clippy --all-targets --all-features -- -D warnings
```

## Design status

Implemented now:

- macOS/Windows/Linux backend selection through Winit 0.30 and WGPU 30;
- sleeping heterogeneous multi-window runtime with stable handles, close interception, targeted focus/close/invalidation, resize, DPI, pointer, wheel, keyboard, full IME preedit/commit routing, and focus events;
- parent-owned normal, dialog-sheet, floating, and transient popup roles with retained restore bounds, declarative native-state snapshots, hidden first-frame creation, child-first teardown, and bounded targeted runtime window commands;
- window-owned macOS `NSAlert`, `NSOpenPanel`, and `NSSavePanel` futures with cancellation-safe lifecycle and hard request/result bounds, plus `NSWorkspace` URL/open/reveal actions and default Cmd-W close routing;
- bounded macOS system notifications with tag replacement, actions, dismissal, one-shot authorization, and application-wide response callbacks, plus opt-in URL-open, Dock-reopen, and system-wake lifecycle callbacks;
- main-thread `Entity<T>`/`WeakEntity<T>` shared state with retained per-window observation, coalesced bounded notification fan-out, and automatic conditional unsubscribe, plus typed `EventEmitter` delivery with RAII `Subscription` lifetimes and bounded deterministic queues;
- main-thread application globals with exact typed access, conditional per-window observation, RAII change subscriptions, deterministic deferred delivery, and bounded notification fan-out;
- cancellable main-thread foreground futures with `Task` drop/detach semantics, fallible typed async view updates, exact event-loop timers, structured window-close cancellation, and hard task/poll/update/timer bounds;
- compatible windows share WGPU device/queue ownership and one lazy bounded image worker pool while retaining independent surfaces, schedulers, input state, and bounded render caches;
- declarative elements, retained Taffy Flexbox and CSS Grid, absolute positioning, clipping, inherited text styles, and Tailwind-like helpers;
- native-style overlay scrollbars with 12-point hit tracks, captured thumb/track dragging, hover expansion, and one-shot auto-hide deadlines;
- macOS hidden-inset titlebars with native traffic-light positioning and explicit topmost `drag`/`no-drag` app regions;
- ordered `z_index` stacking layers plus portal-style overlays with anchor flip/shift, pointer blocking, outside/Escape dismissal, and focus restoration;
- arbitrary delayed GPU tooltips with keyboard accessibility and exact one-shot scheduling, plus bubbling secondary-click listeners and cursor-point anchored context menus;
- macOS `NSView` children composed between the base and transparent overlay WGPU surfaces, with atomic first-frame reveal, keyed lifetime, clipping, sizing, first-responder handoff, and merged AccessKit/AppKit accessibility routing;
- keyed hover/active/focus/click state, captured pointer gestures, Tab traversal, keyboard button activation, and type-safe `ViewContext` listeners;
- typed actions, non-focusable focus scopes, focused-path bubbling, contextual keymaps, programmatic command dispatch, and replay-safe multi-stroke bindings;
- reusable command-palette pickers with bounded fuzzy matching, UTF-8-safe highlights, contextual keyboard navigation, visible-only rows, focus restoration, and heterogeneous typed-action dispatch;
- typed drag/drop with GPU previews and paint-only source/target states, arbitrary process-local values crossing macOS windows without serialization, plus bounded inbound/outbound file, text, and URL formats;
- native macOS application menus with nested/system menus, contextual key equivalents, focused command validation, dynamic replacement, checked/disabled items, and AppKit responder-chain actions;
- controlled plain or attributed single-line and wrapped multiline text editing with grapheme/word/line navigation and deletion, visual-line caret movement, mouse caret and drag selection, two-axis scrolling, copy/cut/paste, IME composition, and bounded text-plus-style undo/redo history;
- semantic browser-style forms with nearest-form Return and submit-button routing, shared controlled field data, bounded document-order validation reports, deterministic first-invalid focus, and one-shot AccessKit live announcements;
- retained `StyledText` with bounded Unicode-safe byte ranges, per-run font/foreground/background/decorations, wrapped BiDi shaping in one cached buffer, visible-only decoration geometry, and document-order selection shared with ordinary text;
- AccessKit trees with semantic roles, labels, disabled/selected state, native focus/click actions, and editable or immutable text-selection actions;
- linear-light colors, premultiplied blending, analytic rounded rectangles, borders, CSS-ordered drop/inset shadows, and HiDPI rendering;
- static, asynchronous, and animated PNG/JPEG/WebP/GIF/RGBA images with intrinsic layout, all web `object-fit` modes, rounded clipping, GPU grayscale, delayed loading/error fallbacks, per-element playback, Reduce Motion, stable identity, and hard-bounded CPU/GPU caches;
- retained SVG/SVGZ icons and tessellated fill/stroke paths with intrinsic layout, web `object-fit`, transforms, dashes, arcs, two-stop gradients, analytic boundary antialiasing, and scoped custom canvas painting;
- retained validated WGSL rectangle effects with framework-owned clipping and blending, bounded per-window pipeline caching, four per-instance parameter vectors, and one triple-buffered instanced upload;
- Cosmic Text/Glyphon shaping, fallback, rasterization, atlas reuse, and bounded text-layout retention;
- fixed-height virtualization, clamped scrolling, scrollbar math, and performance telemetry.

Still required before calling it a production-complete general GUI framework or GPUI-equivalent
application platform:

- true anchored/nonactivating native popup panels on macOS;
- advanced platform input such as pressure and gesture events, plus deterministic headless and
  visual application test contexts;
- Windows/Linux native menu projection and cross-window native drag promotion;
- Windows/Linux runtime and visual CI, plus a portable benchmark matrix.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the renderer and ownership model.

## Why this renderer

Most desktop UI pixels are rectangles, glyphs, icons, images, and modest vector paths. Dedicated data-driven pipelines minimize CPU preparation and draw calls for that workload. QuickGUI uses retained Lyon tessellation plus a narrow WGPU path pipeline instead of making a general compute vector renderer part of every frame; the bounded application-shader path stays opt-in for specialized effects.

The design is informed by the same proven ideas documented by [Zed's GPU UI architecture](https://zed.dev/blog/videogame), while changing the scheduling and ownership choices that matter for low CPU: no permanent redraw loop, frame-boundary input coalescing, retained layout between view changes, and bounded text/list caches. Sublime Text 4 likewise documents GPU compositing as the route to fluid high-DPI UI with lower power use in its [GPU rendering notes](https://www.sublimetext.com/docs/gpu_rendering.html).

## License

MIT.
