# QuickGUI architecture

QuickGUI separates application declaration, retained interaction state, scene preparation, and GPU submission. That separation keeps ergonomic view code away from renderer details without rebuilding every layer for every pointer or wheel event.

```text
Winit events
    │
    ├─ coalesced wheel input ───────────────┐
    ├─ retained hover/press/scroll state ───┤
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
                              ┌─────────────┴─────────────┐
                              ▼                           ▼
                   instanced quad pipeline      cached Glyphon text
                              └─────────────┬─────────────┘
                                            ▼
                                       WGPU surface
```

## Frame scheduling

The event loop runs with `ControlFlow::Wait`. Invalidation queues at most one Winit redraw. Multiple wheel events accumulate into one logical delta before the redraw handler runs. An occluded or zero-sized window is skipped and is not placed in a retry loop.

Hover, pressed state, and retained scroll-container offsets only mark paint dirty. They reuse the existing element declaration and Taffy layout. An application mutation calls `EventContext::invalidate()`, which rebuilds the declarative tree at the next redraw. `request_animation_frame()` is explicit and keeps rebuilding only while a view requests it.

## Element identity and ownership

The application owns its `View`. Each render returns a declarative `Element` tree. Explicit `.id(...)` keys are reserved and validated before generated path IDs are assigned, so duplicate developer keys are errors while hash collisions for unkeyed nodes are deterministically probed around.

An element ID owns:

- hover and pressed paint state;
- click listener lookup;
- scroll-container offset;
- text-layout and glyph-cache identity.

`ViewContext::listener` stores callbacks in a registry parameterized by the concrete view type. `Element::on_click` only carries the stable ID, keeping the element tree non-generic and compact while callbacks can still mutate `&mut Self` without `Rc<RefCell<_>>` application state.

## Layout

Taffy implements Flexbox and intrinsic measurement. Text leaf measurement calls the renderer's shared Cosmic Text cache, so layout and paint do not shape the same string twice. Typography is inherited during tree construction; hover and active variants are deliberately paint-only so a pointer move cannot trigger relayout.

Scrollable nodes retain offsets outside the declaration tree. Fixed-height `VirtualList` is a separate O(1) range calculator for very large data sets; it never allocates in `visible_rows()` and mounts only viewport rows plus configured overscan.

## Rendering

The current renderer has two specialized pipelines:

1. Quads use six shader-generated vertices and an instance containing logical bounds, fill, inside border, radius, and clip. Rounded-corner antialiasing is analytic in WGSL. All visible quads are submitted in one draw call.
2. Text uses Cosmic Text for Unicode shaping/fallback and Glyphon for Swash rasterization plus atlas rendering. Stable `TextId`s prevent reshaping unless content, width, metrics, family, weight, wrap mode, or display scale changes.

Colors are stored in linear-light space. Eight-bit constructors decode sRGB on the CPU, and the sRGB surface performs the output transfer. Quad output is premultiplied before using premultiplied-alpha blending.

Quad instance uploads rotate across three GPU buffers. Capacity grows geometrically. Text layouts are age-evicted every frame and have a hard cap of 256 retained areas; Glyphon's atlas is trimmed after presentation. The deliberately small layout cache keeps long, disjoint scrolling from retaining whole off-screen text buffers while still covering several nearby viewports.

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

## Current boundaries

This milestone establishes the performance architecture and view ergonomics, but a complete platform toolkit also needs accessibility, editable text, clipboard, semantic focus, images, ordered compositing/layers, menus, and broader platform acceptance. Those features should extend the retained tree and narrow renderer rather than bypass its scheduling and cache invariants.
