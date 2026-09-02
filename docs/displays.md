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
- the native scale factor and optional refresh rate;
- the clockwise `rotation_degrees` (`0`, `90`, `180`, or `270`);
- `is_internal`, true for the machine's built-in panel;
- an optional `color_depth` in bits per pixel.

| Field | macOS | Windows | Linux |
| --- | --- | --- | --- |
| `rotation_degrees` | `CGDisplayRotation`, converted to a clockwise quarter turn | `0` | `0` |
| `is_internal` | `CGDisplayIsBuiltin` | `false` | `false` |
| `color_depth` | `NSBitsPerPixelFromDepth(NSScreen.depth)` | `None` | `None` |

Platforms without a native source report the neutral value shown above rather than guessing.

Do not persist `DisplayId` as a physical-monitor preference. Persist `DisplayUuid`, find its current
snapshot entry at launch, then use that entry's current ID. Other platforms honestly return no UUID
until their native stable-identity adapters are implemented.

## Granular display events

`Displays::diff` turns two consecutive snapshots into a bounded, deterministic event list:

```rust
for event in previous.diff(&next) {
    match event {
        DisplayEvent::Added(display) => open_panel_on(display.id()),
        DisplayEvent::Removed(id) => forget_panel_on(id),
        DisplayEvent::MetricsChanged(display) => reflow(&display),
    }
}
```

Both snapshots are sorted by `DisplayId`, so the diff is one linear merge: events are emitted in
ascending identifier order and never exceed `MAX_DISPLAY_EVENTS` (`2 × MAX_DISPLAYS`, 128). A
display present in both snapshots produces `MetricsChanged` only when an observable field differs,
including the primary flag.

The runtime computes this diff at the same screen-parameters boundary that refreshes the coarse
snapshot, and delivers it through one application callback:

```rust
Application::new()
    .on_display_event(|event, cx| match event {
        DisplayEvent::Added(display) => cx.open_window(options_for(&display), Panel::default()),
        DisplayEvent::Removed(id) => close_panels_on(id, cx),
        DisplayEvent::MetricsChanged(display) => reflow(&display, cx),
    })
    .run(|cx| { /* … */ })?;
```

The coarse observation keeps working unchanged: a view that read `cx.displays()` still rebuilds when
the snapshot changes, whether or not a display-event callback is installed. The diff is computed
only when a callback exists, the pending queue is bounded, and events are drained on the next
runtime turn. No timer, observer, or polling loop runs while the display topology is stable.

## Placement

Target automatic placement with the same fluent window API used for root and child windows:

```rust
Application::new().run(|cx| {
    cx.open_window(
        WindowOptions::default().display(display_id),
        Workspace::default(),
    );
})?;

cx.open_window(
    WindowOptions::new("Inspector")
        .size(640.0, 480.0)
        .display(display_id),
    Inspector::default(),
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
activation, and once immediately before each non-popover native window-creation batch. Every ordinary
window performs one final refresh after its hidden first Metal presentation and before it is shown.
That boundary handles a left/right Dock whose reserved width changes when a
command-line app receives its Dock presence, and reconciles only automatic `.display(id)` centering;
explicit global bounds remain untouched. System-popover churn performs no monitor query. Unchanged
snapshots do not invalidate a view. There is no monitor query in `about_to_wait`, no timer, no idle
frame, and no display polling thread.

`Displays::diff` is a pure function, so display-reconfiguration handling is covered by unit tests
over synthetic snapshots rather than by driving real hardware.

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
