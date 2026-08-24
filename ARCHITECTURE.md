# QuickGUI architecture

QuickGUI separates application declaration, retained interaction state, scene preparation, and GPU submission. That separation keeps ergonomic view code away from renderer details without rebuilding every layer for every pointer or wheel event.

```text
Winit + AccessKit events
    │
    ├─ coalesced wheel input ───────────────┐
    ├─ retained hover/focus/input state ────┤
    └─ app mutation → explicit invalidate ──┤
                                            ▼
                                  retained Element tree
                                  + keyed listeners/state
                                            │ app changes only
                                            ▼
                                      Taffy flex layout
                                            │ layout changes only
                                            ▼
                                      reusable Scene
                                ordered plane/z layers
                              ┌─────────────┴─────────────┐
                              ▼                           ▼
                   instanced quad pipeline      cached Glyphon text
                              └─────────────┬─────────────┘
                                            ▼
                         ┌──────── base WGPU surface
                         ├──────── macOS native NSViews (when mounted)
                         └──────── transparent overlay WGPU surface
```

## Frame scheduling

The event loop runs with `ControlFlow::Wait`. Invalidation queues at most one Winit redraw. Multiple wheel events accumulate into one logical delta before the redraw handler runs. An occluded or zero-sized window is skipped and is not placed in a retry loop.

Hover, pressed state, and retained scroll-container offsets only mark paint dirty. They reuse the existing element declaration and Taffy layout. An application mutation calls `EventContext::invalidate()`, which rebuilds the declarative tree at the next redraw. `request_animation_frame()` is explicit and keeps rebuilding only while a view requests it.

## Element identity and ownership

The application owns its `View`. Each render returns a declarative `Element` tree. Explicit `.id(...)` keys are reserved and validated before generated path IDs are assigned, so duplicate developer keys are errors while hash collisions for unkeyed nodes are deterministically probed around.

An element ID owns:

- hover and pressed paint state;
- focus identity and keyboard traversal position;
- click or controlled-input listener lookup;
- scroll-container offset;
- text-input caret, selection, composition, and horizontal offset;
- text-layout and glyph-cache identity.

`ViewContext::listener` stores callbacks in a registry parameterized by the concrete view type. `Element::on_click` only carries the stable ID, keeping the element tree non-generic and compact while callbacks can still mutate `&mut Self` without `Rc<RefCell<_>>` application state.

## Layout

Taffy implements Flexbox and intrinsic measurement. Text leaf measurement calls the renderer's shared Cosmic Text cache, so layout and paint do not shape the same string twice. Typography is inherited during tree construction; hover and active variants are deliberately paint-only so a pointer move cannot trigger relayout.

Scrollable nodes retain offsets outside the declaration tree. Fixed-height `VirtualList` is a separate O(1) range calculator for very large data sets; it never allocates in `visible_rows()` and mounts only viewport rows plus configured overscan.

## Rendering

The current renderer has two specialized pipelines:

1. Quads use six shader-generated vertices and an instance containing logical bounds, fill, inside border, radius, and clip. Rounded-corner antialiasing is analytic in WGSL. All visible quads share one upload and each non-empty stacking layer is submitted in one draw call.
2. Text uses Cosmic Text for Unicode shaping/fallback and Glyphon for Swash rasterization plus atlas rendering. Stable `TextId`s prevent reshaping unless content, width, metrics, family, weight, wrap mode, or display scale changes.

Colors are stored in linear-light space. Eight-bit constructors decode sRGB on the CPU, and the sRGB surface performs the output transfer. Quad output is premultiplied before using premultiplied-alpha blending. Text wraps at word boundaries by default; intrinsic measurements reserve one physical pixel before a max-content width is fed back as a wrap constraint, preventing rounding-only reflow between layout and paint.

Quad instance uploads rotate across three GPU buffers. Capacity grows geometrically. Text layouts are age-evicted every frame and have a hard cap of 256 retained areas; Glyphon's atlas is trimmed after presentation. The deliberately small layout cache keeps long, disjoint scrolling from retaining whole off-screen text buffers while still covering several nearby viewports.

The scene groups primitives by `(plane, z_index)` and retains at most 16 inactive layer buffers.
Quads and text are interleaved per layer, while every overlay layer is ordered after every base
layer. Glyphon renderers are shared by text-bearing layers and capped at eight retained instances.
An ordinary window still uses one WGPU surface. A macOS window with native children creates its
transparent overlay surface only when overlay content or input first becomes active, then retains
that swapchain for reuse. Recreating it on every close/open cycle is intentionally avoided because
Core Animation may keep old IOSurface drawables alive beyond the Rust `Surface` lifetime.

## Overlays and native composition

Anchored overlays resolve after natural Flexbox layout so their target bounds are stable. The
placement algorithm prefers the requested side, flips when the opposite side has more available
space, tries alternate alignment, shifts to the viewport margin, and pins oversized surfaces to
that margin. Portal overlays escape ancestor clips. Hit, scroll, hover, and dismissal regions use
the same plane/z/source order as paint, preventing click-through to visually covered content.

On macOS, each native child is retained by identity and mounted inside two flipped AppKit wrappers:
an outer clipping view and an inner rounded-corner view. Bounds are expressed in QuickGUI logical
points. The parent Winit/WGPU view remains the base surface and a transparent sibling is always
ordered above native children. Its hit test is disabled while no overlay is interactive; while an
overlay is open it forwards input to Winit so outside-click dismissal cannot activate the native
control underneath.

AppKit mouse-down monitoring only queues a redraw for the affected window. At that event boundary,
QuickGUI compares the real first responder with the Winit view and clears stale semantic focus.
Focusing a QuickGUI element makes the Winit view first responder again. This preserves an idle
event loop while giving native controls normal keyboard and IME ownership.

The surface uses guaranteed FIFO presentation and a two-frame latency hint. A latency of one is intentionally not the default because WGPU documents that it prevents CPU/GPU overlap and prioritizes latency over throughput.

## Performance rules

Changes should preserve these invariants:

- no redraw request merely because a window exists;
- no full data-set walk in scrolling or painting;
- no text shaping keyed by screen position;
- no unbounded retained cache;
- no per-primitive GPU draw call;
- no blocking wait for GPU completion on the UI thread;
- no layout-affecting hover style.

CPU frame time, primitive counts, text cache hits, and draw-call counts are exposed through `FrameMetrics`. These are application-side timings through queue submission, not GPU timestamps or end-to-end display latency.

## Native input and accessibility

The retained tree owns semantic focus and exposes it through AccessKit before the macOS window becomes visible. Pointer focus, Tab/Shift-Tab traversal, Space/Return button activation, programmatic focus handles, and accessibility actions all update the same focus owner.

Single-line input keeps UTF-8 byte ranges internally but moves and deletes at Unicode grapheme boundaries. Cosmic Text provides visual caret hit testing and bidirectional selection spans. Composing text is retained separately from the controlled value, Winit preedit cursor offsets remain byte-indexed, and the native IME candidate rectangle follows the painted caret. AccessKit receives a stable `TextRun` child, grapheme character lengths, editable value, and native text-selection state. Clipboard objects are created lazily so applications that never copy or paste pay no startup cost.

## Current boundaries

This milestone establishes the performance architecture, ordered overlays, macOS native child
composition, semantic focus/accessibility, and a production-oriented single-line editing path. The
AccessKit adapter currently owns the Winit view's virtual accessibility children, so embedded native
AppKit accessibility subtrees are not yet merged into the same navigation tree. A complete platform
toolkit also still needs rich and multiline editing, images, shadows, full menus, focus
scopes/keymaps, drag/drop, multi-window ownership, and broader platform acceptance. Those features
should extend the retained tree and narrow renderer rather than bypass its scheduling and cache
invariants.
