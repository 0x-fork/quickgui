# QuickGUI architecture

QuickGUI separates application declaration, retained interaction state, scene preparation, and GPU submission. That separation keeps ergonomic view code away from renderer details without rebuilding every layer for every pointer or wheel event.

```text
Winit + AccessKit events
    │
    ├─ coalesced wheel input ───────────────┐
    ├─ retained hover/focus/input state ────┤
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

## Application and window ownership

The application runtime owns a registry keyed by Winit's native `WindowId` plus a second map from
public `WindowHandle` values. A handle is allocated before native creation, so an event callback
can open a window and immediately retain an identity for later focus, invalidation, or close
commands. Winit serializes callbacks; the runtime temporarily activates only the target entry while
delivering one event, then returns it to the registry. That keeps the established single-window hot
path direct without sharing pointer, keyboard, focus, scheduling, or cache state between windows.

Every entry owns a type-erased `View` boundary, but its listener callbacks downcast back to the
original concrete view type before application code runs. This allows one application to host
unrelated Rust view types without making the declarative `Element` tree generic. The entry also
owns its `UiTree`, listener registry, frame scheduler, pending key sequence, modifiers, pointer
capture, IME state, scene, metrics, renderer surfaces, accessibility adapter, native children, and
bounded caches. Application keymaps, menus, clipboard ownership, compatible WGPU contexts, and the
lazy decode worker pool are shared.

Window commands are collected from `EventContext` and applied only after the delivering callback
releases its active entry. New windows complete hidden first-frame preparation before becoming
visible. Targeted focus and invalidation then operate through the stable-handle map; close drops the
complete entry and its platform resources. `Event::CloseRequested` can be intercepted with
`prevent_close()`. The event loop exits only after the last window and pending window request are
gone. Native menu validation and menu actions always use the most recently focused live window.

Every event-created window retains the delivering `WindowHandle` as its parent. Close requests walk
that relation without recursion, derive a stable child-first order, cancel each window's foreground
tasks, dismiss any native sheet relationship, and only then release native and GPU state. Duplicate
or overlapping close requests coalesce by handle. One event may retain at most 256 window mutations
and one effect cycle at most 1,024; titles and logical geometry are validated before they enter either
queue.

On macOS, dialog children are presented with `beginSheet`, floating windows use
`NSFloatingWindowLevel`, and transient popups use `NSPopUpMenuWindowLevel` plus auxiliary-space and
hide-on-deactivate behavior. The popup is still a Winit-owned `NSWindow`, not an anchored
nonactivating `NSPanel`. Runtime window snapshots are event-driven. Decorated-window zoom reads
AppKit's `isZoomed` directly, while borderless programmatic zoom is retained in QuickGUI state;
calling Winit's borderless `is_maximized` would temporarily mutate the style mask and recursively
emit move/resize callbacks.

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
separately bounded executor described below.

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

## Frame scheduling

The event loop runs with `ControlFlow::Wait`. Per-window invalidation queues at most one Winit
redraw. Multiple wheel events accumulate into one logical delta for their owning window before its
redraw handler runs. An occluded or zero-sized window is skipped and is not placed in a retry loop.

Hover, pressed state, and retained scroll-container offsets only mark paint dirty. They reuse the
existing element declaration and Taffy layout. An application mutation calls
`EventContext::invalidate()`, which rebuilds the declarative tree at the next redraw.
`request_animation_frame()` is explicit and keeps rebuilding only while a view requests it;
`request_repaint_at()` instead stores one exact view deadline. Asynchronous images use owner-tagged
Winit user events for completion and one `WaitUntil` deadline for the 200 ms loading threshold.
Animated images add only the earliest visible frame deadline. Overlay scrollbar visibility adds
one hide deadline after the final scroll/leave/release transition. A pending tooltip adds one show
deadline and is cancelled without repaint if its pointer/focus target changes first. The runtime selects the earliest
key, loading, view, scrollbar, tooltip, or animation deadline across all windows; none of these states
introduces polling or a permanent frame loop.

## Element identity and ownership

The application owns its `View`. Each render returns a declarative `Element` tree. Explicit `.id(...)` keys are reserved and validated before generated path IDs are assigned, so duplicate developer keys are errors while hash collisions for unkeyed nodes are deterministically probed around.

An element ID owns:

- hover and pressed paint state;
- captured pointer-listener identity;
- secondary-click and tooltip-trigger identity;
- drag-source/drop-target identity and one active typed payload;
- focus identity and keyboard traversal position;
- click or controlled-input listener lookup;
- scroll-container and text-area offsets;
- text-input caret, selection, composition, and preferred visual column;
- text-layout and glyph-cache identity.

`ViewContext::listener`, `ViewContext::pointer_listener`, `ViewContext::context_menu_listener`, `ViewContext::drag_listener`,
`ViewContext::drop_listener`, and `ViewContext::action_listener` erase
callbacks into the owning window's registry and restore the concrete view type at invocation. `Element::on_click`,
`Element::on_pointer`, `Element::on_context_menu`, `Element::on_drag`, `Element::on_drop`, and `Element::on_action` only carry the stable ID, keeping the element tree
non-generic and compact while callbacks can still mutate `&mut Self` without `Rc<RefCell<_>>`
application state. Pointer capture retains only one target, origin, and latest position per window;
it ends on button release or window-focus cancellation and does not schedule frames while idle.

## Typed drag and drop

An internal primary-button press records only a source ID and origin until motion crosses two
logical points. The source callback then creates one `Arc<dyn Any>` payload and an optional GPU
preview element. Exact `TypeId` matching chooses the topmost compatible target while normal
pointer blockers preserve stacking semantics. Source `dragging` and target `drag_over` styles are
paint-only. An optional exact-type `can_drop` predicate is retained with the target and gates both
highlighting and release, so visual acceptance cannot disagree with delivery. Release downcasts
once into the registered callback; Escape or focus loss discards the session without application
work.

The preview has its own small Taffy tree, text/input state, animation state, and deterministic ID
namespace. It paints at the highest overlay layer but contributes no hit regions or native child
views. Pointer moves translate this detached tree and repaint the scene without rebuilding the
application view or relaying out the main tree. A stationary internal drag has no deadline unless
its preview itself contains an active animation.

Winit native file events feed the same typed path through `DroppedFiles`. Per-path enter events are
coalesced once at the event-loop boundary, and submit events are grouped before the target callback.
At most 4,096 paths are retained; overflow is reported on the value. macOS enter and submit
boundaries read AppKit's current content-view pointer position because Winit does not include it in
the file event, keeping the eventual drop target exact without polling.

On macOS every internal typed drag is eligible for native promotion. The application runtime owns
one main-thread registry capped at 256 simultaneous sessions. When a pointer first leaves the
logical viewport, the source moves a clone of its existing `Arc<dyn Any>` plus exact `TypeId`,
source window handle, and source element into that registry. An `NSUUID` token is the only private
value written to AppKit, so arbitrary Rust payloads cross QuickGUI windows without serialization,
copying their contents, or requiring `Send`.

The pasteboard writer composes that private token with an optional bounded
`ExternalDragPayload::Files`, `Text`, or `Url`, preserving public interoperability on the same
native dragging items. QuickGUI then starts one
`beginDraggingSessionWithItems:event:source:` session. Its Objective-C source, session, and RAII
registry entry stay owned by the originating runtime window until AppKit posts one completion
through the event-loop proxy. The terminal Winit release is excluded from retained hit testing so
it cannot click through at the drop position. Focus loss clears an unpromoted gesture but does not
prematurely release a platform-owned session.

Winit may stop content-view cursor delivery before its tracking area reports `CursorLeft` during a
captured gesture. The first press on any drag source therefore lazily installs and arms an AppKit
`LeftMouseDragged` local monitor. It forwards the original event unchanged, compares only the
source view's bounds, and atomically queues at most one boundary message. Release, cancellation,
promotion, and focus loss disarm it; windows that never use drag sources install nothing.

File payload construction examines at most 4,097 iterator entries and retains at most 4,096 paths,
16 KiB per encoded path, and 8 MiB across paths. The application supplies the directory bit, so
promotion performs no filesystem metadata calls; AppKit receives file `NSURL` writers and shared
system-symbol icons. Plain text retains at most 1 MiB in one shared UTF-8 allocation and truncates at
a scalar boundary. URL construction retains at most 16 KiB and rejects missing or invalid absolute
schemes plus whitespace and controls; AppKit receives an `NSURL` writer. The drag source advertises
copy only because QuickGUI has no source-side move commit phase. No timer, polling path, or frame
loop is installed.

Inbound macOS offers extend Winit's existing `NSWindow` delegate through a dynamic subclass; file
pasteboards forward every selector to Winit unchanged. A window advertises the private typed,
string, and URL pasteboard formats only while the rendered view has compatible exact-type
listeners. A valid private token resolves to the original shared Rust payload. Typed, URL, and text
representations form one ordered offer, with process-local data first.

After each damaged frame, the host reuses one topmost-first snapshot containing at most 8,192
exact listener acceptances and 4,096 relevant native targets or pointer blockers. Selection walks
visual regions before offered types, so a private payload cannot reach through a higher compatible
public-format target. AppKit can return `Copy` or `None` synchronously using the same `can_drop`
predicate as retained delivery; predicate panics are caught before reaching the Objective-C
boundary. Drag updates replace one pending slot behind one event-loop wake, terminal drops cannot
be overwritten by a late exit, and stationary drags schedule no work. Pasteboard conversion copies
at most the bounded text allowance plus one UTF-8 scalar; text reports scalar-safe truncation at 1
MiB, while invalid or greater-than-16-KiB URLs are rejected. The runtime rechecks the live tree
before painting `drag_over` or invoking the typed callback, so a snapshot can never authorize
delivery to a target removed by a preceding application update. Delivery distinguishes same-window,
cross-window, and external origins without changing callback typing.

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

A focus request may target a control introduced by the invalidation from that same event, such as
opening a command palette and focusing its search field. If the handle is not in the current tree,
the runtime retains it for exactly the pending rebuild, applies it after the new focus index exists,
then discards it whether or not the target appeared. This avoids both a two-event focus flash and a
persistent stale request; the resulting focus notification uses the ordinary path and introduces
no timer.

## Picker and command registry

`PickerState<T>` keeps reusable command-palette mechanics independent from application behavior.
Source items retain a label, optional detail/shortcut, hidden aliases, disabled state, and an
application-owned value. Empty queries preserve source order. Non-empty queries scan bounded text
without allocating normalized copies per item, rank case-folded subsequences with prefix,
word-boundary, and consecutive bonuses, then partition before sorting only the top 2,048 matches.
Matched label ranges are coalesced on UTF-8 boundaries and reuse `StyledText` for visible emphasis.

One picker admits at most 65,536 items, 64 KiB of text per item, and 16 MiB of searchable text in
total. Queries stop at 128 grapheme clusters and 4 KiB. These limits bound scan work, score scratch storage,
highlight metadata, and the retained result table even when the registry comes from dynamic data.
The generic value remains application-owned in the same sense as all other view state.

The standard surface derives stable child IDs from one caller-owned identity, registers its query,
row-click, and typed navigation listeners in the normal per-view registry, and mounts only the
`VirtualList` visible range plus one overscan row. Selection skips disabled entries, wraps for
Up/Down, reveals with the smallest possible scroll delta, and exposes native selected/disabled
semantics. `AnyAction` values preserve their original concrete payload. Activation can close the
overlay, restore the underlying focus path, then call `dispatch_any_action`, so key bindings,
buttons, the palette, and native menus reach the same handlers.

No picker work occurs during paint, pointer hover, or idle time. Query/source replacement performs
one synchronous bounded match pass; wheel input follows the existing virtual-scroll path, while
keyboard movement changes only selection and the retained offset. There is no debounce timer,
polling task, or display-rate animation.

## Tooltips and context surfaces

An element tooltip retains an arbitrary GPU element tree outside normal layout. Hover resolution
uses the topmost hit region and then walks retained parents, so nested visual children inherit the
nearest tooltip without leaking through an unrelated overlay. Keyboard focus follows the same
target path, while a pointer press, scroll, or drag cancels pending and visible tooltip state.

Entering a tooltip stores one bounded target ID and exact show time. It does not immediately
repaint. At the 500 ms default deadline, the runtime performs one paint-only wake-up, builds a
small detached Taffy tree, and places it through the normal anchor flip/alternate-alignment/clamp
algorithm. The detached tree contributes no hit regions or native children. Moving inside one
trigger schedules nothing; leaving a pending trigger cancels without a frame, and leaving a visible
trigger performs one hide repaint. Tooltip content is capped at 256 elements, declarations indexed
per window are capped at 4,096, and text tooltip labels populate the trigger's AccessKit
description.

Secondary-button press resolves the same topmost retained ancestry and invokes one view-local
`ContextMenuEvent` callback with logical pointer position and modifiers. `anchor_at` models that
point as a zero-sized anchor, reusing the established overlay placement and dismissal machinery.
Context surfaces remain application-defined element trees, so typed actions, focus restoration,
accessibility roles, and styling do not require a parallel menu/widget system. Neither an open
context surface nor a stationary tooltip introduces an idle deadline.

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

Taffy implements Flexbox, CSS Grid, and intrinsic measurement. Grid containers accept the
GPUI-compatible equal-track helpers plus explicit fixed, percentage, fractional, intrinsic,
fit-content, and `minmax(px, fr)` tracks. Equal columns or rows remain one compact `repeat()` style
component; explicit templates, spans, and line positions are clamped to 1,024 tracks per axis
before Taffy sees them. This bounds layout work from dynamic application data well below Taffy's
own 10,000-track safety limit without adding a second layout representation.

Text leaf measurement calls the renderer's shared Cosmic Text cache, so layout and paint do not
shape the same string twice. Typography is inherited during tree construction; hover and active
variants are deliberately paint-only so a pointer move cannot trigger relayout. Grid participates
in the same retained layout boundary: resize or a rebuilt view recomputes it, while scrolling,
selection, hover, and other paint-only changes reuse its boxes and schedule no idle frames.

Scrollable nodes retain offsets outside the declaration tree. Their vertical overlay scrollbar has
a 12-point invisible hit track, a 4-point revealed thumb, and an 8-point hover/capture thumb. The
topmost eligible track captures both thumb and track presses, keeps capture outside its bounds, and
cannot click through to content. Scroll motion reveals it; leaving or releasing schedules one hide
deadline. Pointer motion that does not cross a hover boundary schedules no redraw.

Fixed-height `VirtualList` is an O(1) range calculator for very large data sets; it never allocates
in `visible_rows()` and mounts only viewport rows plus configured overscan. A viewport bound with
`Element::virtual_scroll` shares its offset through one lock-free handle and enters the same
retained scrollbar path as an ordinary overflow container. Wheel and drag motion mark the view
dirty only when the offset changes; hover expansion and autohide remain paint-only.

## Rendering

The current renderer has six specialized pipelines. Windows using the same performance profile
share a compatible WGPU instance, adapter, device, and queue; a surface-incompatible window falls
back to its own context. Surface configuration, pipelines, upload buffers, text state, and bounded
asset caches remain per-window, preventing one window's working set or resize from invalidating
another:

1. Shapes use six shader-generated vertices and one declaration-ordered instance stream. Quads carry logical bounds, fill, inside border, radius, and clip. Drop and inset shadows carry the element box, translated/spread subject box, color, blur radius, and clip. Rounded-corner antialiasing and Gaussian-CDF shadow falloff are analytic in WGSL, so shadows allocate no blur textures or retained cache entries. Outer shadows, the element quad, and inset shadows preserve CSS paint order while every non-empty stacking layer remains one draw call.
2. Images use six shader-generated vertices and one shared instance upload. Straight-alpha RGBA8 pixels are sampled from sRGB textures, converted to premultiplied linear output in the shader, and masked by the same logical clip and rounded box used for layout. Consecutive primitives with the same image identity share one draw call; texture switches preserve image source order.
3. SVGs retain a parsed `resvg` tree and rasterize only an identity-and-physical-size cache miss.
   The GPU stores `R8Unorm` alpha masks, while inherited tint, rounded clipping, translation, and
   rotation remain per-instance shader data. Scale participates in the raster key to preserve edge
   quality. Consecutive primitives with the same mask share a draw without making color part of the
   cache identity.
4. Paths retain CPU-tessellated Lyon triangles. Each uploaded vertex carries barycentric coordinates,
   a true-boundary mask, and a paint index; WGSL derives edge coverage only for outline edges, so
   internal triangulation stays opaque without a permanent multisample framebuffer. One storage
   paint record supplies scale/translation, clip, solid or two-stop linear gradient, and
   linear-sRGB/sRGB/Oklab interpolation. All paths at one overlap order share one draw.
5. Application shaders retain validated WGSL fragment functions behind a framework-owned vertex
   stage and fragment wrapper. Rect, physical clip, and four `vec4<f32>` parameter slots are
   per-instance data; shaders sharing one retained identity batch at an overlap order. The wrapper
   enforces clipping and converts linear straight alpha to premultiplied output.
6. Text uses Cosmic Text for Unicode shaping/fallback and Glyphon for Swash rasterization plus atlas rendering. Stable `TextId`s prevent reshaping unless content, width, metrics, family, weight, wrap mode, or display scale changes.

Colors are stored in linear-light space. Eight-bit constructors decode sRGB on the CPU, and the sRGB surface performs the output transfer. Shape, image, SVG, path, and application-shader output is premultiplied before using premultiplied-alpha blending. Text wraps at word boundaries by default; intrinsic measurements reserve one physical pixel before a max-content width is fed back as a wrap constraint, preventing rounding-only reflow between layout and paint.

Shape instance uploads rotate across three GPU buffers. Capacity grows geometrically. A shape
instance is 96 bytes, and shadow overdraw is clipped to the viewport and limited to three Gaussian
standard deviations. Text layouts are age-evicted every frame and have a hard cap of 256 retained
areas; Glyphon's atlas is trimmed after presentation. The deliberately small layout cache keeps
long, disjoint scrolling from retaining whole off-screen text buffers while still covering several
nearby viewports.

Image instances also rotate across three geometrically growing buffers. Decoded CPU images are
immutable `Arc` allocations, so cloning an `Image` preserves cache identity without copying pixels.
Only viewport-visible image identities are admitted for upload. The per-renderer GPU cache is capped
at both 256 textures and 128 MiB of RGBA texels; an upload evicts the least-recently-used identity
that is not visible in the current frame. If one visible working set itself exceeds either cap,
admission is deterministic in scene order and later images remain unsubmitted rather than allowing
unbounded residency. Cache bytes, image count, uploads, and total draws are exposed in
`RenderStats`.

SVG parsing accepts at most 4 MiB of source or bounded SVGZ output and loads the system-font
database only if the source may contain text. SVG `<image>` references are disabled, keeping the
asset path vector-only. A raster target is derived from the fitted source UVs, display scale, and
render scale, then constrained to 4096 px per axis and 16 million pixels. Only visible unique keys
are admitted in scene order. The per-renderer cache retains at most 512 one-channel textures and 32
MiB, evicting the least-recently-used key outside the visible working set. First-use rasterization
is synchronous; cache hits perform no SVG parsing or CPU raster work, and recoloring never uploads a
new texture.

`PathBuilder` validates coordinates and styles before tessellating fills or strokes. A retained path
is immutable and shared by identity; it owns de-indexed triangles plus one three-bit boundary mask
per triangle. Commands are capped at 65,536, retained vertices at 196,605, dash expansion at 262,144
segments, and retained geometry at 4 MiB. The renderer rotates both vertex and paint uploads across
three geometrically growing buffers. Frame admission follows source order and is capped at 262,144
vertices and 16,384 paints; later paths are omitted rather than growing transient memory without a
bound. Declarative path elements use intrinsic Flexbox measurement and all five `ObjectFit` modes.
Canvas callbacks receive local bounds and a scoped painter whose commands are translated and clipped
to the element before entering the same retained scene.

`CustomShader::new` composes an application `quickgui_fragment` function with QuickGUI's fixed
interface, then runs Naga parsing and validation before the asset enters retained state. Bind-group
resources, pipeline overrides, and application entry points are rejected. The source is capped at
64 KiB and stores one shared immutable allocation. Pipeline compilation is lazy under a WGPU
validation error scope; each window retains at most 32 pipelines and evicts the least-recently-used
identity outside the visible working set. Admission keeps at most 4,096 visible rectangles per
frame in source order. Rectangles use one 96-byte instance record, one upload, and three rotating
geometrically-grown buffers, with no per-effect bind groups or intermediate textures. Custom
primitives enter the same `(plane, z_index)` layers and overlap-order stream as every other paint.
No time uniform or redraw source is implicit, so static effects add no idle CPU work. WGSL remains
trusted application code because validation cannot prevent a deliberately non-terminating GPU
program from resetting the device.

Asynchronous image resources are resolved before Taffy layout. Filesystem paths share a stable path key, while
custom loaders use a retained handle identity. The first resource in the application lazily creates
two sleeping decode workers backed by one shared 64-job synchronous channel. Every job and
completion carries its owning `WindowHandle`; completed decodes return through the event-loop
proxy, so only that clean window wakes once rather than polling worker state. Loading and fallback closures
produce ordinary child elements and therefore participate in the same flexbox and text-wrapping
rules as the rest of the tree. The per-window CPU cache independently caps all resource states at
256 entries and decoded residency at 128 MiB, evicting least-recently-used entries outside the
current tree. Each file is capped at 64 MiB encoded, and every decoded image retains the existing
4096 px per-axis and 64 MiB allocation limits. Worker panics become failed resources, and teardown
detaches instead of waiting indefinitely on application-provided blocking loaders.

`ViewContext::spawn_background` uses a separate application-wide two-thread pool with a 64-job
synchronous queue. Work and its type-erased completion are tagged with the owning `WindowHandle`;
one user event wakes that window and downcasts the callback to its concrete view type. A full queue
fails the spawn synchronously, worker panics become typed completion errors, and idle workers
sleep. Neither image loading nor blocking application work consumes event-loop frames while
pending. `spawn_blocking` is an explicit alias.

## Foreground tasks

`ViewContext::spawn` and `EventContext::spawn` schedule a non-`Send` future with `async-task`'s
local executor. The scheduler callback may be woken from any thread, but it only moves a `Runnable`
into one bounded synchronized ready queue and sends one coalesced Winit user event. The application
thread drains at most 256 ready futures per event-loop turn before yielding to platform input; it
is the only thread that polls or destroys application future state. The application admits at most
4,096 live foreground tasks and each native window owns at most 1,024.

The public `Task<T>` handle is deliberately main-thread-only. Dropping it requests cancellation;
`detach()` releases the result handle without releasing window ownership. Closing the owner window
aborts both retained and detached work, discards its queued view updates and timers, and prevents a
late wake from touching a replacement view. Runtime shutdown closes and drains the ready queue on
the application thread. A bounded leak is used only if an external waker races after that thread
has stopped accepting work, because destroying captured `Rc` state on the waking thread would be
unsound.

`AsyncViewContext<V>::update` does not reenter the view from inside a future poll. It moves one
typed closure into the task's at-most-64-entry update queue, returns `Pending`, and the runtime
executes that closure against the owning view after the runnable releases executor state. The
closure result then wakes the future. Missing owners and mismatched view types are reported through
`AsyncContextError` rather than dereferencing stale state.

`AsyncViewContext::sleep` registers one monotonic deadline in the task registry. The earliest task
timer joins image, animation, scrollbar, tooltip, key-sequence, and requested-repaint deadlines in
Winit's `ControlFlow::WaitUntil`; when due, its waker is scheduled exactly once. There is no timer
thread, interval tick, polling future, or animation-frame request. A task owns at most 64 pending
timers, the application retains at most 4,096 timers, and one task can queue at most 64 view
updates.

Animated GIF/WebP resources decode their composited RGBA frames on the same bounded workers.
Direct `AnimatedImage` values and loaded animations share one immutable representation capped at
256 frames and 64 MiB of unique decoded pixels; repeated `Image` identities are counted once. Frame
delays are clamped to 16.667 ms so malformed assets cannot force more than 60 presentations per
second. Playback state is keyed by the element's stable runtime ID rather than the asset, allowing
two uses of one animation to retain independent phases across view rebuilds. Only elements that
intersect the current clip remain active. Occlusion, explicit motion reduction, and macOS's system
Reduce Motion preference pause deadlines while preserving the current frame. Finite loop metadata
stops on the final frame. A delayed event advances elapsed time arithmetically and performs at most
one repaint, rather than iterating through every missed frame.

The scene groups primitives by `(plane, z_index)` and retains at most 16 inactive layer buffers.
Every accepted primitive also enters one global paint stream. An allocation-reusing R-tree assigns
each primitive one greater than the maximum order of intersecting earlier bounds; primitives sharing
an order are provably disjoint and may batch by pipeline or texture without changing pixels. The
renderer walks orders globally across shapes, paths, images, SVGs, and text, preserving web sibling
overlap while avoiding one draw per primitive. Effective bounds include clips, masks, and render
transforms. Every overlay layer remains after every base layer. Glyphon renderers are shared by
text-bearing order batches and capped at 32 retained instances.
An ordinary window still uses one WGPU surface. A macOS window with native children creates its
transparent overlay surface only when overlay content or input first becomes active, then retains
that swapchain for reuse. Recreating it on every close/open cycle is intentionally avoided because
Core Animation may keep old IOSurface drawables alive beyond the Rust `Surface` lifetime.

On macOS, QuickGUI creates and retains each `CAMetalLayer` itself, then gives WGPU the documented
`CoreAnimationLayer` surface target. AppKit live-resize callbacks only record the newest physical
size; WGPU reconfiguration is deferred to the next coalesced redraw immediately before drawable
acquisition. During AppKit's live-resize transaction, the owned base and overlay layers enable
`presentsWithTransaction`; ordinary frames disable it again. This keeps resize presentation in the
window-server transaction without reaching through WGPU's backend internals or configuring a
surface repeatedly inside `frameDidChange:`.

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
Winit APIs that resolve the window's current content view, such as IME cursor placement, are
skipped only during that detached preparation pass and resume after RAII reattaches the same view.
The declarative `WindowState` snapshot uses the retained initial mode for that one pass instead of
querying a temporarily absent AppKit content view.

## macOS window chrome

`WindowKind::Dialog` is parent-modal and is presented as an AppKit sheet only after its hidden GPU
frame is complete. `Floating` and `PopUp` use their native elevated levels; the latter is transient
across spaces and hides on application deactivation. Each kind uses the same retained view, input,
accessibility, scheduling, and bounded renderer ownership as a normal window.

`TitleBarStyle::HiddenInset` keeps AppKit's standard window controls while making the titlebar
transparent and extending the Winit content view through it. Traffic-light coordinates are logical
top-left points; the runtime converts them through AppKit's content-layout rect and preserves the
native inter-button spacing. Resize and scale transitions reapply the configured position.

Hidden-inset creation sets `NSWindow.movable` to false, removing AppKit's implicit titlebar drag
region. An element's optional `AppRegion::Drag` or `AppRegion::NoDrag` declaration enters the same
ordered hit list as pointer targets. The topmost explicit declaration wins, with built-in overlay
scrollbars taking precedence. A primary press in a drag region starts AppKit's synchronous native
window drag, enabling movability only for that call and restoring the hidden-inset restriction on
return. No drag-region hover or click state is painted and no continuous tracking loop is added.

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

CPU frame time, quad/shadow/image/SVG/path/text counts, path vertices and skipped paths, CPU/GPU image residency, GPU SVG-mask residency,
resource loading/failure counts, mounted/active animation counts, image uploads, SVG
rasterizations, text cache hits, and draw-call counts are exposed through `FrameMetrics`. These are
application-side timings through queue submission, not GPU timestamps or end-to-end display
latency.

## Native input and accessibility

The retained tree owns semantic focus and exposes it through AccessKit before the macOS window becomes visible. Pointer focus, Tab/Shift-Tab traversal, Space/Return button activation, programmatic focus handles, and accessibility actions all update the same focus owner.

`StyledText` stores one shared UTF-8 string and at most 4,096 sorted, non-overlapping highlight
ranges. The complete value remains one Cosmic Text buffer and one text scene primitive; span
attributes supply foreground, family, weight, italic, and native decoration metadata. Background
geometry is grouped from visible glyph metadata in one pass, while Cosmic Text's font-derived
underline and strikethrough metrics produce decoration rectangles. Background shapes enter the
ordered scene before glyphs and decoration shapes after glyphs, all through the existing instanced
quad pipeline. The retained layout key includes byte ranges and glyph/decoration attributes but
intentionally excludes background color, so selection or syntax-background recoloring does not
reshape. The per-window 256-entry text bounds and eight-frame age eviction remain unchanged.

Ordinary and styled immutable text leaves register their retained buffer geometry in document
order during paint. One per-window selection stores anchor/focus endpoints as stable element IDs
plus UTF-8 byte offsets, so a captured drag can cross separately laid-out leaves without merging or
reshaping them. Double-click expands to adjacent Unicode word segments, triple-click expands to a
newline-delimited logical line, and Shift extends the retained anchor. Controls suppress automatic
selection unless their subtree explicitly opts back in. Selection rectangles come from the same
Cosmic Text buffer and are painted before glyphs, including BiDi and styled runs.

Static selection participates in native Edit-menu validation, platform Copy/Select All shortcuts,
and AccessKit `TextRun` selection actions. Cross-leaf copy preserves document order, inserts a
space for leaves on one visual line or a newline between lines, and stops at 8 MiB on a UTF-8
boundary. Selection state alone invalidates paint: it neither changes layout nor starts an idle
frame loop.

On macOS, the renderer removes the system `GB18030 Bitmap` face from Cosmic Text's font database
at startup. That legacy non-scalable face reports itself as a monospaced CJK fallback but yields
infinite advances and cannot be rasterized by Swash. Filtering it once lets normal scalable script
fallback handle mixed Latin, CJK, Arabic, and emoji without per-layout probing, text fragmentation,
or additional frame work.

Text fields and text areas keep UTF-8 byte ranges internally but move and delete at Unicode grapheme and word boundaries. Cosmic Text provides wrapped visual-line caret movement, two-dimensional hit testing, and bidirectional per-line selection rectangles from the same retained layout used for painting. Text areas share the retained scroll map and native-style overlay scrollbar with ordinary scroll containers, including nested wheel propagation, captured dragging, and one-shot auto-hide deadlines. Composing text is retained separately from the controlled value, Winit preedit cursor offsets remain byte-indexed, and the native IME candidate rectangle follows the painted and scrolled caret.

`styled_text_input` and `styled_text_area` attach the same immutable, at-most-4,096-run table used
by `StyledText` to the controlled editor value. One Cosmic Text entry supplies measurement, caret
positions, hit testing, selection and marked-text geometry, backgrounds, glyphs, and decorations;
there is no per-span element tree. An accepted replacement shifts unaffected runs, splits an
intersected run, inherits style only inside a run, and merges adjacent equal styles. The next
controlled render can replace that provisional table with application syntax or semantic runs.
IME cancellation restores the exact text and table backup, while undo snapshots include shared
run tables and count both `TextHighlight` storage and named-family bytes inside the existing 512
KiB per-stack budget. Background-only recoloring still reuses glyph shaping, and editing creates no
timer or continuous frame request.

Every committed edit constructs one proposed complete value. Maximum grapheme length and the optional application filter run before retained text, selection, or history changes; rejected IME commits restore the preedit backup. This keeps typing, paste, accessibility `SetValue`, IME commit, undo, and redo on one transaction boundary without cloning history per keypress. Listener comparisons use the committed composition backup rather than visible preedit text, so preedit stays private while a real commit emits exactly one controlled-value change. Invalidity is declarative and paint-only, with matching AccessKit invalid state and validation description. A single-line Return listener receives only the committed value, ignores repeat, and is suppressed while invalid; text areas retain normal newline behavior.

Semantic forms reuse the retained parent map to resolve the nearest owner on Return or submit-button activation. Validation walks only that form subtree at the user-action boundary, stops at nested forms, excludes disabled controls, and records controls in document order. Valid attempts share at most 256 retained input values with the callback; invalid attempts retain at most 256 issues with 4 KiB UTF-8-safe messages, focus the first focusable issue, and replace one AccessKit assertive live node. The live node receives a fresh identity even for an identical retry, then is removed by a valid attempt or an unmounted form. Programmatic requests coalesce within a bounded 64-entry event queue, and recursive callbacks stop after eight nested attempts. None of this adds timers, polling, per-frame validation, or copied text-area payloads.

AccessKit receives a stable `TextRun` child, grapheme character lengths, editable value, multiline semantics, native text-selection state, and validation metadata. Clipboard objects are created lazily so applications that never copy or paste pay no startup cost. Undo and redo retain at most 100 text-plus-style snapshots and 512 KiB per direction for each mounted input; an external controlled-value replacement clears history.

When native AppKit children are mounted, QuickGUI installs a no-ivar Objective-C subclass above AccessKit's existing Winit-view subclass. Its child and navigation queries preserve AccessKit's virtual nodes and append the live native accessibility roots in composition order. Accessibility focus and screen-point hit testing route into a native subtree when its responder owns focus or its frame contains the point; otherwise they fall through to AccessKit. Identity/order changes post one AppKit layout notification, while ordinary redraws allocate nothing and post nothing. Teardown restores AccessKit's exact class before AccessKit later restores Winit's class.

## Current boundaries

This milestone establishes the performance architecture, ordered overlays, bounded delayed
tooltips, cursor-anchored context surfaces, macOS native child
composition, hybrid AccessKit/AppKit accessibility, semantic focus, and production-oriented
single-line and multiline editing paths, typed actions, focus scopes, contextual keymaps, native macOS application
menus, bounded static/asynchronous/animated raster images, retained SVG icons, retained paths and
custom canvas painting, retained custom shaders, retained CSS Grid, static and controlled editable styled text, structured forms, typed entity events, application globals, cancellable foreground tasks, and analytic CSS-ordered box shadows. A complete
platform toolkit still needs true anchored/nonactivating native popup panels, advanced gesture
input, deterministic application test
contexts, non-macOS menu projection, cross-window native drag promotion, and broader platform
acceptance. Those features should extend the retained tree and narrow renderer rather than bypass
its scheduling and cache invariants.
