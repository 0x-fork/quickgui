# Runtime and ownership

[Architecture index](README.md) · [Documentation](../README.md)

## Application and window ownership

The windowless `Application` builder is the core lifecycle boundary for embedders. Its
`AppRunner` becomes ready on the first native event-loop turn without requiring a placeholder
window, and `open_window` reserves a stable handle before native creation. `App::new(view)` remains
the Rust convenience API that supplies the first view up front. Closing an empty application does
not accidentally trigger `LastWindowClosed`; that policy begins after a native window has opened.

The application runtime owns a registry keyed by Winit's native `WindowId` plus a second map from
public `WindowHandle` values. A handle is allocated before native creation, so an event callback
can open a window and immediately retain an identity for later focus, invalidation, or close
commands. Winit serializes callbacks; the runtime temporarily activates only the target entry while
delivering one event, then returns it to the registry. That keeps the established single-window hot
path direct without sharing pointer, keyboard, focus, scheduling, or cache state between windows.
Rust views and listeners read this identity from `ViewContext::window_handle` and
`EventContext::window_handle`; language bindings project those core-routed handles into their own
callback context rather than choosing a window from focus or creation order.

Every entry owns a type-erased `View` boundary, but its listener callbacks downcast back to the
original concrete view type before application code runs. This allows one application to host
unrelated Rust view types without making the declarative `Element` tree generic. The entry also
owns its `UiTree`, listener registry, frame scheduler, pending key sequence, modifiers, pointer
capture, IME state, scene, metrics, renderer surfaces, accessibility adapter, native children, and
bounded caches. Application assets, custom-font metadata, keymaps, menus, clipboard ownership,
compatible WGPU contexts, and the lazy decode worker pool are shared. Asset and font declarations
are resolved once before the first native window; runtime callbacks clone only the stable asset
handle and all font-database mutation remains serialized on the application thread.

Window commands are collected from `EventContext` and applied only after the delivering callback
releases its active entry. New windows complete hidden first-frame preparation before becoming
visible. Targeted focus and invalidation then operate through the stable-handle map; close drops the
complete entry and its platform resources. `Event::CloseRequested` can be intercepted with
`prevent_close()`. The event loop exits only after the last window and pending window request are
gone. Native menu validation and menu actions always use the most recently focused live window.

Each entry retains its effective semantic `WindowAppearance` separately from the optional native
preference. `None` follows the platform; an explicit light/dark command updates native chrome and
the retained snapshot together. Winit's effective-theme event updates system-following entries,
delivers `Event::AppearanceChanged`, and invalidates only declarative window-state observers.
Switching preference is part of the existing bounded window-command queue and creates no separate
observer task.

Every event-created window retains the delivering `WindowHandle` as its parent. Close requests walk
that relation without recursion, derive a stable child-first order, cancel each window's foreground
tasks, dismiss any native sheet relationship, and only then release native and GPU state. Duplicate
or overlapping close requests coalesce by handle. One event may retain at most 256 window mutations
and one effect cycle at most 1,024; titles and logical geometry are validated before they enter either
queue.

On macOS, dialog children are presented with `beginSheet`, floating windows use
`NSFloatingWindowLevel`, and transient popups use `NSPopUpMenuWindowLevel` plus auxiliary-space and
hide-on-deactivate behavior. `PopUp` and `AnchoredPopup` allocate Winit's real `NSPanel` subclass
with the nonactivating style rather than changing an existing `NSWindow` object's class. QuickGUI
backports Winit's upstream panel allocation change in the versioned `quickgui-winit` support crate.
The matching `quickgui-accesskit-winit` adapter links to that same crate, so accessibility and the
renderer keep one Winit type universe even for downstream Cargo users; this is an explicit
dependency rather than a root-only `[patch]`. The two Apache-2.0 support crates are released before
`quickgui` and can disappear when AccessKit and stable Winit converge on 0.31. Runtime window
snapshots are event-driven. Decorated-window zoom reads AppKit's `isZoomed`
directly, while borderless programmatic zoom is retained in QuickGUI state; calling Winit's
borderless `is_maximized` would temporarily mutate the style mask and recursively emit move/resize
callbacks.

The text dependency chain is similarly explicit: `quickgui-cosmic-text` owns ordered per-style
fallback attributes and shaping-cache identity, and `quickgui-glyphon` depends on that exact
version while adding paint-only text-area opacity. Publishing the main crate therefore requires
Cosmic Text before Glyphon; downstream code still imports their conventional `cosmic_text` and
`glyphon` Rust library names through QuickGUI's private dependency graph.

## Native platform services

`EventContext` validates platform-service inputs before retaining a request. One callback can queue
32 operations and one effect cycle can retain 128. Native prompts and file panels carry a
single-receiver `PlatformResponse`; its waker is installed only while awaited, so an open AppKit
sheet adds no frame deadline or polling source. The runtime starts requests only after the active
view callback releases its borrows.

On macOS, prompts use `NSAlert`, path selection uses `NSOpenPanel`, saves use `NSSavePanel`, and all
three attach to the deepest visible sheet in their QuickGUI parent chain. A window relation may own
one active platform dialog and an application at most 32. Unrelated windows can present
independently. AppKit completion first retires the active-dialog identity through Winit's event
proxy, then resolves the response and wakes its foreground task. Dropping the response sends the
inverse cancellation event; closing a window cancels its foreground tasks and native sheet before
releasing the window. Late completion and cancellation events carry a monotonically increasing ID,
so they cannot remove a newer dialog.

Open-panel results preserve macOS filesystem bytes and are rejected above 4,096 paths, 16 KiB per
path, or 16 MiB total. URL, open-path, and Finder-reveal requests use `NSWorkspace` synchronously on
the AppKit thread after callback release. Unbound Cmd-W calls `performClose`, letting Winit emit the
same interceptable `CloseRequested` event as the traffic-light button.

Application lifecycle hooks are not routed through an arbitrary focused window. QuickGUI extends
Winit's retained application-delegate object with a no-ivar dynamic subclass and associated state,
preserving Winit's concrete identity and inherited launch, wake, and termination methods. The
QuickGUI selectors carry capability flags for URL-open and Dock-reopen callbacks; a separate
thread-safe `NSWorkspace` wake observer is installed only when requested. Native callbacks copy at
most 256 URLs, 16 KiB each and 1 MiB total, then send one application event through Winit's proxy.
The resulting `EventContext` has no current window but shares the ordinary bounded global, entity,
window-creation, and platform-effect queues. No lifecycle hook owns a timer or redraw source.

`UNUserNotificationCenter` is created only for a bundled application and installed before launch
when response handling is configured. The first notification starts one authorization request;
until its background completion returns through the event-loop proxy, posts coalesce by tag in a
64-entry queue and dismissal removes matching queued work. Tags, text, actions, native response
strings, and distinct category sets are all bounded before retention. The category cache stops at
64 immutable action sets and posts later notifications without buttons. Authorization, delivery,
and response callbacks never mutate QuickGUI application state off the event-loop thread.

## Shared entity ownership

`Entity<T>` is a stable `Rc<RefCell<T>>` identity for application state intentionally shared across
otherwise independent view types. It remains main-thread-only: Winit serializes every render and
event callback, while background jobs send their result back before application state is touched.
This avoids a lock and atomic traffic on the normal render path. `WeakEntity<T>` carries the same
identity without extending its lifetime.

`ViewContext::observe` records an entity ID in the current window's listener registry and reads the
value in one operation. The registry is cleared and rebuilt with the declarative view, so a
conditional branch that stops reading an entity automatically unsubscribes on that rebuild.
`Entity::update` mutates synchronously and adds the identity to `EventContext`; duplicate updates in
one callback coalesce. Once the callback releases its active window, the runtime scans the small
window registry once and invalidates only entries whose retained observation set intersects the
changed IDs. It does not maintain a second reverse-index graph that would need lifecycle cleanup.

Each window may retain at most 4,096 observed identities. An event callback retains at most 1,024
distinct notifications; crossing that limit clears the list and requests one all-window
invalidation, preserving correctness with bounded memory. Scheduler invalidation still coalesces to
one redraw per affected window. Entity observation owns no thread, timer, deadline, GPU resource, or
idle frame.

Typed events use the same stable entity identity plus the event's Rust `TypeId`. An emitter declares
`EventEmitter<E>`, then moves `E` into one uniquely owned erased queue entry. The entry keeps the
source entity alive only through deferred delivery, while each subscriber callback captures a weak
source handle. Delivery begins after the emitting event callback releases its view and entity
borrows. Events remain FIFO and uncoalesced; matching windows run in stable creation order, then
subscriptions run in registration order. Events emitted by subscribers append to the same queue, so
recursive chains are breadth-first rather than reentrant.

`ViewContext::subscribe` returns an RAII `Subscription`. Dropping it marks the callback inactive
immediately; the next declarative rebuild prunes its retained entry. `detach()` keeps it for the
remaining lifetime of the subscribing window. A window retains at most 4,096 subscriptions, one
callback emits at most 1,024 events, the application queue retains at most 4,096 events, and one
effect turn invokes at most 65,536 subscription callbacks before failing loudly. These are count
bounds around application-owned payloads, not hidden payload copies. Subscription delivery adds no
thread, timer, deadline, GPU resource, polling pass, or idle frame. Foreground work uses the
separately bounded [foreground executor](scheduling.md#foreground-tasks).

## Application globals

The application owns one main-thread `TypeId -> Box<dyn Any>` store behind a cloneable
`Rc<RefCell<_>>` handle. `Global` is an empty `'static` marker: a concrete type can appear once,
does not need `Send` or `Sync`, and may be installed before launch with `App::global`. Event
callbacks can synchronously read, default, replace, mutate, or remove values through
`EventContext`; overlapping borrows fail loudly instead of adding locks or runtime aliasing.

`ViewContext::watch_global` combines an exact typed read with a conditional window observation.
As with entities, that set is cleared and rebuilt with the declarative view, so a branch that stops
reading a global automatically stops receiving invalidations. GPUI-shaped `observe_global` instead
returns the same RAII `Subscription` used by entity events; `subscribe_global` is an explicit alias.
Its callback is retained independently of the declarative observation set, can read the new global
through `EventContext`, and is cancelled as soon as its subscription drops.

Mutable access records the global's `TypeId` in the current `EventContext`. Duplicate changes in
one callback coalesce. Delivery begins only after the active view and global guards have been
released: exact declarative observers are invalidated once, then active callbacks visit windows in
creation order and subscriptions in registration order. Global types use a coalesced FIFO queue;
entity events use their existing uncoalesced FIFO queue. The deferred effect pump drains mutations
created by either callback class without re-entering application code while it is borrowed.

One application retains at most 1,024 global types. One window observes at most 1,024 types and
retains at most 1,024 global subscriptions. One event callback records at most 256 distinct
changes; crossing that bound switches to one conservative all-global notification. A deferred
effect turn queues at most 1,024 exact types and invokes at most 65,536 global callbacks. These are
count bounds around application-owned values, not payload-size promises. Global observation owns
no thread, timer, deadline, GPU resource, polling pass, or idle frame.
