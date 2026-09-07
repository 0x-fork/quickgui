# Rendering optimization coverage

This implementation covers independent geometry invalidation, retained subtree drawing commands,
broader incremental mutations, Rust dependencies scoped by subtree and phase, and GPU reuse.
Instrumentation and correctness/work regression checks make the scope and limits measurable.

## Implemented foundations

- `FrameMetrics.pipeline` records mutation, declaration, reconciliation, layout, natural geometry,
  paint, accessibility, and renderer elapsed time, plus visited/measurement/cache counters.
- Natural bounds and scroll-snap geometry survive paint-only changes. The layout hover pass and
  paint share them. Stable hit regions, bounds, dismiss regions, and selection regions are retained.
  Editable inputs and state transforms conservatively refresh interaction geometry when needed.
- Substantial static subtrees retain immutable `Arc<PaintLayer>` command chunks. Dirty ancestor
  paths identify branches to visit. Inherited color, opacity, named groups, focus-within, selection,
  and scrollbar visibility participate in invalidation. Cache references have a conservative
  32 MiB bound, and callbacks, animation owners, native views, and compositing groups remain dynamic.
- GPU compositing retains raw group pixels and blurred output. Root transform/opacity changes can
  reuse pixels; nested effects, content, scale, physical texture generations, and effect fallback
  invalidate them. GPU texture ownership stays with the existing bounded pool; command snapshots
  have a separate 32 MiB limit.
- Shape, image, SVG, path, and shader buffers compare against the correct rotating physical slot
  and upload changed 256-byte blocks, with at most 32 writes per buffer. Large buffers fall back to
  full uploads rather than growing an unbounded shadow. Glyph vertex uploads compare exact bytes
  with bounded shadows. Telemetry reports tracked primitive uploads and retained shadow bytes.
- A test-only thread-local allocator measures gross allocation calls/bytes without instrumenting
  application builds. The toolbar-hover regression measured 2 painted nodes, 0 natural-geometry
  nodes, 9 allocations, and 3,040 allocated bytes with both 100 and 10,000 unrelated mounted nodes.
  This measures UI-tree work, not complete frame time or GPU time.
- Headless GPU checks compare cached output pixel-for-pixel with fresh rendering through rotation,
  opacity, blur, color, scale, and nested-transform changes.

## Incremental declarations and dependencies

- `ElementUpdate::Replace` validates a final batch before reconciling only the replaced layout
  subtrees. Keyed reparenting detaches changed edges before attaching them. Normal mount-state
  synchronization preserves focus, selection, scroll, input, animation, and accessibility state
  and releases unmounted resources. Mixed paint/replacement batches remain atomic on rejection.
- Rust `ViewContext::component` owns a restart scope. Reverse entity/global indexes and explicit
  parent/child scope indexes route notifications and dispose only the affected scopes. Dirty
  parents subsume child restarts. Root observations retain full-declaration semantics.
- `ViewContext::bind` connects dependencies directly to text/color/opacity/transform mutations.
  Paint bindings skip layout; text bindings update measurement and semantics; transforms update
  placement, hits, and accessibility geometry. Bindings cannot declare callbacks or child scopes.
- `with_scope`, `View::render_scope`, and `AppRunner::invalidate_elements` let the native host
  rebuild ordinary nodes after layout, listener, property removal, insertion, removal, and keyed
  movement. Both sides of a move are invalidated. Old callback ownership is cleared for all roots
  before any replacement is declared, and indexed listener keys are never reused.

## Validation

Core tests compare retained layout against fresh layout after mixed updates, reordering, reparenting,
resize, and unmount. Work checks isolate changed measurement and reconciliation nodes, and cover
geometry retention, inherited style, named groups, selection, and callback-owned fallbacks. Scope
tests cover nested/conditional dependencies, globals, parent coalescing, root observations, listener
replacement, subscription disposal, and phase-specific bindings. Host tests exercise property and
listener changes, preserved focused input, keyed moves, detached construction, and subtree cleanup.

Headless GPU tests compare reused compositing pixels with fresh rendering, exercise texture
repurposing and budget pressure, distinguish base/overlay uniforms in one submission, and read back
sparse uploads from separate physical buffers. The allocation regression is a UI-tree probe, not
a complete frame-time benchmark. Run `scripts/with-macos-ghostty-zig.sh cargo test --lib`, the same
wrapper with `cargo test -p quickgui-host --lib`, and `bash scripts/check-go.sh` with Go on PATH.

## Performance boundaries

- Structural validation and mount-index synchronization still scan the final tree. Editable inputs,
  changing text geometry, state transforms, and layout changes conservatively refresh interaction
  geometry. Static drawing caches do not remove all renderer preparation or draw submissions.
- Compound native parts, hoisted portals, declaration callbacks, asynchronous image fallbacks, root
  child-list changes, and unsupported scopes use the complete declaration path. This preserves the
  existing coordinated-control and resource-lifetime behavior.
- Environment and focus invalidation remain at view scope. Ordinary explicit root invalidation
  runs the root; applications opt into entity/global component scopes or direct value bindings.
- Phase durations are CPU-side elapsed timings. Primitive upload telemetry excludes texture/atlas
  uploads and compositor-specific uniforms. Cached group pixels reduce offscreen passes, while
  transforms and opacity still require compositing into the presented target.

Partial GPU damage rectangles are a later, profile-dependent extension. The renderer does not
assume swapchain contents survive presentation; retained command chunks and group textures are
separate from a persistent window backing target. Live interaction and visual QA remain separate
from source, work-counter, and headless GPU validation.
