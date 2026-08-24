# QuickGUI

QuickGUI is a damage-driven, GPU-accelerated GUI foundation for Rust desktop apps. It combines a GPUI-style fluent view API, Taffy flexbox layout, a purpose-built WGPU renderer, cached Unicode text shaping, and bounded virtual scrolling.

The hot path is intentionally small:

- windows use `ControlFlow::Wait` and do no application rendering while clean;
- wheel events are coalesced at the OS frame boundary;
- retained element trees can repaint hover and scroll state without rebuilding the view or rerunning flex layout;
- rectangles, borders, and rounded corners are one instanced GPU draw call;
- text shaping and rasterization are retained by stable element IDs;
- dynamic instance buffers are triple-buffered rather than overwritten while the GPU may still read them;
- `VirtualList` computes the mounted range in O(1), independent of item count.

The current milestone is a runnable framework core, not a claim that every production widget already exists. The macOS runtime and 100,000-row demo have live visual acceptance. Windows and Linux backends are configured through Winit/WGPU but still need CI and runtime acceptance on those operating systems.

## View API

Views use regular Rust with JSX-like composition, Tailwind-style spacing, inherited typography, flexbox, stable identities, and view-local listeners:

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
- sleeping single-window runtime with resize, DPI, pointer, wheel, keyboard, IME commit, and focus events;
- declarative elements, Taffy flexbox, absolute positioning, clipping, inherited text styles, and Tailwind-like helpers;
- keyed hover/active/click state and type-safe `ViewContext` listeners;
- linear-light colors, premultiplied blending, analytic rounded rectangles, borders, and HiDPI rendering;
- Cosmic Text/Glyphon shaping, fallback, rasterization, atlas reuse, and bounded text-layout retention;
- fixed-height virtualization, clamped scrolling, scrollbar math, and performance telemetry.

Still required before calling it a production-complete general GUI framework:

- accessibility trees and semantic controls;
- editable text widgets, selection, clipboard, and full IME composition;
- image/SVG/path primitives, shadows, and ordered overlay layers;
- focus traversal, menus, popovers, drag/drop, and multi-window APIs;
- Windows/Linux runtime and visual CI, plus a portable benchmark matrix.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the renderer and ownership model.

## Why this renderer

Most desktop UI pixels are rectangles, glyphs, icons, and images. Dedicated data-driven pipelines minimize CPU preparation and draw calls for that workload. Vello remains a good future opt-in path backend, but its own project currently describes it as alpha and lists glyph caching and GPU-memory allocation among active work; QuickGUI therefore does not make a general compute vector renderer part of every frame.

The design is informed by the same proven ideas documented by [Zed's GPU UI architecture](https://zed.dev/blog/videogame), while changing the scheduling and ownership choices that matter for low CPU: no permanent redraw loop, frame-boundary input coalescing, retained flex layout between view changes, and bounded text/list caches. Sublime Text 4 likewise documents GPU compositing as the route to fluid high-DPI UI with lower power use in its [GPU rendering notes](https://www.sublimetext.com/docs/gpu_rendering.html).

## License

MIT.
