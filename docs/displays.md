# Displays and window placement

[Documentation index](README.md)

QuickGUI exposes active monitors as one immutable, bounded snapshot. Coordinates are global logical
desktop pixels with a top-left origin, matching `WindowBounds`; `visible_bounds` excludes persistent
native chrome such as the macOS menu bar and Dock.

```rust
let displays = cx.displays();
let primary = cx.primary_display();
let selected = cx.find_display(saved_id);
let current = cx.current_display();
```

The `ViewContext` methods are declarative observations. QuickGUI rebuilds only a window whose latest
render read display state. Event callbacks have unobserved `displays`, `primary_display`, and
`find_display` methods for placement decisions without forcing a later rebuild.

Each `Display` contains:

- a process-level `DisplayId` used to target a window;
- a stable `DisplayUuid` on macOS for persisted monitor preferences;
- a bounded localized name;
- full and visible global logical bounds;
- the native scale factor and optional refresh rate.

Do not persist `DisplayId` as a physical-monitor preference. Persist `DisplayUuid`, find its current
snapshot entry at launch, then use that entry's current ID. Other platforms honestly return no UUID
until their native stable-identity adapters are implemented.

## Placement

Target automatic placement with the same fluent window API used for root and child windows:

```rust
App::new(Workspace::default())
    .display(display_id)
    .run()?;

cx.open_window(
    Inspector::default(),
    WindowOptions::new("Inspector")
        .size(640.0, 480.0)
        .display(display_id),
);
```

When no explicit `window_bounds` are supplied, selecting a display centers the requested logical
size in its visible work area and shrinks only if the work area is smaller. Borderless fullscreen is
created on that monitor. An unknown or disconnected ID falls back to the current primary display;
it never creates an off-screen window. Explicit global bounds remain authoritative.

`Display::centered_bounds` and `Display::constrain_bounds` provide the same finite, work-area-safe
geometry for restoration code without mutating a native window.

## Scheduling and memory

The runtime retains at most 64 descriptions and 4 KiB per display name. Public snapshots contain
Rust values and `Arc` storage, not `NSScreen`, Core Foundation UUID, or video-mode objects. macOS
refreshes the snapshot from `NSApplicationDidChangeScreenParametersNotification`, application
activation, and once immediately before each non-popup native window-creation batch. Every ordinary
window performs one final refresh after its hidden first Metal presentation and before it is shown.
That boundary handles a left/right Dock whose reserved width changes when a
command-line app receives its Dock presence, and reconciles only automatic `.display(id)` centering;
explicit global bounds remain untouched. Anchored-popup churn performs no monitor query. Unchanged
snapshots do not invalidate a view. There is no monitor query in `about_to_wait`, no timer, no idle
frame, and no display polling thread.

`TestAppContext` starts with one deterministic display and accepts a validated replacement through
`simulate_displays_change`. This covers targeting, disconnect fallback, current-display state, and
conditional observation without Winit or WGPU.

Run the native multi-monitor probe with:

```console
cargo run --release --example displays
scripts/macos-display-acceptance-gate.sh
```

The ordinary example is interactive. The gate enables its self-driving mode and creates hidden,
non-key windows on every active display. It proves exact centering against each window's first-render
snapshot, then separately proves the settled retained/AppKit display identity, scale, native-frame
origin, full and visible screen bounds, and containment of the complete native outer frame in the
current work area. This distinction permits a Dock work-area change after placement without moving
an already visible ordinary window. A secondary display is required by default. Mixed-scale
hardware is reported as `passed`, `failed`, or `skipped`; require it explicitly with
`QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_MIXED_SCALE=1` on the release topology.
