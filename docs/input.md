# Input and interaction

[Documentation index](README.md)

## Native cursor styles

`CursorStyle` exposes the same macOS cursor vocabulary as GPUI, with a typed setter and
Tailwind-compatible helpers:

```rust
use quickgui::{CursorStyle, div};

let splitter = div().cursor(CursorStyle::ResizeLeftRight);
let link = div().cursor_pointer();
let disabled_action = div().disabled(true).cursor_not_allowed();
let draggable = div()
    .cursor_grab()
    .active(|style| style.cursor_grabbing());
```

The helper family covers `default`, `pointer`, `text`, `move`, `not-allowed`, `context-menu`,
`crosshair`, `vertical-text`, `alias`, `copy`, `no-drop`, `grab`, `grabbing`, every edge and
diagonal resize direction, and row/column resize. The same helpers are available inside `hover`,
`active`, `focus`, validation, and drag state closures. An explicit `cursor_default()` is distinct
from having no declaration: it resets a cursor contributed by a lower hit region, just as a
foreground web element can override an ancestor or covered surface.

Cursor resolution uses the existing topmost retained hit stack. Pointer blockers prevent styles
behind them from leaking through; selectable text infers an I-beam; inactive drag sources infer an
open hand and an active internal drag uses a closed hand. Disabled controls lose inferred cursors,
while an explicit style such as `cursor_not_allowed()` remains visible. Native scrollbars and
`app_region_drag()` retain the default arrow.

The runtime changes the native cursor only when its resolved value changes. Cursor declarations
add no timer, animation frame, allocation-sized event payload, redraw, or idle polling source.
Each already-damaged paint refreshes the cursor once at a stationary pointer, so a view rebuild or
scroll cannot leave the icon from content that moved away. Cursor-only hover declarations resolve
directly from retained state and do not request a paint. Run
`cargo run --release --example cursors` to inspect every style.

## Desktop mouse events

Ordinary desktop input uses a bounded web-style target path while the existing `on_pointer`
contract remains the direct press-to-release capture primitive for scrollbars, splitters, and
custom drags:

```rust
let capture = cx.mouse_down_listener("editor", |this, event, cx| {
    this.hide_completion_menu();
    // Propagation and retained defaults are separate decisions.
    if this.tool_owns(event.position) {
        cx.prevent_default();
    }
});
let down = cx.mouse_down_listener("line", |this, event, cx| {
    this.select_click(event.position, event.click_count, event.modifiers);
    cx.invalidate();
});
let hover = cx.hover_listener("line", |this, hovered, cx| {
    this.line_hovered = *hovered;
    cx.invalidate();
});

div()
    .id("editor")
    .capture_any_mouse_down(capture)
    .child(
        div()
            .id("line")
            .on_mouse_down(MouseButton::Left, down)
            .on_hover(hover),
    )
```

Capture runs root-to-target and bubble runs target-to-root. The attachment API includes
`on_mouse_down`, `on_any_mouse_down`, `capture_any_mouse_down`, `on_mouse_down_out`, matching
mouse-up forms, `on_mouse_move`, `on_mouse_exit`, and boolean `on_hover`. Outside handlers run as
capture before the in-path target, which lets a transient surface close without racing the control
underneath it. `stop_propagation()` ends the remaining path. `prevent_default()` independently
suppresses QuickGUI focus, text selection, click, context-menu, dismissal, and drag-start behavior;
terminal direct pointer capture and drag cleanup cannot be prevented.

`MouseDownEvent` and `MouseUpEvent` include the logical position, button, modifiers, and shared
multi-click count. The versioned macOS Winit support crate copies AppKit's exact bounded count into
the queued window event before returning from `sendEvent:`; reading `NSApplication.currentEvent`
later would be incorrect because Winit buffers delivery. Other backends use the same bounded
500 ms/four-point fallback. Motion and exit events expose the most recently pressed button without
allocating a button set. `on_hover` reacts both to pointer motion and to layout moving beneath a
stationary pointer. A hover callback that needs geometry can read `cx.pointer_position()`; this is
the current event's window-local snapshot and does not query the platform or install tracking work.

An element that declares no desktop mouse callback retains only one empty optional pointer and
allocates nothing for this layer. Opt-in declarations are capped at 16 per element and 8,192 per
window; dispatch paths are capped at 256 ancestors and reuse window-owned scratch buffers. Mouse
motion performs no outside-listener scan, schedules no timer, and creates no idle frames.
Run `cargo run --release --example mouse_input` to inspect capture order, outside presses, hover,
and native double/triple-click counts. The macOS acceptance gate posts bounded native events through
AppKit and verifies exact left/right capture, bubble, outside, prevention, drag, hover, exit, and
multi-click behavior through Winit's production path.

## Scroll wheel, pressure, and gestures

Custom canvases, maps, editors, and timelines can receive GPUI-shaped wheel input without moving
ordinary scrolling off its retained fast path:

```rust
let wheel = cx.scroll_wheel_listener("canvas", |this, event, cx| {
    let pixels = event.delta.pixel_delta(40.0);
    this.zoom = (this.zoom * (1.0 + pixels.y * 0.003)).clamp(0.5, 3.0);
    cx.prevent_default();
    cx.invalidate();
});

div().on_scroll_wheel(wheel)
```

`ScrollWheelEvent` retains the logical pointer position, modifiers, and native
`Started`/`Moved`/`Ended`/`Cancelled` phase. `ScrollDelta::Pixels` preserves precise trackpad
motion, while `ScrollDelta::Lines` preserves discrete wheel units until the application calls
`pixel_delta(line_height)`. Non-finite values become zero and public per-event pixel and line
bounds protect application arithmetic.

Wheel callbacks bubble from the topmost listening element through listening ancestors. Input
propagates by default; `cx.stop_propagation()` skips later ancestor callbacks, while
`cx.prevent_default()` independently suppresses the retained scroll container under the pointer.
This matches the web distinction between propagation and default behavior.

An ordinary scroll container with no explicit wheel listener keeps the existing allocation-free
frame-boundary coalescing path. Opting into a listener delivers each native event so delta kind and
gesture boundaries are not lost, but repeated invalidations still collapse into one pending Winit
redraw. No listener, recognizer, timer, monitor, or idle frame exists after the element unmounts.

### Raw multi-contact touch

Raw contacts use the same web-style attachment model while retaining an independent identity for
each finger:

```rust
let touch = cx.touch_listener("canvas", |this, event, cx| {
    match event.phase {
        TouchPhase::Started | TouchPhase::Moved => {
            this.contacts.insert(event.id, event.position);
        }
        TouchPhase::Ended | TouchPhase::Cancelled => {
            this.contacts.remove(&event.id);
        }
    }
    cx.invalidate();
});

div().on_touch(touch)
```

QuickGUI hit-tests only `TouchPhase::Started`, then captures that contact to the nearest listening
element until `Ended` or `Cancelled`; moving outside the original bounds does not retarget it.
Callbacks bubble child-first through listening ancestors and can call `cx.stop_propagation()`.
Focus loss synthesizes one cancellation for every retained contact. A window captures at most 32
contacts, positions are finite bounded logical points, and optional force is normalized to
`0.0..=1.0`.

On macOS the existing Winit content view accepts AppKit direct and indirect `NSTouch` contacts and
forwards exact began, moved, ended, and cancelled sets. Resting touches are excluded, and AppKit
does not expose contact force through this path. The listener owns no recognizer, global monitor,
timer, polling source, or idle frame; invalidations still coalesce through the ordinary scheduler.

### Native pressure and gestures

One web-style element can compose typed Force Touch, pinch, rotation, and smart-magnify listeners:

```rust
let pressure = cx.mouse_pressure_listener("canvas", |this, event, cx| {
    this.pressure = event.pressure;
    cx.invalidate();
});
let pinch = cx.pinch_listener("canvas", |this, event, cx| {
    this.zoom = (this.zoom * (1.0 + event.delta).max(0.1)).clamp(0.5, 3.0);
    cx.invalidate();
});

div()
    .on_mouse_pressure(pressure)
    .on_pinch(pinch)
```

`MousePressureEvent`, `PinchEvent`, `RotationEvent`, and `SmartMagnifyEvent` include the current
logical pointer position and modifier state. Continuous gestures also include a
`Started`/`Moved`/`Ended`/`Cancelled` phase. Native non-finite input becomes zero, pressure is
clamped to `0.0..=1.0`, and per-event pinch and rotation deltas have public hard bounds.

macOS events travel through Winit's existing AppKit view and application event loop. QuickGUI
performs one topmost retained-tree lookup, lets a child target bubble to its nearest listening
ancestor, and respects overlay pointer blockers. The typed element callback runs before the same
event reaches `View::event`. No recognizer, monitor, timer, polling source, allocation-sized event
payload, or implicit animation loop is added. Repeated invalidations still collapse into Winit's
single pending redraw. Run `cargo run --release --example gesture_input` on a Mac with a trackpad.

## Keyboard layouts, text, and commands

Command identity is separate from text production. `Event::KeyDown.key` is the normalized key used
for shortcuts; `key_char` is the normalized printable character the same press could produce
before IME composition. Only `Event::TextInput` carries exact committed text. Caps Lock therefore changes inserted
text without silently changing `platform-a`, Option can produce `å` while the command key remains
`a`, and a dead key or input method remains authoritative for the eventual commit.

On macOS QuickGUI snapshots the active TIS keyboard layout and a fixed 128-key translation table.
The table includes plain, Shift, Command, Command-Shift, Option, and Option-Shift identities. It is
replaced only after AppKit posts
`NSTextInputContextKeyboardSelectionDidChangeNotification`; there is no input-source poll, retained
native layout object, or redraw for a view that does not observe the layout. This covers command
layouts such as Dvorak-QWERTY, non-ASCII layouts that switch to ASCII under Command, shifted
punctuation, and Option-produced binding characters without adding a native query in QuickGUI's
per-key matching path.

Views can read the cheaply cloned snapshot declaratively:

```rust
let layout_name = cx.keyboard_layout().name();
```

That render branch rebuilds when the input source changes. Event callbacks can read the same
snapshot through `EventContext::keyboard_layout`. Deterministic tests use
`simulate_keyboard_layout_change` and can attach native character metadata with
`Keystroke::with_key_char`. Run `cargo run --release --example actions_keymap` to inspect the live
layout plus normalized command/printable identities.

Bindings declared against a US keyboard can explicitly opt into Apple's automatic key-equivalent
localization:

```rust
KeyBinding::new("platform-[", NavigateBack, Some("Editor")).use_key_equivalents()
```

On a German layout that effective shortcut becomes Cmd-Ö, matching AppKit and SwiftUI rather than
requiring Option-Cmd-5. QuickGUI retains the original declaration, applies one finite static map at
the layout-notification boundary, and rebuilds the first-stroke index once. It performs no mapping
scan or native call while processing key events. Bindings remain literal by default, which is
important for user-configurable shortcuts that are already localized. Native menu items receive
the same effective shortcut with AppKit's additional automatic remapping disabled, so menu display
and focused key dispatch cannot diverge.

Applications that need to update non-view state can observe the same boundary directly:

```rust
App::new(view).on_keyboard_layout_change(|layout, cx| {
    tracing::info!(layout = layout.id(), "keyboard layout changed");
    cx.update_global::<ShortcutLabels, _>(|labels| labels.refresh(layout));
})
```

The callback runs after the new command map is installed. A view that only displays the layout
should continue to call `cx.keyboard_layout()` instead, retaining the narrower declarative
invalidation path.

Commands use typed actions and a keymap that is independent from rendering. Handlers live on the
focused element path, so editor commands naturally take precedence over pane and workspace
fallbacks:

```rust
quickgui::actions!(editor, [Save, SplitLeft]);

let editor_focus = cx.focus_handle("editor");
let capture_save = cx.action_listener("workspace", |this, _: &Save, _cx| {
    this.note_command_started();
});
let save = cx.action_listener("editor", |this, _: &Save, cx| {
    this.save();
    cx.invalidate();
});
let key = cx.key_down_listener("editor", |this, event, cx| {
    if event.key == Key::Escape {
        this.close_completion_menu();
        cx.prevent_default();
    }
});

div()
    .id("workspace")
    .capture_action(capture_save)
    .child(
        div()
            .track_focus(editor_focus)
            .key_context("Editor mode=insert")
            .on_action(save)
            .on_key_down(key)
    )
```

```rust
App::new(view).bind_keys([
    KeyBinding::new("platform-s", Save, Some("Editor")),
    KeyBinding::new("platform-k left", SplitLeft, Some("Workspace > Editor")),
]);
```

Typed actions use two ordered phases. `capture_action` runs root-to-focus and propagates by
default; `stop_propagation` consumes it before deeper capture or bubble handlers. `on_action` then
runs focus-to-root and consumes by default; call `cx.propagate()` to continue bubbling. Buttons,
command palettes, and native menus can call `cx.dispatch_action(Save)` and share both phases.

Raw `capture_key_down`/`on_key_down` and matching key-up listeners use the same frozen focused path.
Keymap actions run first. Only an action that propagates through every bubble handler falls through
to raw key listeners, which prevents a low-level handler from observing a shortcut already handled
as a command. Raw key events propagate by default. `stop_propagation` only ends the listener path;
`prevent_default` independently suppresses QuickGUI's direct editing, focus traversal, focused
activation, Escape dismissal, and default macOS close behavior. A separately delivered IME commit
remains authoritative. The coarse window-level `View::event` callback remains available after
targeted dispatch for application-wide observation.

Action and key callbacks are exact binding slots rather than per-event closures. Participating
elements allocate one small listener list; ordinary elements allocate none. One focused path is
bounded at 256 ancestors, with at most 32 action and 16 key declarations per element and 4,096 of
each per window. Reused window-owned vectors make dispatch allocation-free after warmup and no
listener owns a timer or frame request.

Context predicates support `!`, `&&`, `||`, `>`, `==`, and `!=`. Incomplete multi-stroke bindings
sleep until the next key or one timeout wake-up, then replay unmatched input. See
`cargo run --release --example actions_keymap`.

Reusable command palettes use `PickerState<T>`. The state owns bounded fuzzy matches, highlighted
UTF-8 ranges, disabled-aware selection, and a `VirtualList`; the application keeps command behavior
as ordinary typed values:

```rust
let palette = PickerState::new([
    PickerItem::new("Save File", AnyAction::new(Save)).shortcut("⌘S"),
    PickerItem::new("Toggle Sidebar", AnyAction::new(ToggleSidebar))
        .keywords("view explorer panel"),
])?;

App::new(view).bind_keys(picker_key_bindings());

// Inside View::render:
let colors = self.command_palette_colors();
let input = text_input(self.palette.query().clone())
    .placeholder("Search commands…")
    .h(48.0)
    .border(1.0, colors.border)
    .bg(colors.input);

let palette = self.palette.element(
    cx,
    "command-palette",
    "Command palette",
    |view| &mut view.palette,
    input,
    div().child("No matching commands"),
    |matched| {
        div()
            .bg(if matched.is_selected() { colors.selected } else { Color::TRANSPARENT })
            .child(matched.item().label().clone())
    },
    |view, action, cx| {
        view.palette_open = false;
        cx.focus(view.editor_focus);
        cx.dispatch_any_action(action);
        cx.invalidate();
    },
)
.w(560.0)
.rounded_xl()
.border(1.0, colors.border)
.bg(colors.surface);
```

`PickerLayout` retains only bounded row height and maximum visible-row geometry. The application
supplies the input, empty state, every visible row, root paint, text hierarchy, matched-range
highlighting, status copy, and interaction-state appearance. QuickGUI decorates those elements
with stable IDs, virtual positions, controlled input, semantics, and listeners without retaining a
theme or renderer closure.

The default contextual bindings cover Up/Down, Page Up/Down, Cmd/Ctrl-Up/Down, and Return without
stealing those keys from ordinary inputs. Escape uses the normal dismissible-overlay path and
restores focus. One picker accepts at most 65,536 items, 16 MiB of searchable metadata, 128 query
graphemes and 4 KiB of query text, and 2,048 ranked results; only the configured visible rows are
declared. Matching runs
only after `set_query` or `set_items`, uses a bounded top-result partition, and schedules no timer or
idle frame. Run `cargo run --release --example command_palette`; benchmark the 25,000-item query
path with `cargo bench --bench picker`.

Application menus use GPUI-shaped declarations and derive their key equivalents from the same
contextual keymap:

```rust
let menus = [
    Menu::new("File").items([
        MenuItem::action("Save", Save),
        MenuItem::separator(),
        MenuItem::submenu(Menu::new("Layout").items([
            MenuItem::action("Split Left", SplitLeft),
        ])),
    ]),
];

App::new(view).menus(menus);
```

On macOS these are real `NSMenu` trees with native keyboard navigation, nested submenus, checked
and disabled state, and focus-aware validation. `EventContext::set_menus` replaces labels or state
after an application mutation without introducing an idle update loop. `MenuItem::os_action`
first follows AppKit's responder chain for Cut/Copy/Paste/Select All/Undo/Redo, then falls back to
the same typed action and QuickGUI text input. This lets one Edit menu serve both GPU controls and
embedded `NSView` children. Opening a menu also cancels an incomplete multi-stroke prefix.

Overlays use a document-independent GPU plane, so they escape ancestor clipping and can be
anchored to any stable element ID:

```rust
overlay()
    .anchor_to(trigger.id(), AnchorPlacement::BottomEnd)
    .anchor_gap(8.0)
    .viewport_margin(12.0)
    .on_dismiss(dismiss)
    .restore_focus_to(trigger)
    .accessibility_role(AccessibilityRole::Menu)
    .child("Popover content")
```

Placement follows web popover behavior: use the preferred side when it fits, flip to the opposite
side when it has more room, try alternate alignment, then shift inside the viewport. A dismissible
overlay blocks pointer input behind it, dismisses on outside press or Escape, and restores its
trigger's focus.

Tooltips are ordinary bounded QuickGUI element trees owned by the framework:

```rust
button()
    .tooltip("Save the current file")
    .child("Save");

div().tooltip(
    Tooltip::new(custom_tooltip_content())
        .placement(AnchorPlacement::Right),
);
```

A tooltip waits for one exact 500 ms deadline by default, paints without rebuilding the application
view, stays visible for keyboard focus as well as pointer hover, and schedules nothing while
stationary. Each tooltip is capped at 256 elements and each window indexes at most 4,096 tooltip
declarations. Duration animations and springs inside custom content re-resolve only the detached
tree; hiding it drops their playback and deadline together. Text tooltips also become the trigger's
native accessibility description.

Web-style context menus use a targeted secondary-click listener and the same edge-aware placement
engine at the original logical pointer position:

```rust
let context = cx.context_menu_listener("row", |this, event, cx| {
    this.menu_position = Some(event.position);
    cx.invalidate();
});

div().id("row").on_context_menu(context);

overlay()
    .anchor_at(menu_position, AnchorPlacement::BottomStart)
    .on_dismiss(dismiss)
    .accessibility_role(AccessibilityRole::Menu)
    .child(menu_items)
```

Nested targets bubble to their nearest retained ancestor listener, while overlay blockers prevent
click-through. Dismissible menus retain outside-click/Escape dismissal and focus restoration. Run
`cargo run --example tooltips_context_menu` for the combined macOS example.

Typed drags and native files, text, and URLs use the same exact-type listener path:

```rust
let source = cx.drag_listener("source", |_this, _event, _cx| {
    Drag::new(Card { id: 7 }).preview(card_preview())
});
let cards = cx.drop_listener("target", |this, card: &Card, event, cx| {
    this.accept(card, event.position);
    cx.invalidate();
});
let files = cx.drop_listener("target", |this, files: &DroppedFiles, _event, cx| {
    this.open(files.paths());
    cx.invalidate();
});
let native_text = cx.drop_listener("target", |this, value: &DroppedText, _event, cx| {
    this.insert(value.as_str());
    cx.invalidate();
});
let native_url = cx.drop_listener("target", |this, value: &DroppedUrl, _event, cx| {
    this.open_url(value.as_str());
    cx.invalidate();
});

div().on_drag(source).dragging(|style| style.bg(dragging_color));
div()
    .on_drop(cards)
    .on_drop(files)
    .on_drop(native_text)
    .on_drop(native_url)
    .can_drop::<Card>(|card| !card.locked)
    .drag_over(|style| style.border(2.0, accent));
```

On macOS, the original arbitrary Rust value automatically remains available when a drag crosses
into another QuickGUI window. Add a public representation only when native applications should
also see existing files/directories, plain text, or an absolute URL:

```rust
use quickgui::{ExternalDragUrl, FileDragPaths};

let files = FileDragPaths::files([manifest_path]);
let project = ExternalDragUrl::new("https://github.com/egoist/quickgui")?;

let source = cx.drag_listener("source", move |_this, _event, _cx| {
    Drag::new(Card { id: 7 })
        .preview(card_preview())
        .external_files(files.clone())
});

let text_source = cx.drag_listener("text", move |_this, _event, _cx| {
    Drag::new(Card { id: 8 }).external_text("QuickGUI native text drag")
});

let url_source = cx.drag_listener("url", move |_this, _event, _cx| {
    Drag::new(Card { id: 9 }).external_url(project.clone())
});
```

A primary-button gesture crosses a two-point threshold before becoming a drag, which preserves
ordinary clicks. Only one payload and one preview tree are retained per window. Compatible targets
are selected by Rust `TypeId`; overlapping incompatible targets are skipped without violating
pointer blockers. Preview motion repaints the retained scene without rerunning `View::render` or
Flexbox, and stopping the pointer schedules no work. Escape, focus loss, or button release tears
the session down immediately.

Leaving a macOS window promotes the original `Arc<dyn Any>` into an application-wide registry
capped at 256 simultaneous sessions. AppKit receives only a random UUID under QuickGUI's private
pasteboard type; the Rust value is neither serialized nor copied. A destination resolves it into
the same exact-type listener path and reports `DragOrigin::CrossWindow { window, source }`. If a
writer also exposes text, URL, or file formats, selection is one topmost-first pass: the highest
compatible element wins before its preferred offered type, and a live-tree check still precedes
delivery. Snapshot storage is capped at 8,192 exact listener acceptances and 4,096 relevant target
or blocker regions, failing closed past either bound.

Finder drops are grouped into one `DroppedFiles` value, capped at
4,096 paths, and their enter notification is coalesced at the event-loop boundary. An outbound
drag stays on the internal GPU path until the pointer exits the viewport, then starts one AppKit
session and reports its result through `Event::ExternalDragEnded`. Files become `NSURL`
pasteboard writers; the application supplies each path's directory bit, avoiding synchronous
filesystem metadata, while construction caps the payload at 4,096 paths, 16 KiB per encoded path,
and 8 MiB total path storage. Plain text becomes an `NSString`, retains at most 1 MiB, and truncates
only at a UTF-8 boundary. URLs become `NSURL` writers, reject missing/invalid absolute schemes,
whitespace, controls, and values above 16 KiB before promotion. The source advertises copy only, so
QuickGUI never implicitly relocates or mutates application-owned data. Inbound text and URLs use
the aliases `DroppedText` and `DroppedUrl`, so the same exact type can make a round trip through a
native app. AppKit registers those pasteboard formats only while matching listeners exist. Its
synchronous cursor decision uses a topmost-first snapshot capped at 4,096 relevant targets and
pointer blockers; `can_drop` gates both the native cursor and eventual live-tree delivery. Hover
motion is coalesced to one pending event-loop wake, and UTF-8 extraction retains at most 1 MiB of
text or rejects URLs above 16 KiB. Winit continues to own and receive every Finder file selector.
There is no polling, animation loop, or deadline while the pointer is stationary. See
`cargo run --release --example drag_drop` for public formats and
`cargo run --release --example multi_window` for arbitrary cross-window values.
