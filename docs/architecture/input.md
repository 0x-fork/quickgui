# Input and interaction architecture

[Architecture index](README.md) · [Documentation](../README.md)

## Element identity and ownership

The application owns its `View`. Each render returns a declarative `Element` tree. Explicit `.id(...)` keys are reserved and validated before generated path IDs are assigned, so duplicate developer keys are errors while hash collisions for unkeyed nodes are deterministically probed around.

An element ID owns:

- hover and pressed paint state;
- captured pointer-listener identity;
- pressure, pinch, rotation, and smart-magnify listener identity;
- secondary-click and tooltip-trigger identity;
- drag-source/drop-target identity and one active typed payload;
- focus identity and keyboard traversal position;
- click or controlled-input listener lookup;
- scroll-container and text-area offsets;
- text-input caret, selection, composition, and preferred visual column;
- text-layout and glyph-cache identity.

`ViewContext::listener`, `ViewContext::pointer_listener`, `ViewContext::context_menu_listener`,
the targeted desktop mouse, focused key, and four native gesture listener constructors,
`ViewContext::drag_listener`, `ViewContext::drop_listener`, and
`ViewContext::action_listener` erase callbacks into the owning window's registry and restore the
concrete view type at invocation. Most element attachments carry only the stable ID. The opt-in
mouse, focused-key, and action attachments additionally retain compact callback-slot and phase
bindings so multiple callbacks with the same element ID and payload type remain exact. Their
optional boxed lists preserve the compact ordinary `Element` path while callbacks can still mutate
`&mut Self` without `Rc<RefCell<_>>` application state. Pointer capture retains only one target,
origin, and latest position per window; it ends on button release or window-focus cancellation and
does not schedule frames while idle.

Targeted desktop mouse callbacks are distinct from direct pointer capture. A declaration stores
one optional boxed list only on participating elements; the window registry stores erased callback
slots, and the retained tree rebuilds a flat binding array plus element ranges. Press/release
dispatch resolves one topmost visual target, retains at most 256 target-to-root IDs, invokes
outside capture first, then capture root-to-target and bubble target-to-root. Motion uses only that
ancestor path, so it never scans unrelated listeners. Callback slots, per-element declarations,
and pending hover changes have explicit hard bounds, while window-owned vectors and the existing
hover hash tables are cleared and reused between events.

Propagation and retained defaults are accumulated independently. A prevented press never changes
focus, selection, click, context-menu, dismissal, or drag-start state. A prevented release still
clears pressed styling and selection gestures; direct pointer capture and active drag teardown run
before preventable mouse-up defaults. The macOS Winit support event copies AppKit's exact
multi-click count before buffered delivery, with a finite time/distance tracker for other backends.
Hover listener paths are retained
separately from paint hover state and reconciled both on motion and after an already-damaged paint,
so moving layout beneath a stationary pointer creates entry/exit callbacks without an idle poll.
The versioned macOS Winit event copies `NSEvent.clickCount` before Winit buffers the event; the
runtime therefore never depends on a stale `NSApplication.currentEvent`. Native window exit keeps
the last retained in-window hit point distinct from the hardware boundary sample used only for
external-drag promotion, so `on_mouse_exit` receives the element path that was actually left.

Cursor declarations are plain copied values on elements and hit regions; they do not own an
element ID, callback, or scheduler source. Paint resolves explicit style first, then bounded
inference for selectable text and inactive drag sources. Pointer movement walks the existing
topmost hit stack and changes Winit's cursor only when the mapped native value differs. Each
already-damaged paint performs one lookup at the retained stationary pointer, reconciling both
state-dependent declarations and content that scrolled underneath it. Scrollbars and application
drag regions intercept this lookup with the default arrow, preserving their native interaction
contract without a separate tracking monitor. Cursor state declarations are stored separately
from paint overrides, so a cursor-only hover edge does not invalidate the scene.

## Focused actions and raw keys

The dispatch index stores flat action and key binding arrays plus element ranges alongside the
existing parent map. A keymap match first dispatches its typed action. Capture action slots run
root-to-focus and propagate by default; a capture callback can stop the complete remaining path.
Bubble action slots then run focus-to-root and consume individually by default, so
`EventContext::propagate` is an explicit fallback decision. Native-menu availability consults the
same attached binding index rather than treating an unattached callback registration as a command.

Only an action that reaches the end of bubble falls through to raw key dispatch. Key-down and
key-up slots then run root-to-focus capture followed by focus-to-root bubble. The path and callback
slots are copied into reused window scratch before the first callback, so focus changes,
invalidation, nested actions, or listener declarations for the next render cannot mutate an event
already in flight. `stop_propagation` and `prevent_default` are accumulated independently. The
latter gates direct text editing, traversal, activation, dismissal, and default close behavior;
separate IME preedit/commit events retain platform authority. The legacy window-level `View::event`
remains a coarse observer after targeted dispatch.

Focused paths retain at most 256 ancestors. Elements retain at most 32 action and 16 key bindings;
each window callback registry retains at most 4,096 of either kind. Callback vectors, flat binding
arrays, range maps, and dispatch scratch are cleared and reused. A window without these listeners
adds no dispatch allocation, monitor, deadline, or redraw source, and dispatch itself never scans
unrelated elements.

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

The platform keystroke preserves two identities. `key` is the normalized command identity;
`key_char` is the printable character the press could produce before composition. Matching first
uses the command identity. When `key_char` differs, it may also match a binding after removing
Option and Shift while retaining Control and Command, following the native distinction between a
layout key and its printable result. The first-stroke hash index probes only those two identities;
it never falls back to scanning the complete keymap. Committed text stays on the independent IME
path and is never synthesized from either command candidate.

The macOS runtime owns one immutable layout snapshot and a fixed table for all 128 virtual key
codes. Each entry stores at most four UTF-16 code units translated for six modifier combinations;
UTF-16-to-UTF-8 conversion writes into inline fixed buffers. TIS input-source and layout-data
objects are released immediately after the table is built. AppKit's keyboard-selection
notification replaces the table, clears any multi-stroke prefix that would otherwise span two
layouts, and invalidates only views whose current declaration read `keyboard_layout()`. Ordinary
keypress matching performs a physical-key lookup plus fixed-buffer reads and adds no polling,
timer, or retained native object.

Key-equivalent localization is separate and opt-in per binding. A finite static table maps the
single-character declaration to Apple's observed localized equivalent for the active input-source
ID. The keymap retains one shared immutable copy of the original strokes only for opted-in
bindings. At a layout notification it rewrites their small effective stroke vectors and rebuilds
the existing first-stroke hash index; ordinary dispatch stays a direct indexed lookup. Native menu
items consume those effective strokes and disable AppKit's second automatic localization pass.
There is no per-key map scan, input-source query, menu probe, or idle task.

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

The unstyled surface derives stable child IDs from one caller-owned identity, registers its query,
row-click, and typed navigation listeners in the normal per-view registry, and mounts only the
`VirtualList` visible range plus one overscan row. `PickerLayout` retains only bounded row height
and visible-row count. The application supplies the input, empty state, row elements, highlighting,
status content, and root appearance. Selection skips disabled entries, wraps for Up/Down, reveals
with the smallest possible scroll delta, and exposes native selected/disabled semantics.
`AnyAction` values preserve their original concrete payload. Activation can close the overlay,
restore the underlying focus path, then call `dispatch_any_action`, so key bindings, buttons, the
palette, and native menus reach the same handlers.

No picker work occurs during paint, pointer hover, or idle time. Query/source replacement performs
one synchronous bounded match pass; wheel input follows the existing virtual-scroll path, while
keyboard movement changes only selection and the retained offset. There is no debounce timer,
polling task, or display-rate animation.

## Controlled popovers

`Popover` pairs two caller-owned stable IDs with one application-owned open boolean. Its unstyled
trigger has exact expanded, controls, and has-popup accessibility state. A separate caller-owned
positioner is the existing portal overlay configured for anchor fitting; the caller-owned popover
retains topmost independently configurable Escape/outside-press dismissal, pointer blocking,
visible title/description relationships, and trigger focus restoration. A merged unstyled surface
is available when independent positioner presentation is unnecessary. Closed content is unmounted
rather than retained in a hidden component store.

Opening focus uses the runtime's existing one-rebuild pending focus request. A listener can set the
controlled value and call `focus_surface` in the same event; an optional declared initial target or
the popover root is resolved after mounting and then discarded. Nested popovers derive dismissal order
from paint order, so no independent global stack or listener monitor is necessary. A settled popover
owns no timer, frame request, polling task, or idle deadline.

That retained overlay is physically window-bounded. `SystemPopover` is the overflow host for
menus, selects, and other dropdowns: it resolves the trigger ID from retained geometry at the
event boundary and opens a parent-owned child window with its own WGPU surface. On macOS collision
constraints use the display work area, not the parent viewport. The lookup is event-driven and
installs no per-element geometry observer.

`PopoverMenu` composes its bounded model into that child using ordinary elements. One root keeps
semantic focus and projects the highlighted derived row as its active descendant. Keymap actions
handle directional/boundary movement, activation, submenu open, and close; one raw-key listener
performs bounded alphanumeric prefix navigation. Pointer hover updates the same highlighted index,
so input methods cannot diverge. Toggle and radio preview state is local to the open model while
the original concrete action reaches the non-popover owner window's existing focused path.

The event context resolves direct parent, popover root, and nearest non-popover owner from retained
window ownership. Targeted actions queue only the `(WindowHandle, AnyAction)` pair until the current
callback releases its view borrow. A nested submenu therefore bypasses intermediate popover views
without a command bridge. Closing the popover root reuses child-first teardown for the entire menu
chain. Both targeted delivery queues and popover nesting are hard bounded; clean menus retain no
deadline, task, polling source, or animation frame.

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

Duration animations and springs declared inside the content retain a tooltip-local motion clock.
Their frames rebuild and lay out only the detached tree, never the application view. The resolved
result is sanitized again so an animator cannot add interaction or nested transient surfaces.
Hiding the tooltip drops the motion registry and its exact deadline immediately.

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

## Native input and accessibility

The retained tree owns semantic focus and exposes it through AccessKit before the macOS window becomes visible. Pointer focus, Tab/Shift-Tab traversal, Space/Return button activation, programmatic focus handles, and accessibility actions all update the same focus owner.

Winit maps AppKit wheel motion, magnification, rotation, smart magnification, Force Touch, and raw
`NSTouch` contacts into window events. QuickGUI keeps their public payloads `Copy` and fixed-size:
scroll deltas retain their precise-pixel or discrete-line identity, pressure is finite and clamped
to one, pinch and rotation deltas are finite and bounded, and unknown future pressure stages remain
an explicit value rather than being discarded. Winit's AppKit view refreshes the precise cursor
location immediately before every pointer-relative event. The retained hit tree examines only
topmost visual regions and their parent chain, so wheel and touch listeners bubble child-first,
another listener can stop propagation, and a higher pointer-blocking overlay prevents
click-through. Default wheel scrolling is a separate decision and can be prevented without
stopping ancestor callbacks.

The Winit content view opts into AppKit's direct
[`allowedTouchTypes`](https://developer.apple.com/documentation/appkit/nsview/allowedtouchtypes),
excludes resting contacts, and queries only the phase-specific set delivered by each responder
callback. Indirect trackpad contacts expose device-normalized coordinates rather than a screen or
view location, so they remain on the native scroll and gesture paths and never enter window
hit-testing. One direct-contact ID is derived from
[`NSTouch.identity`](https://developer.apple.com/documentation/appkit/nstouch) and is retained only
through that contact's lifetime. QuickGUI hit-tests `Started` once and stores at most 32
`TouchId -> target/last-sample` captures per window. Move and terminal events reuse the target;
focus loss removes captures one at a time while dispatching synthetic cancellation, without
building a temporary contact list. Native coordinates become bounded top-left logical points, and
optional platform force becomes finite normalized scalar data. macOS `NSTouch` supplies no force
in this bridge.

Windows without an explicit wheel listener still aggregate platform deltas once at the frame
boundary and move retained scroll offsets without rebuilding layout. Listener declarations retain
no per-event allocation or scheduling state; native events are delivered individually only when an
application asks for exact delta type or gesture phase. There is no gesture monitor, timer,
recognizer registry, or permanent redraw source, and callback invalidations share the normal
coalesced scheduler.

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

`Field` is a declaration-time relationship layer over those existing controls, not another form
runtime. A stable control ID derives root, label, description, and error IDs; the mounted control
references the visible label and description plus the visible error while invalid. A normal label
adds one retained activation edge that focuses a text input or activates a clickable control after
the label event, while a passive label adds only naming. `Fieldset` projects a named group and
passes its disabled bit explicitly into fields created from the descriptor, so it needs no
descendant registry or inherited-state walk. Required/invalid/disabled flags and one 4 KiB bounded
shared message are the only semantic payload; colors, geometry, typography, focus treatment, and
error motion remain application-owned. Settled fields create no frame, task, timer, or observer.

`Collapsible` reuses the ordinary button path for pointer, Enter, and Space activation. Its trigger
projects expanded state and a controls edge only while the caller-controlled panel is mounted.
`Accordion` composes the same disclosure behavior with semantic headings and panel regions. In
accordance with current APG guidance, every enabled accordion trigger remains in the normal Tab
sequence; there is no roving-focus registry, orientation mode, or arrow-key interception.
`AccordionState` retains only a sorted, bounded vector of open stable IDs, so closed items cost no
state and settled disclosure trees add no scheduling source.

AccessKit receives a stable `TextRun` child, grapheme character lengths, editable value, multiline semantics, native text-selection state, and validation metadata. Clipboard objects are created lazily so applications that never copy or paste pay no startup cost. Undo and redo retain at most 100 text-plus-style snapshots and 512 KiB per direction for each mounted input; an external controlled-value replacement clears history.

When native AppKit children are mounted, QuickGUI installs a no-ivar Objective-C subclass above AccessKit's existing Winit-view subclass. Its child and navigation queries preserve AccessKit's virtual nodes and append the live native accessibility roots in composition order. Accessibility focus and screen-point hit testing route into a native subtree when its responder owns focus or its frame contains the point; otherwise they fall through to AccessKit. Identity/order changes post one AppKit layout notification, while ordinary redraws allocate nothing and post nothing. Teardown restores AccessKit's exact class before AccessKit later restores Winit's class.
