# Deterministic test architecture

[Architecture index](README.md) · [Documentation](../README.md)

## Deterministic test context

The optional `test-support` feature exposes `TestAppContext`, which consumes the same `App`
builder without constructing Winit, WGPU, AccessKit, background workers, or platform objects. Each
headless window owns the production type-erased view adapter, retained `UiTree`, listener registry,
window snapshot, parent relation, and dirty bit. Application globals, keymaps, menus, deferred
entity/global queues, application callbacks, and foreground task registry remain shared exactly as
they are in the native runtime.

A declaration calls the production `View::render` adapter, but `UiTree::set_root_for_test` stops
after stable-ID validation and retained identity/focus/form/text reconciliation, before Taffy
layout or scene/GPU preparation. This makes element-addressed click, focus, input, action, keymap,
and form tests deterministic without allocating a GPU. Semantic click first applies the same focus
transition as native pointer and accessibility activation, then invokes the keyed listener and form
route.

The first geometry or screenshot operation lazily constructs one `OffscreenRenderer`. `UiTree`
then uses the same statically dispatched text-layout interface, Taffy pass, paint traversal, scene,
and bounded WGPU pipeline caches as a native window. The only alternate boundary is the final
target: an RGBA8 render attachment is copied through one row-aligned, 64 MiB-bounded staging buffer
instead of being presented to a Winit surface. The mapped rows are compacted into one immutable
snapshot, and both the transient texture and staging buffer are dropped after capture. Repeated
geometry-only assertions do not submit GPU work; repeated screenshots reuse the renderer caches.

The foreground spawner receives a controlled monotonic clock only in test builds. Production
compiles to the direct system-clock variant; tests wake exact timer deadlines through
`advance_time` without sleeping. Requested animation frames are never advanced implicitly.
Window creation, child-first close, targeted commands, application callbacks, deferred observers,
and ready futures drain through one explicit effect pump capped at 1,024 turns, with an independently
bounded dispatch queue. This converts accidental recursive application effects into a test error
instead of a hung process. Visual layout and painting receive the injected clock rather than
`Instant::now`, so screenshots remain stable until the test explicitly advances time or a frame.
The native renderer stays monomorphized through the narrow layout trait, adding no virtual dispatch,
thread, timer, polling source, or frame-time branch to production.
