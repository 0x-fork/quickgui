# Deterministic testing

[Documentation index](README.md)

Enable the test-only API for downstream tests without adding it to production builds:

```toml
[dev-dependencies]
quickgui = { version = "0.1", features = ["test-support"] }
```

`TestAppContext` consumes the ordinary `Application` configuration plus explicit root
`WindowOptions` and reuses its concrete views, retained
element identity/focus/form/input state, listener registry, actions, keymap, globals, entities,
window ownership, and foreground executor. It creates no Winit event loop, native window, WGPU
adapter, background worker, accessibility service, timer thread, or polling loop for ordinary
semantic tests:

```rust
let (mut cx, counter) = Application::new().into_test_context(
    WindowOptions::default(),
    Counter { count: 0 },
)?;
let window = counter.window_handle();

cx.click(window, "increment")?;
assert_eq!(cx.read(counter, |view| view.count)?, 1);

cx.simulate_keystrokes(window, "ctrl-s")?;
cx.advance_time(Duration::from_millis(500))?;
```

## Visual frames

Geometry and screenshot calls lazily create one windowless WGPU renderer. They run the production
Taffy layout, retained paint tree, Cosmic Text shaping, scene ordering, and WGPU pipelines against
an offscreen RGBA8 target; no native window, event loop, presentation loop, background thread, or
polling source is introduced:

```rust
let window = counter.window_handle();
let mut visual = cx.visual(window)?;

visual.assert_element_bounds(
    "card",
    Rect::new(24.0, 18.0, 320.0, 96.0),
    0.01,
)?;

let actual = visual.capture_screenshot()?;
actual.assert_matches_png(
    "tests/snapshots/counter.png",
    VisualTolerance::new(1, 0),
)?;
```

Screenshots use the deterministic window scale factor and are capped at 4,096 physical pixels per
axis and 64 MiB of tightly packed RGBA storage. Snapshot clones share that storage. Comparisons scan
in place without allocating a diff bitmap and report changed-pixel and channel-delta statistics.
`write_png` explicitly creates a baseline when desired; tests never update goldens implicitly.
The renderer and all of its existing bounded caches are reused by later visual calls, but are never
created by a context that only uses semantic operations. Raw native `NSView` pixels cannot exist in
the windowless WGPU target, so capture rejects such a frame instead of silently producing a partial
baseline; its QuickGUI host element can still be checked with `element_bounds`.

Visual geometry and screenshot requests settle variable-height list feedback before returning.
The context performs at most eight production layout/declaration passes and fails deterministically
if an application changes an item's height on every render. A stable measured list therefore
produces repeatable geometry without adding an implicit animation frame.

Semantic clicks focus and activate a stable element ID; controlled text input, Tab traversal,
typed action bubbling, contextual multi-stroke bindings, form submission, application lifecycle
callbacks, heterogeneous child windows, and targeted window commands settle synchronously.
`QuitMode`, child-first `on_window_closed` delivery, explicit application exit, and windowless
Dock-reopen callbacks use the same production ownership rules; the test context can prove a reopen
creates a fresh root without starting Winit.
Wheel propagation and default prevention can be exercised with `simulate_scroll_wheel`.
`simulate_retained_scroll` and `retained_scroll_offset` instead route through the production
geometry/default-scroll path, so tests can prove paint-only overflow scrolling does not increment
`render_count`; virtual containers still rebuild when their mounted slice follows the shared
offset. `simulate_scrollbar_press`, `simulate_scrollbar_drag_to`, and
`simulate_scrollbar_release` drive the built-in overlay scrollbar through the production press,
drag, and release path, painting between events as a native window does, and
`scrollbar_drag_active` reports whether the drag survived the frames it caused. Raw contacts use
`simulate_touch`, whose element argument selects a target only for
`Started` while
later phases exercise the production `TouchId` capture. Pressure and gesture listeners use
`simulate_mouse_pressure`, `simulate_pinch`, `simulate_rotation`, and
`simulate_smart_magnify`. All use the same typed listener registry without native input services.
`simulate_appearance_change` updates the remembered system appearance,
delivers `Event::AppearanceChanged` only to system-following windows, and lets tests prove that an
unobserved change does not rebuild a view. Explicit light/dark commands use the same retained
window-command path as production. Background-appearance commands update the same retained
`WindowState` snapshot, and repeated or component-equivalent transitions are regression-tested for
one coalesced rebuild; native pixel composition remains part of macOS visual acceptance.
`simulate_keyboard_layout_change` replaces the same immutable layout snapshot as the native macOS
notification, remaps opted-in key equivalents, invokes `on_keyboard_layout_change`, and rebuilds
only declarative observers. A test can select `com.apple.keylayout.German` and dispatch `cmd-ö` to
exercise a `platform-[` declaration without changing the host keyboard. `simulate_keystroke`
accepts a constructed `Keystroke`, including `with_key_char(...)`, so Option characters,
command-layout identity, and the separation between binding matches and committed text remain
independently testable.
`simulate_keystroke` also exercises production keymap-before-raw ordering, action capture/bubble,
focused key capture/bubble, propagation, and default prevention. `simulate_key_up` delivers the
matching release path without inventing a key-down or native input service. Tests can therefore
prove that a handled shortcut never leaks into a raw key listener, an explicitly propagated action
does fall through, stopped propagation remains independent from editing defaults, and listener-only
dispatch does not increment `render_count`.
The same production default-key path covers radio groups and tabs. Tabs tests assert the single
roving Tab stop, axis-specific arrows, Home/End, manual Enter/Space activation, automatic
activation, looping and non-looping edges, disabled-item skipping, panel mounting, exact
TabList/Tab/TabPanel projection, and zero additional renders after idle settlement.
The test context also owns a deterministic in-memory clipboard. Tests can seed it with
`write_to_clipboard`, inspect it with `read_from_clipboard`, call the same methods from an
`EventContext`, and prove that simulated editor copy/cut/paste shortcuts share that service. The
macOS Find pasteboard is represented by a separate in-memory slot.
Foreground futures run on the production local executor against an injected monotonic clock, so
`advance_time` never sleeps, and continuous animation frames advance only through explicit
`advance_frame` calls. Every operation stops after 1,024 effect turns rather than hanging on a
recursive application loop. Visual paint uses that same injected clock so repeated captures cannot
advance animations through wall-clock drift. Native platform requests deliberately fail without an
explicit adapter.

The same clock drives declarative duration animations and analytic springs. `advance_frame`
advances unthrottled motion only for windows that requested a frame; `advance_time` alone fires an
exact max-FPS deadline. Tests can therefore prove completion returns to sleep, synchronized
windows share an epoch, retargeting keeps spring velocity, pausing does not catch up, and Reduce
Motion schedules no frame.

`VisualTestContext::move_pointer` uses production layout and hit testing, then repaints the retained
tree without rebuilding the view. Combined with its injected-time helper and pixel reads, visual
tests can verify transition interpolation, interruption, completion, and the invariant that a
paint-only state change does not increment the view render count.

With both `inspector` and `test-support` enabled,
`TestAppContext::inspector_snapshot(window)` returns the bounded production retained-tree
projection after materializing layout on demand. It provides deterministic hierarchy, bounds,
clip, paint order, hit-region, focus-path, accessibility, and truncation assertions without a
native window or GPU adapter. See the [inspector guide](inspector.md).

## Live macOS acceptance

Deterministic tests cover the retained semantics and offscreen renderer, but they cannot exercise
AppKit resize transactions, `CAMetalLayer` drawable acquisition, WindowServer presentation, or
real `NSView` composition. Before a macOS 0.1 release, run all live gates from a logged-in desktop
with at least two connected displays:

```console
scripts/macos-performance-gate.sh
scripts/macos-acceptance-gate.sh
scripts/macos-display-acceptance-gate.sh
```

The first gate audits bidirectional 100,000-row scrolling and idle behavior. The second audits
native resizing with wrapped Unicode text, native-child detach/remount/geometry/teardown, a real
`NSTextField` editor-to-QuickGUI first-responder handoff, retained/native represented-file and
edited-state set/clear agreement, eight native context-menu/submenu activation cycles, and four
owner-press plus four native Escape dismissal cycles with a submenu open. Those 16 active-app
interaction cycles require owner focus restoration and complete child-window teardown; neither
dismissal route may deliver the menu command. A further 128 root-plus-submenu command cycles run at
a declared 50 ms inter-cycle cadence. Current RSS and physical footprint are sampled after cycles
32 and 128, positive growth is limited to 16 MiB and 24 MiB respectively, and every cycle must leave
zero popover windows. A final cooperative activation handoff to Finder requires
application-deactivation teardown with no owner focus reclamation. The gate also checks cache and
draw-call bounds, idle frames, whole-process CPU, peak RSS, and owned graphics footprint. These
timing and compositor checks complement rather than replace `cargo test`; neither belongs in
headless hosted CI. Popover input is delivered directly through live AppKit/Winit responders and each
surface is closed before it remains visibly composited, so this proves native lifecycle and event
routing but is not a human-visible popover QA recording. Because the second gate posts real AppKit
pointer events, changes the key window, and ends by yielding foreground activation to Finder, do
not use the mouse or keyboard while it runs.

The independent display gate creates one hidden, non-key WGPU window per active display. On the
first native render it requires the retained window to be centered against that render's immutable
snapshot. After one 150 ms deadline it compares retained and AppKit display ID, scale, window-frame
origin, full screen bounds, and visible work area, then requires the complete native outer frame to
remain inside that final work area. `WindowState` carries content size while `NSWindow.frame`
includes title-bar chrome, so only their global origins are directly comparable. The split avoids
mistaking a later Dock work-area change for failed placement or requiring a visible window jump.
The default gate requires a secondary display. Set
`QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_SECONDARY=0` only for a diagnostic single-display run; the
result then says `secondary_status: "skipped"`, which is not two-display proof. Set
`QUICKGUI_DISPLAY_ACCEPTANCE_REQUIRE_MIXED_SCALE=1` for the mixed-scale release run. Schema 3 reports
the initial and settled observations separately. The probe uses one deadline per hidden window and
one watchdog, never a monitor poll or idle frame loop.
