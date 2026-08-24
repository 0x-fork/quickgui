# QuickGUI

QuickGUI is a damage-driven, GPU-accelerated GUI foundation for Rust desktop apps. It combines a GPUI-style fluent view API, Taffy flexbox layout, a purpose-built WGPU renderer, cached Unicode text shaping, native accessibility, and bounded virtual scrolling.

The hot path is intentionally small:

- windows use `ControlFlow::Wait` and do no application rendering while clean;
- wheel events are coalesced at the OS frame boundary;
- retained element trees can repaint hover and scroll state without rebuilding the view or rerunning flex layout;
- rectangles, borders, and rounded corners share one instance upload and one draw per non-empty stacking layer;
- text shaping and rasterization are retained by stable element IDs;
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

`Scene` and `Quad` remain public as the lower-level escape hatch for specialized widgets and future custom paint elements. Ordinary application views should use `Element` builders.

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

The demo renders a selectable 100,000-row list while mounting only the visible rows plus two rows of overscan. It supports trackpad/wheel scrolling, hover and pressed feedback, click listeners, arrow/page navigation, Home/End, a proportional scrollbar, and in-window CPU/render telemetry.

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
- keyed hover/active/focus/click state, Tab traversal, keyboard button activation, and type-safe `ViewContext` listeners;
- typed actions, non-focusable focus scopes, focused-path bubbling, contextual keymaps, programmatic command dispatch, and replay-safe multi-stroke bindings;
- native macOS application menus with nested/system menus, contextual key equivalents, focused command validation, dynamic replacement, checked/disabled items, and AppKit responder-chain actions;
- controlled single-line text input with grapheme-safe movement/deletion, mouse caret and drag selection, horizontal scrolling, copy/cut/paste, and bounded undo/redo history;
- AccessKit trees with semantic roles, labels, disabled/selected state, native focus/click actions, and editable value/selection actions;
- linear-light colors, premultiplied blending, analytic rounded rectangles, borders, and HiDPI rendering;
- Cosmic Text/Glyphon shaping, fallback, rasterization, atlas reuse, and bounded text-layout retention;
- fixed-height virtualization, clamped scrolling, scrollbar math, and performance telemetry.

Still required before calling it a production-complete general GUI framework:

- rich and multiline text editing, word navigation, and input validation hooks;
- image/SVG/path primitives and shadows;
- command-palette widgets, drag/drop, multi-window APIs, and Windows/Linux native menu projection;
- Windows/Linux runtime and visual CI, plus a portable benchmark matrix.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the renderer and ownership model.

## Why this renderer

Most desktop UI pixels are rectangles, glyphs, icons, and images. Dedicated data-driven pipelines minimize CPU preparation and draw calls for that workload. Vello remains a good future opt-in path backend, but its own project currently describes it as alpha and lists glyph caching and GPU-memory allocation among active work; QuickGUI therefore does not make a general compute vector renderer part of every frame.

The design is informed by the same proven ideas documented by [Zed's GPU UI architecture](https://zed.dev/blog/videogame), while changing the scheduling and ownership choices that matter for low CPU: no permanent redraw loop, frame-boundary input coalescing, retained flex layout between view changes, and bounded text/list caches. Sublime Text 4 likewise documents GPU compositing as the route to fluid high-DPI UI with lower power use in its [GPU rendering notes](https://www.sublimetext.com/docs/gpu_rendering.html).

## License

MIT.
