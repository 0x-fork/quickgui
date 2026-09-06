# Windows and shared state

[Documentation index](README.md)

## Multiple windows

An event handler can mount a different concrete `View` type in another native window and keep its
stable handle. Closing one window does not affect unrelated windows:

```rust
let open = cx.listener("open-inspector", |this, cx| {
    this.inspector = Some(cx.open_window(
        WindowOptions::new("Inspector")
            .size(520.0, 360.0)
            .background(Color::rgb8(20, 22, 27)),
        Inspector::new(),
    ));
    cx.invalidate();
});

let focus = cx.listener("focus-inspector", |this, cx| {
    if let Some(inspector) = this.inspector {
        cx.focus_window(inspector);
        cx.invalidate_window(inspector);
        // cx.close_window_handle(inspector);
    }
});
```

State shared by unrelated view types uses a main-thread entity instead of a lock or manual window
handle fan-out:

```rust
use quickgui::{Entity, EventEmitter, Subscription};

struct WorkspacePublished {
    revision: usize,
}
impl EventEmitter<WorkspacePublished> for WorkspaceState {}

let workspace = Entity::new(WorkspaceState::default());

// During either window's render, this both reads and retains the observation.
let revision = cx.observe(&self.workspace, |state| state.revision);

// During an event callback, every observing window receives one coalesced invalidation.
self.workspace.update(cx, |state, _cx| {
    state.revision += 1;
});

// Store this `Subscription` on the view; dropping it cancels delivery.
if self.workspace_events.is_none() {
    self.workspace_events = Some(cx.subscribe(
        &self.workspace,
        |this, _source, event: &WorkspacePublished, cx| {
            this.last_published_revision = event.revision;
            cx.invalidate();
        },
    ));
}

// Typed events are delivered after this callback releases its view/entity borrows.
self.workspace.emit(cx, WorkspacePublished { revision: 1 });
```

`Entity<T>` is deliberately main-thread-owned and lock-free; background work returns through the
existing UI-thread completion callback before updating it. Conditional observations disappear on
the next declarative rebuild, and `WeakEntity<T>` breaks ownership cycles. A window retains at most
4,096 entity identities. One callback retains at most 1,024 distinct notifications, then falls back
to one bounded all-window invalidation rather than losing a change or growing memory without bound.
Typed events are not coalesced: they retain FIFO emission order, then visit windows in creation order
and subscriptions in registration order. `Subscription` follows GPUI's RAII lifetime contract;
`detach()` keeps it until the subscribing window closes. Each window retains at most 4,096 active
subscriptions, one callback emits at most 1,024 events, one effect cycle queues at most 4,096 events,
and recursive delivery stops at 65,536 callback invocations. There is no polling, subscription task,
or idle frame.

Application-wide settings and services use one exact Rust type instead of an entity handle shared
through every view constructor:

```rust
use quickgui::{Application, Global, Subscription, WindowOptions};

#[derive(Default)]
struct Appearance {
    warm: bool,
}
impl Global for Appearance {}

Application::new()
    .global(Appearance::default())
    .run(|cx| {
        cx.open_window(WindowOptions::default(), Launcher::default());
    })?;

// During render, this read conditionally watches the type for this window.
let warm = cx.watch_global::<Appearance, _>(|appearance| appearance.warm);

// During an event callback, mutation is synchronous and notification is deferred until
// the callback releases its borrows.
cx.update_global::<Appearance, _>(|appearance| appearance.warm = !appearance.warm);

// Store the RAII subscription on the view; dropping it cancels callback delivery.
let subscription: Subscription = cx.observe_global::<Appearance>(|this, cx| {
    this.last_theme_change = cx.global::<Appearance>().warm;
    cx.invalidate();
});
```

`observe_global` follows GPUI's callback shape; `subscribe_global` is an explicit alias, while
`watch_global` is QuickGUI's conditional declarative read. `has_global`, `global`, and `try_global`
provide unobserved reads. `global_mut`,
`default_global`, `set_global`, `update_global`, and `remove_global` are available in event
callbacks and coalesce changes by concrete `TypeId`. The application retains at most 1,024 global
types; a window may observe 1,024 types and retain 1,024 RAII global subscriptions. One callback
tracks 256 exact changed types before conservatively notifying all global observers, one deferred
turn queues at most 1,024 types, and recursive callback delivery stops at 65,536 invocations.
Globals are main-thread-owned, so ordinary access is lock-free and adds no thread, timer, polling
pass, GPU resource, or idle frame. See `cargo run --release --example multi_window` for exact
cross-window observation and cancellation behavior.

Each window independently owns its retained view, element/UI tree, listener registry, frame
scheduler, focus, pointer capture, IME/key state, scene, surfaces, metrics, and bounded render
caches. Compatible windows with the same `PerformanceProfile` share the heavyweight WGPU device
and queue. `Event::CloseRequested` is delivered before destruction; a view can call
`cx.prevent_close()`, while `cx.close_window()` explicitly closes the current window. Async image
completions are tagged with the owning `WindowHandle`, so identical per-window request IDs cannot
wake or mutate another cache. On macOS, arbitrary typed drag values can cross these independent
windows without serialization; the example includes a bidirectional process-local card transfer.
See `cargo run --release --example multi_window`.

The Rust core also owns application-wide lookup. `EventContext::window_registry()` captures mounted
and already queued windows at the start of the callback; `windows()` and `active_window()` are
convenience reads over the same immutable snapshot. `AppRunner::window_registry()`,
`active_window()`, and `window_state(handle)` expose the corresponding externally pumped view, so a
binding does not need a second focus or window-identity registry. Handles sort by stable creation
identity and round-trip through `WindowHandle::as_u64`/`from_u64`. A snapshot retains at most 4,096
handles and reports `is_truncated()` if that pathological bound is reached. The runtime reuses the
shared handle slice while membership is unchanged, so ordinary pointer/key events do not allocate
another registry.

## Application and window lifecycle

`QuitMode` makes final-window behavior explicit. `Default` follows GPUI and native desktop
convention: macOS remains in the event loop for Dock reopen or application-menu commands, while
Windows and Linux quit after their last window closes. Examples and applications that always want
one behavior can select it directly:

```rust
use quickgui::{Application, QuitMode, WindowOptions};

Application::new()
    .quit_mode(QuitMode::Explicit)
    .on_window_closed(|window, cx| {
        cx.update_global::<Session, _>(|session| session.note_closed(window));
    })
    .on_reopen(|had_visible_windows, cx| {
        if !had_visible_windows {
            cx.open_window(workspace_window_options(), Workspace::default());
        }
    })
    .run(|cx| {
        cx.open_window(WindowOptions::default(), Workspace::default());
    })?;
```

`LastWindowClosed` always exits with the final window; `Explicit` waits for `cx.exit()` or native
application termination. Parent teardown remains child-first. `on_window_closed` runs after each
window, native composition host, dialog, and window-owned foreground task are gone, and its
application-wide context has no implicit parent. It can update shared state or open a replacement
top-level window. One callback slot is retained, and registering again replaces it rather than
growing an observer list.

Components that own child handles do not need the application-wide hook. During rendering,
`ViewContext::on_child_window_closed(handle, ...)` observes one known direct child, while
`on_any_child_window_closed(...)` can be declared before a handle exists and compares the delivered
handle with component state. The latter covers a child opened and closed in the same event turn.
Both run after child teardown while the direct parent is still alive, are refreshed declaratively,
share a 256-callback per-window bound, and add no native observer, timer, task, or idle work.

`cx.exit()` uses the same structured teardown instead of dropping the event loop out from under
windows. Native Quit is normalized through that ownership path as the loop exits. With no windows
and `Explicit` mode, QuickGUI stays in `ControlFlow::Wait`; there is no redraw, timer, or lifecycle
polling. The `platform_services` example closes its final window, remains resident on macOS, and
creates a fresh window when its Dock icon is clicked.

`Event::CloseRequested` with `cx.prevent_close()` keeps one window open, and
`on_before_quit`/`on_will_quit` with `cx.prevent_quit()` hold the two preventable quit phases.
`AppRunner::request_quit()` gives an embedding runtime the same preventable path that native
Command-Q takes, while `AppRunner::exit()` keeps bypassing both phases for a forced shutdown. The
JavaScript host builds `window.onCloseRequested`, `app.on("beforeQuit")`, and
`app.on("willQuit")` on exactly these hooks; see [QuickGUI UI and JavaScript
bindings](ui.md#lifecycle-vetoes-in-javascript).

`cx.relaunch()` adds one prepared replacement process to that same teardown. QuickGUI releases the
single-instance guard and process integrations after the last close callback, then spawns exactly
once. See [Relaunch and signed updates](relaunch-and-updates.md) for overrides and updater flow.

Window creation also retains a native role, restore geometry, initial visibility, and runtime
capabilities:

```rust
use quickgui::{
    SystemPopover, Rect, Size, WindowBounds, WindowKind, WindowLevel, WindowOptions,
};

let dialog = cx.open_window(
    WindowOptions::new("Confirm delete")
        .window_kind(WindowKind::Dialog)
        .window_bounds(WindowBounds::Windowed(Rect::new(220.0, 140.0, 560.0, 360.0)))
        .minimum_size(480.0, 300.0)
        .maximum_size(960.0, 720.0)
        .resizable(false)
        .maximizable(false)
        .closable(false)
        .content_protected(true)
        .window_level(WindowLevel::AlwaysOnTop),
    ConfirmDelete::new(),
);

let menu = SystemPopover::new(244.0, 178.0)
    .gap(6.0)
    .open(cx, "actions-trigger", "Actions", ActionsMenu)?;

cx.set_window_bounds(WindowBounds::windowed(240.0, 160.0, 720.0, 520.0))
    .expect("valid bounds");
cx.set_window_minimum_size(Size::new(560.0, 360.0))?;
cx.clear_window_minimum_size_handle(dialog)?;
cx.set_window_maximum_size_handle(dialog, Size::new(1200.0, 900.0))?;
cx.set_window_closable_handle(dialog, true)?;
cx.set_window_shadow_handle(dialog, false)?;
cx.toggle_fullscreen().expect("native window");
cx.hide_window().expect("native window");
cx.show_window_handle(dialog).expect("queued child handle");
```

`WindowOptions::display` targets automatic centering and borderless fullscreen at
one active monitor. `WindowState::display_id` reports the monitor currently containing a window.
See [Displays and window placement](displays.md) for work-area geometry, stable macOS identities,
disconnect fallback, and the event-driven observation contract.

Editor windows can also retain a represented file, native edited-state indication, and an explicit
system-tabbing identifier. Runtime commands cover the character palette, tab navigation, merging,
detaching, tab-bar visibility, and tab overview; `WindowState::native_tabs` is a fixed-size cached
snapshot rather than a retained list of platform objects. See [Native document windows](document-windows.md)
for the full API, validation bounds, deterministic simulation, and AppKit ownership contract.

Event callbacks enqueue those mutations through `EventContext`. A render pass observes future
native state changes through `ViewContext`:

```rust
let state = cx.window_state();
```

`WindowState::minimum_size` and `maximum_size` report the effective inner-size constraints. Normal
windows default to a `320 x 240` minimum and no maximum; system popovers are unconstrained. Runtime
set/clear commands are validated through the same finite 32,768-point dimension bound as resize
commands, and startup rejects a minimum larger than its maximum. Changing a constraint requests at
most one necessary native resize; subsequent OS resize events follow the ordinary damage path
rather than a framework correction loop. Explicit programmatic bounds remain application-authored
restore geometry and are still reconciled by the native window manager. A view that does not call
`window_state()` is not rebuilt when only a constraint changes.

Movable, resizable, minimizable, maximizable, and closable are independent policies. The native
maximize button is enabled only when both resizable and maximizable are true. Decoration, shadow,
content protection, and `WindowLevel::{AlwaysOnBottom, Normal, AlwaysOnTop}` are also retained in
`WindowState`, have startup builders, and have current-window plus handle-targeted commands.
`automatic_window_level` derives the established role behavior: floating/popover roles stay above
ordinary windows, while normal/dialog roles use normal stacking. Content protection is enforced by
Winit on macOS and Windows but remains subject to OS capture limitations; runtime shadow mutation is
currently native on macOS, while some other window managers always draw decorated-window shadows.
Unsupported window-manager hints still retain the requested core state rather than making a
binding-specific source of truth.

Pointer and shell policy are part of the same retained state. `set_cursor_visible`,
`set_cursor_grab`, `set_cursor_position`, and `set_cursor_hit_test` have handle-targeted variants;
`cursor_screen_position()` reads the hardware pointer in global logical desktop coordinates.
Windows additionally exposes taskbar progress plus an accessibility-described overlay icon, while
macOS exposes application-wide Dock badges/icons/menus. Query
`DesktopIntegrationSupport::current()` before selecting platform-specific presentation and see
[Desktop integrations](desktop-integrations.md) for the complete matrix.

The live macOS acceptance gate starts below a larger runtime minimum, verifies the one-time native
growth, compares both `WindowState::minimum_size` and AppKit's `contentMinSize`, clears the
constraint, and then reaches a viewport below the former minimum. Programmatic resize remains
application-authored geometry; AppKit's retained content minimum is the contract used for user
edge/corner resizing.

The effective native palette is also available as a narrower declarative observation:

```rust
use quickgui::WindowAppearance;

let appearance = cx.appearance();
let dark = appearance == WindowAppearance::Dark;
```

Windows follow the operating system by default. `WindowOptions::window_appearance` forces the
initial light or dark appearance, while `follow_system_appearance` removes that preference.
Callbacks can change the same policy without recreating the window:

```rust
cx.set_window_appearance(WindowAppearance::Dark)?;
cx.follow_system_window_appearance()?;
cx.set_window_appearance_handle(dialog, WindowAppearance::Light)?;
```

The command updates retained state after the callback releases its borrows. A native effective
change while following the system delivers `Event::AppearanceChanged`; explicit commands do not
fabricate an operating-system event. Calling `cx.appearance()` observes future effective changes
and rebuilds only that window. A view that does not observe appearance and ignores the event stays
clean.

Window background composition is a separate policy from light/dark appearance:

```rust
use quickgui::WindowBackgroundAppearance;

let options = WindowOptions::new("Palette")
    .window_background(WindowBackgroundAppearance::Blurred);

cx.set_window_background_appearance(WindowBackgroundAppearance::Transparent)?;
cx.set_window_background_appearance_handle(
    dialog,
    WindowBackgroundAppearance::Opaque,
)?;
```

`Opaque` is the default fast path. `Transparent` exposes the desktop through scene pixels whose
alpha is below one, while `Blurred` adds the platform's native background blur behind the same
alpha-capable GPU surface. `cx.window_state().background_appearance` declaratively observes the
effective retained policy. A command mutates only the native transparency or blur component that
actually changed, coalesces into one redraw, and adds no animation, timer, or idle frame. See
`cargo run --release --example window_background`.

Every `open_window` call makes the delivering window the new window's retained parent. Closing a
parent closes its descendants child-first and cancels their foreground tasks. On macOS, `Dialog`
is an AppKit sheet, `Floating` uses the floating level, and both `Popover` and `SystemPopover` are
true nonactivating `NSPanel` subclasses at the transient popover-menu level. `PopoverOptions` uses the
parent content's logical top-left coordinate space and supports nine anchor points, nine gravity
directions, offsets, and independently selectable flip/slide/resize work-area constraints.

A system popover with `grab: true` becomes key without activating the application and closes on
Escape or a mouse press outside it. The first grab lazily installs one shared local/global AppKit
event-monitor pair; nested grabs are capped at 32, use only the top entry, and remove the pair when
the stack empties. Passive popovers install no monitor. AppKit child-window ownership keeps a visible
panel attached to parent movement, and all placement and dismissal is event-driven—there is no
popover timer, polling pass, or idle frame. Non-macOS backends use a parent-owned borderless Winit
fallback until their compositor-native popover paths land. See
`cargo run --release --example system_popover`.

Every system popover event context identifies both its direct parent and the nearest non-popover
owner. `dispatch_action_to_parent` is appropriate for an ordinary child view;
`dispatch_action_to_popover_owner` crosses an arbitrarily nested popover chain and invokes the owner's
ordinary focused typed-action path. `close_popover_chain` targets the first popover below that owner,
so normal child-first teardown closes all submenu descendants without a parallel component stack.
Cross-window action retention is capped per event and per effect cycle; closed targets are ignored
safely.

Until stable Winit and AccessKit share the release containing native panel allocation, QuickGUI
uses the versioned `quickgui-winit` and `quickgui-accesskit-winit` support crates in `vendor/`.
They keep one Winit type universe for downstream packages and are released in that order before
`quickgui`; no consumer-side Cargo patch is required.

Title, bounds, move, resize, minimum/maximum constraints, minimize, restore, zoom, fullscreen,
visibility, movability, resizability, minimizability, maximizability, closability, decorations,
shadow, content protection, stacking level, appearance, background composition, focus, attention,
and close commands can target the current window or a stable `WindowHandle`. Mutations are
validated before retention, capped at 256 per event and 1,024 per effect cycle, and applied after
the application callback releases its borrows. Calling `cx.window_state()` or `cx.appearance()`
declaratively observes native changes; it does not install a timer or polling frame. See
`cargo run --release --example window_controls`, `cargo run --release --example appearance`, and
`cargo run --release --example window_background`.
\n
## Window lifecycle events and constrain hooks

Native window transitions are ordinary `Event` variants delivered to `View::event`:

```rust
use quickgui::{Event, EventContext, Point, Size, View};

fn event(&mut self, event: &Event, cx: &mut EventContext) {
    match event {
        // Delivered once, after this window's first frame reached the screen.
        Event::FirstPresented => { let _ = cx.show_window(); }
        Event::Minimized(minimized) => self.dock_state = *minimized,
        Event::Maximized(maximized) => self.zoomed = *maximized,
        Event::FullscreenChanged(fullscreen) => self.immersive = *fullscreen,
        Event::OcclusionChanged(occluded) => self.paused = *occluded,
        Event::WindowLevelChanged(level) => self.level = *level,
        // Constrain hooks run before the next frame is laid out.
        Event::WillResize { proposed_size } => {
            let _ = cx.constrain_resize(Size::new(proposed_size.width, 480.0));
        }
        Event::WillMove { proposed_position } => {
            let _ = cx.constrain_move(Point::new(proposed_position.x, 0.0));
        }
        _ => {}
    }
}
```

`Event::FirstPresented` is the flicker-free ready-to-show moment. A window created with
`WindowOptions::show(false)` prepares its first frame offscreen and can be revealed from this event
without a timer. It is delivered exactly once per window; later frames never repeat it.

`Minimized`, `Maximized`, and `FullscreenChanged` are equality-suppressed: an unchanged transition
delivers nothing. Winit does not surface AppKit's `windowDidMiniaturize:`,
`windowDidEnterFullScreen:`, or `windowDidDeminiaturize:` callbacks, so QuickGUI derives them by
reading `NSWindow.isMiniaturized`, the AppKit fullscreen mask, and the zoom geometry at the native
events that accompany those transitions — resize, move, and occlusion. macOS reports a Dock
minimize as occlusion, which is where `Event::Minimized` originates there. The read is entirely
event-driven; an idle window performs no sampling, and no observer or timer is installed.

`Event::WillResize` and `Event::WillMove` are QuickGUI's portable stand-in for
`windowWillResize:toSize:` and `windowWillMove:`. They are delivered from the corresponding native
event, and `constrain_resize`/`constrain_move` request at most **one** corrective native resize or
move per proposal. The corrective value is remembered, so the event it produces cannot start
another round: a constrain hook can never loop. Both hooks validate through the same finite
32,768-point dimension and 16,777,216-point coordinate bounds as every other window command and
return `WindowCommandError::InvalidBounds` for a non-finite or out-of-range value.

Application-level activation reuses the observers QuickGUI already installs:

```rust
Application::new()
    .on_did_become_active(|cx| cx.update_global::<Session, _>(|s| s.focused = true))
    .on_did_resign_active(|cx| cx.update_global::<Session, _>(|s| s.focused = false))
```

Both are macOS-only today (`NSApplicationDidBecomeActiveNotification` and
`NSApplicationDidResignActiveNotification`). They add no additional native observer: the resign
notification is the same one that already dismisses grabbing system popovers. One callback slot is
retained per hook; registering again replaces it.

## Stacking and input policy

`WindowLevel` now names AppKit's stacking constants directly. `WindowLevel::macos_level()` is the
exact `NSWindowLevel` QuickGUI applies:

| `WindowLevel` | `NSWindowLevel` | Windows / Linux |
| --- | --- | --- |
| `AlwaysOnBottom` | `-1` | always-on-bottom hint |
| `Normal` | `0` | normal |
| `AlwaysOnTop` | `3` (`NSFloatingWindowLevel`) | topmost |
| `Floating` | `3` (`NSFloatingWindowLevel`) | topmost |
| `ModalPanel` | `8` (`NSModalPanelWindowLevel`) | topmost |
| `MainMenu` | `24` (`NSMainMenuWindowLevel`) | topmost |
| `Status` | `25` (`NSStatusWindowLevel`) | topmost |
| `PopUpMenu` | `101` (`NSPopUpMenuWindowLevel`) | topmost |
| `ScreenSaver` | `1000` (`NSScreenSaverWindowLevel`) | topmost |

`AlwaysOnTop` deliberately shares `NSFloatingWindowLevel` with `Floating` so existing applications
keep their established behavior. Non-macOS backends collapse every above-normal level to Winit's
single topmost hint and retain the exact requested level in `WindowState::window_level`, so the
core stays the source of truth. A command that changes the *effective* level delivers
`Event::WindowLevelChanged` to that window.

```rust
cx.set_window_level(WindowLevel::Status)?;
cx.move_window_top()?;                       // raise without activating the application
cx.move_window_above(other_handle)?;         // order directly above a sibling
cx.set_ignore_mouse_events(true, true)?;     // clicks pass through, hover still arrives
cx.set_window_enabled(false)?;               // visible but inert
cx.set_aspect_ratio(Some(Size::new(16.0, 9.0)))?;
cx.set_window_button_visibility(false)?;     // hide the macOS traffic lights
cx.set_window_shadow(false)?;
```

Every command has a `_handle` variant and shares the 256-per-event / 1,024-per-effect-cycle window
command bounds.

- **`move_window_top` / `move_window_above`** — macOS uses `orderFront:` and
  `orderWindow:relativeTo:`, which change stacking without activating the application or making the
  window key. `move_window_above` rejects a window ordering above itself with
  `WindowCommandError::InvalidWindowOrder`, and silently ignores a sibling handle that is no longer
  mounted. Windows and Linux have no portable restack request in Winit, so they fall back to a
  native focus request; this is an honest approximation, not the same operation.
- **`set_ignore_mouse_events(ignore, forward)`** — macOS sets `ignoresMouseEvents` and, when
  `forward` is true, keeps `acceptsMouseMovedEvents` on so AppKit's existing event-driven
  `mouseMoved:` stream still reaches the window while every press falls through. `forward` is
  meaningless without pass-through and is normalized to `false` when `ignore` is `false`. Other
  backends map the request onto `set_cursor_hittest`, which cannot forward motion; they report
  `forward_mouse_events` truthfully in `WindowState`. `set_cursor_hit_test` remains the simple
  all-or-nothing form.
- **`set_window_enabled(bool)`** — AppKit has no `EnableWindow`, so macOS expresses "visible but
  inert" with `ignoresMouseEvents` plus refusing key-window status and resigning key. Windows uses
  Win32 `EnableWindow` (a small addition that is **unverified** on a live Windows desktop). Linux
  retains the requested state and logs that the backend does not implement it.
- **`set_aspect_ratio(Option<Size>)`** — macOS applies `contentAspectRatio`. Every platform also
  applies a portable clamp inside the resize path: after the view's own `constrain_resize`
  narrowing, the width is kept authoritative and the height is recomputed from the ratio. The ratio
  must be finite, both components positive, and neither component more than
  `MAX_WINDOW_ASPECT_RATIO` (1,000) times the other; otherwise the command fails with
  `WindowCommandError::InvalidAspectRatio` before anything is retained.
- **`set_window_button_visibility(bool)`** — macOS hides the close/minimize/zoom
  `standardWindowButton`s while keeping the titlebar. Other platforms retain the requested state
  and log that only macOS separates the buttons from the titlebar; use `decorated(false)` there.

`WindowState` reports `window_level`, `ignore_mouse_events`, `forward_mouse_events`,
`window_enabled`, `aspect_ratio`, and `window_buttons_visible`, and `WindowOptions` has matching
`ignore_mouse_events`, `window_enabled`, `aspect_ratio`, and `window_button_visibility` builders so
a window can start in any of these policies.

## Reaching these from an embedding host

`AppRunner` mirrors the window-scoped commands so an externally pumped host — the Bun binding, a
test harness, or any embedder driving `pump` itself — reaches them without an `EventContext`:

```rust
runner.move_window_to_top(handle)?;
runner.move_window_above(handle, other)?;
runner.set_window_ignore_mouse_events(handle, true, true)?;
runner.set_window_enabled(handle, false)?;
runner.set_window_aspect_ratio(handle, Some(Size::new(16.0, 9.0)))?;
runner.set_window_button_visibility(handle, false)?;
runner.set_window_always_on_top(handle, true, Some(WindowLevel::ScreenSaver))?;
```

Each one queues an ordinary `WindowCommand` under the existing `MAX_PENDING_WINDOW_COMMANDS` bound
and is applied on the next event-loop turn, so an embedder never mutates a native window from
outside the application thread. A handle is a valid target from the moment `open_window` returns
it: a command queued before the platform window exists waits for that window's creation on the
same turn instead of failing, and is dropped only if the window is closed first. `runner.window_state(handle)` and `runner.displays()` supply the
`WindowState::restore_state` pair, and `WindowOptions::restore` accepts the result unchanged.

Per-window menus and native popup menus need the runtime's window-scoped `EventContext`, which only
exists inside an effect cycle, so `AppRunner` declares them and the runtime resolves them during its
next `process_window_commands` turn:

```rust
runner.set_window_menus(handle, [Menu::new("Document").action("Export…", Export)])?;
runner.use_application_menus_for_window(handle)?;

// Resolves once the popup closes, whether an item ran or the user dismissed it.
let closed = runner.show_window_popup_menu(handle, menu, Some(Point::new(24.0, 48.0)))?;
```

Both deferred queues carry the `MAX_PENDING_NATIVE_POPUP_MENUS` bound, a popup declared for a window
that closes first completes with `PlatformError::Unavailable`, and a popup menu declared for an
unmounted window is rejected before anything is retained.

## Persisting and restoring window geometry

`WindowRestoreState` is a `serde` `Serialize`/`Deserialize` value an application can store next to
its own settings:

```rust
use quickgui::{WindowOptions, WindowRestoreState};

// During rendering, this observes both window state and displays:
let restore: WindowRestoreState = cx.window_restore_state();
let json = serde_json::to_string(&restore)?;

// From an event callback, capture it explicitly:
let restore = cx.window_state().restore_state(cx.display_snapshot());

// On the next launch:
let restore: WindowRestoreState = serde_json::from_str(&json)?;
cx.open_window(
    WindowOptions::new("Workspace").restore(&restore, cx.display_snapshot()),
    Workspace::default(),
);
```

It stores the windowed restore rectangle (`x`, `y`, `width`, `height`), the `maximized` and
`fullscreen` modes, the process-level `display_id`, the stable `display_uuid` when the platform
exposes one (macOS), and the capturing display's `scale_factor`. A maximized or fullscreen window
still persists the rectangle it returns to.

`WindowOptions::restore` (and the underlying `WindowRestoreState::resolve`) validates the value
against the live display snapshot and never places a window off every connected display:

1. The remembered display is matched by stable UUID first, then by process identifier.
2. If that display is connected and its **work area** intersects the rectangle, the rectangle is
   used exactly.
3. If the remembered display is connected but the rectangle no longer intersects its work area, the
   rectangle is clamped into that work area with `Display::constrain_bounds`.
4. If the remembered display is gone but some other connected display's work area intersects the
   rectangle, that display is adopted and the rectangle is used exactly.
5. Otherwise the window is centered on the primary display at the persisted size.
6. A rectangle that fails the ordinary window-bounds validation (non-finite, non-positive, or
   outside the 16,777,216-point coordinate / 32,768-point dimension range) falls back to a centered
   960 x 640 window.

`ResolvedWindowRestoreState::adjusted` reports whether steps 3, 5, or 6 changed the persisted
geometry, so an application can tell the user its window moved. `WindowRestoreState::is_valid`
exposes the same validation without resolving.
