# QuickGUI architecture

QuickGUI separates application declaration, retained interaction state, scene preparation, and GPU submission. That separation keeps ergonomic view code away from renderer details without rebuilding every layer for every pointer or wheel event.

```text
Winit + AccessKit events
    │
    ├─ coalesced wheel input ───────────────┐
    ├─ native pressure/gestures ────────────┤
    ├─ retained hover/focus/input state ────┤
    ├─ layout key + printable key char ─────┤
    ├─ contextual keymap → typed action ────┤
    ├─ native menu id → typed action ───────┤
    ├─ local app mutation → invalidate ─────┤
    ├─ shared entity → observed windows ────┤
    └─ typed entity event → subscriptions ──┤
                                            ▼
                                  retained Element tree
                                  + keyed listeners/state
                                            │ app changes only
                                            ▼
                                  Taffy Flexbox/Grid layout
                                            │ layout changes only
                                            ▼
                                      reusable Scene
                                ordered plane/z layers
                              ┌─────────────┴─────────────┐
                              ▼                           ▼
                  instanced shape pipeline      cached Glyphon text
                              ├──── retained path pipeline ────┤
                              ├──── sampled image pipeline ────┤
                              ├──── cached SVG mask pipeline ──┤
                              └─────────────┬───────────────────┘
                                            ▼
                         ┌──────── base WGPU surface
                         ├──────── macOS native NSViews (when mounted)
                         └──────── transparent overlay WGPU surface
```

## Modules

- [Runtime and ownership](runtime.md) — windows, platform effects, shared entities, and globals.
- [Scheduling](scheduling.md) — damage-driven frames, foreground work, and performance invariants.
- [Native extensions](extensions.md) — optional backends, Go import discovery, ABI ownership, and release artifacts.
- [Performance guide](performance.md) — required invariants and regression checks for runtime and rendering changes.
- [Input and interaction](input.md) — keyboard layouts, retained identity, actions, menus, drag/drop, gestures, text, and accessibility.
- [Layout and rendering](rendering.md) — Taffy layout, WGPU pipelines, caches, images, animation, and first-frame presentation.
- [macOS composition](macos.md) — native window chrome, anchored panels, AppKit children, and overlay surfaces.
- [Deterministic testing](testing.md) — the headless production-view adapter and bounded effect executor.
- [Current boundaries](boundaries.md) — remaining platform and test gaps.

Return to the [documentation index](../README.md).
