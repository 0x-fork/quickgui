# View API and layout

[Documentation index](README.md)

## View API

Views use regular Rust with JSX-like composition, composable Tailwind-style spacing, inherited typography, Flexbox and CSS Grid, stable identities, and view-local listeners:

```rust
use quickgui::{
    Application, Color, EventContext, IntoElement, View, ViewContext, WindowOptions, div, text,
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
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("Counter").size(480.0, 320.0),
            Counter { count: 0 },
        );
    })
}
```

`Scene`, `Quad`, `Shadow`, `PathPrimitive`, `ImagePrimitive`, `SvgPrimitive`, and
`CustomShaderPrimitive` remain public as the lower-level escape hatch for specialized widgets.
Ordinary application views should use `Element` builders or the scoped `canvas(...)` painter.

Typography cascades through ordinary containers. `text_left`, `text_center`, and `text_right` match
GPUI's alignment helpers; `text_justify` adds the web paragraph case. Alignment affects shaping only
when the assigned width matters, so left-aligned no-wrap labels keep the existing width-independent
cache fast path. `whitespace_normal`/`whitespace_nowrap`, all three ellipsis placements,
`truncate`, and `line_clamp` provide the corresponding web/GPUI overflow vocabulary. Font slant,
complete `Font` values, independently inherited OpenType features and ordered fallback families,
solid/wavy single or double underline, 0/1/2/4/8-point thickness, underline color, strikethrough,
and explicit decoration reset inherit through the same retained text style; see
[Text, editing, and forms](text-and-forms.md) for composition and selection semantics.

`hidden()` is GPUI/CSS `display: none`: its complete subtree leaves layout, paint, hit testing,
focus, accessibility, image resolution, container-query callbacks, and animation scheduling.
`invisible()` keeps the layout box and controlled input state warm but suppresses paint and runtime
interaction for the subtree; `visible()` restores it without changing `block`, `flex`, or `grid`.

`opacity(value)` clamps finite values to `0.0..=1.0` and applies to the complete subtree. Nested
values multiply, including rich text, images, SVGs, paths, canvas/custom-shader primitives, and
macOS native child views. Like CSS and GPUI, opacity does not remove layout, hit testing, focus, or
accessibility; use `hidden()` or `invisible()` when those semantics are wanted. Opacity is carried
as paint data, so changing it does not reshape text, rerasterize SVG masks, or allocate an
offscreen group texture.

Hover, active, focus, validation, and drag-state variants are paint-only. Add
`.transition(Duration::from_millis(140))` to interpolate their colors, border, radius, inherited
text color, opacity, and bounded shadow list without rebuilding the view or rerunning Taffy. Layout
values use `AnimationExt::with_animation`; see [Declarative motion](animations.md).

## Flexbox and spacing

The fluent surface includes GPUI's everyday flex vocabulary rather than requiring direct
Taffy mutation:

```rust
div()
    .flex_row()
    .items_center()
    .justify_between()
    .gap_x_3()
    .gap_y_2()
    .child(sidebar.w(192.0).flex_none())
    .child(content.min_w(0.0).flex_1())
    .child(inspector.flex_initial().self_stretch())
```

`flex_row_reverse`/`flex_col_reverse`, `flex_auto`, `flex_initial`, `flex_none`, an explicit
`flex_basis`, sanitized grow/shrink factors, baseline and per-item alignment, wrapped-content
alignment, `justify_around`/`justify_evenly`, and `aspect_ratio`/`aspect_square` all map directly to
the retained Taffy style. These declarations allocate no runtime object and schedule no frame.

Margins use web names: `m`, `mx`, `my`, `mt`, `mr`, `mb`, and `ml`. Finite negative values are
accepted, the matching `_auto` helpers participate in CSS auto-margin distribution, and common
four-point scale suffixes run from `_0` through `_32`. Axis-specific gaps compose independently.
For example, `.max_w(700.0).mx_auto()` centers a bounded content column without an extra wrapper.
Run the focused gallery with:

```console
cargo run --release --example flex_layout
```

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

## Container queries

Use `container_query` when a reusable view must respond to the box its parent actually assigned,
instead of the whole window size:

```rust
use quickgui::{container_query, div, text};

container_query(|size| {
    if size.width < 480.0 {
        div().flex_col().child(text("Compact layout"))
    } else {
        div().grid().grid_cols(3).child(text("Wide layout"))
    }
})
```

The query fills its parent by default; ordinary sizing helpers such as `.w(320.0)`, `.flex_1()`,
and `.max_w(...)` can refine that box. The box participates in its parent's Taffy layout as a
leaf. Only after its size is known does QuickGUI invoke the callback and lay the returned element
out as an independent root inside that fixed size. Callback contents therefore cannot change the
query's intrinsic size or create a layout feedback loop.

Within one retained declaration, an unchanged assigned size reuses the existing callback subtree.
A retained relayout invokes only affected callbacks, preserves stable declarative-animation IDs,
and creates no timer, polling source, GPU resource, or idle frame. Queries work inside normal
views, tooltips, and drag previews; detached surfaces keep their existing pointer-passive contract.
One window is hard-limited to 1,024 mounted queries and 16 nested query levels.

The current callback deliberately receives `Size` only. Capture cloneable entities, globals, and
listeners from the surrounding `View::render` declaration when responsive contents need
application state. Query contents come exclusively from the callback, so `.child(...)` and
`.children(...)` on the query itself are rejected.

Run the live responsive example and resize through its compact, two-column, and three-column
breakpoints:

```console
cargo run --release --example container_queries
```

## Variable-height lists

`VirtualList` remains the allocation-free O(1) choice for uniform rows. Wrapped comments, chat
messages, logs, and other differently sized items use intrusive `ListState` instead:

```rust
use quickgui::{ListState, div, text};

// Store this on the view. The estimate sizes unmeasured offscreen content.
let comments = ListState::new(comment_count, 96.0).with_overscan(3);

// In render, seed the known viewport and mount only its bounded range.
comments.set_viewport_size(cx.size().width, cx.size().height);
let visible = comments.visible_rows();
let rows = comments.render_rows(visible.range, |index| {
    div()
        .w_full()
        .px_4()
        .py_3()
        .child(text(comment_text(index)).w_full().wrap())
});

div()
    .relative()
    .size_full()
    .overflow_hidden()
    .variable_virtual_scroll(&comments)
    .child(rows)
```

Rows stay in one normal Flexbox column, so their real wrapped heights stack correctly in the first
layout that sees them. Taffy reports those heights into a sparse 64-item metric index; a changed
measurement requests one correcting declaration, while settled rows and scrollbar hover remain
paint-only. The logical top item and pixel inset survive measurement changes. Width changes
invalidate measurements, and `remeasure_items(range)` handles content changes without throwing
away unaffected blocks.

`set_item_count` preserves unchanged prefix measurements; use `reset` when item identity changes.
Bottom-aligned transcripts can combine `ListAlignment::Bottom` and `FollowMode::Tail`. Cloned
`ListState` values intentionally share scroll and measurement state, so mount one shared state as
one list.
