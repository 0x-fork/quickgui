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
