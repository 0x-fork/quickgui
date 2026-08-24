# QuickGUI architecture

QuickGUI separates application declaration, retained interaction state, scene preparation, and GPU submission. That separation keeps ergonomic view code away from renderer details without rebuilding every layer for every pointer or wheel event.

```text
Winit + AccessKit events
    │
    ├─ coalesced wheel input ───────────────┐
    ├─ retained hover/focus/input state ────┤
    ├─ contextual keymap → typed action ────┤
    ├─ native menu id → typed action ───────┤
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

`ViewContext::listener` and `ViewContext::action_listener` store callbacks in a registry
parameterized by the concrete view type. `Element::on_click` and `Element::on_action` only carry the
stable ID, keeping the element tree non-generic and compact while callbacks can still mutate
`&mut Self` without `Rc<RefCell<_>>` application state.

## Focused action and key dispatch

Typed actions separate command meaning from input source. Keyboard bindings, ordinary buttons,
command palettes, and native menu adapters all dispatch the same type-erased action value back into
a typed handler. The retained tree records a parent pointer and optional `KeyContext` for each
element. Dispatch follows only the root-to-focus path: key predicates resolve on that context stack,
then matching handlers bubble from the focused node toward the root. A bubble handler consumes by
default and must call `EventContext::propagate()` to continue, preventing accidental duplicate
workspace/editor commands.

Contextual precedence is deepest match first, then latest binding at the same depth. Global
bindings behave as though they match the deepest context, allowing user bindings loaded last to
override defaults. The keymap indexes bindings by their first normalized stroke instead of scanning
the complete map on each keypress.

Multi-stroke prefixes are retained per window with the focus identity that started them. A focus
change clears the prefix. A mismatch or timeout replays the longest exact action prefix, or the raw
key input when no action matched, before processing the new stroke. Pending storage cannot grow
beyond a registered binding sequence. The only scheduling cost is `ControlFlow::WaitUntil` for one
prefix deadline; a clean window otherwise remains on `ControlFlow::Wait`.

## Native application menus

The cross-platform `Menu`/`MenuItem` model owns the same concrete action values as the keymap.
Action equality includes payloads, so two commands of the same Rust type do not accidentally share
a displayed shortcut. On macOS, QuickGUI appends only its declared root items to Winit's existing
menu bar and removes those exact Objective-C objects on drop; Winit continues to own About,
Services, Hide, and Quit.

Each native action item stores a small numeric ID. Its Objective-C target sends only that `usize`
through Winit's thread-safe event proxy, while the runtime retains the non-atomic, main-thread-only
typed action value. Focused handler availability and contextual single-stroke key equivalents are
recomputed only after a retained-tree change, focus boundary, or menu-open event. AppKit owns menu
tracking and submenu keyboard navigation, so a clean application still sleeps.

Cut, Copy, Paste, Select All, Undo, and Redo can carry an `OsAction`. The target first asks AppKit's
responder chain to perform the selector, preserving native behavior in embedded `NSView` controls.
If no responder accepts it, the numeric event returns to focused QuickGUI action dispatch and then
the retained text-input implementation. Menu-open events clear pending multi-stroke input, matching
keyboard dispatch semantics rather than replaying a prefix while a native menu is active.

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

On macOS, a single opaque AppKit launch shield uses the configured background color and stays above
all three planes until the base renderer completes `Presented`. Surface retries and occlusion keep
the shield mounted. Before ordering the window onscreen, QuickGUI runs view declaration, Flexbox,
text shaping, scene construction, native reconciliation, and GPU buffer preparation. A hidden Metal
surface is occluded before a render pass can execute, and WGPU intentionally refuses to acquire its
drawable. For the first surface submission only, QuickGUI retains and detaches the Winit content
view from the still-hidden `NSWindow`. Its CAMetalLayer then has no hosting window, so WGPU can
acquire, render, present, and await the actual surface drawable without exposing any AppKit window.
RAII reattaches the same content view to the same window at its unchanged frame after completion;
only then is the window ordered onscreen. This avoids partial native content, black launch covers,
and cross-monitor movement without polling. Failure to produce that hidden first surface frame is a
startup error rather than permission to reveal a partial window. Later occlusion sleeps normally.

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

Single-line input keeps UTF-8 byte ranges internally but moves and deletes at Unicode grapheme boundaries. Cosmic Text provides visual caret hit testing and bidirectional selection spans. Composing text is retained separately from the controlled value, Winit preedit cursor offsets remain byte-indexed, and the native IME candidate rectangle follows the painted caret. AccessKit receives a stable `TextRun` child, grapheme character lengths, editable value, and native text-selection state. Clipboard objects are created lazily so applications that never copy or paste pay no startup cost. Undo and redo retain at most 100 snapshots and 512 KiB per direction for each mounted input; an external controlled-value replacement clears history.

When native AppKit children are mounted, QuickGUI installs a no-ivar Objective-C subclass above AccessKit's existing Winit-view subclass. Its child and navigation queries preserve AccessKit's virtual nodes and append the live native accessibility roots in composition order. Accessibility focus and screen-point hit testing route into a native subtree when its responder owns focus or its frame contains the point; otherwise they fall through to AccessKit. Identity/order changes post one AppKit layout notification, while ordinary redraws allocate nothing and post nothing. Teardown restores AccessKit's exact class before AccessKit later restores Winit's class.

## Current boundaries

This milestone establishes the performance architecture, ordered overlays, macOS native child
composition, hybrid AccessKit/AppKit accessibility, semantic focus, and a production-oriented
single-line editing path, typed actions, focus scopes, contextual keymaps, and native macOS
application menus. A complete platform toolkit still needs rich and multiline editing, images,
shadows, command palettes, drag/drop, multi-window ownership, non-macOS menu projection, and broader
platform acceptance. Those
features should extend the retained tree and narrow renderer rather than bypass its scheduling and
cache invariants.
