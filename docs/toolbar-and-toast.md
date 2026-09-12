# Toolbar, toggles, and toasts

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI provides an unstyled toolbar with roving focus, pressed-state toggle buttons and toggle
groups, and a bounded in-window toast queue with live-region announcements. The application owns
every icon, label, color, radius, placement, and animation; QuickGUI owns roles, relationships,
keyboard navigation, and exact dismissal timing.

The behavior follows the current [Base UI](https://base-ui.com/react/overview/quick-start)
contracts and the WAI-ARIA [Toolbar](https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/) and
[Button (toggle)](https://www.w3.org/WAI/ARIA/apg/patterns/button/) patterns.

Run the caller-styled gallery with:

```console
cargo run --release --example toolbar_toast
```

## Toolbar

A toolbar takes the caller's ordered item list. QuickGUI keeps no item registry: navigation walks
that slice, so the application's own data is the single source of order and disabled state.

```rust
use quickgui::{Toolbar, ToolbarItem, ToolbarState, div};

let items = [
    ToolbarItem::new("undo"),
    ToolbarItem::new("redo").disabled(true),
    ToolbarItem::new("share"),
];
let toolbar = Toolbar::new("commands", &self.toolbar, &items);
let mut root = toolbar
    .root()
    .accessibility_label("Document commands");
for item in items {
    let entry = toolbar.item(item.value()).expect("declared item");
    root = root.child(entry.key_with(
        cx,
        entry.item_with(div().child("Undo").on_click(clicked)),
        |view: &mut Editor| &mut view.toolbar,
    ));
}
```

Exactly one enabled item is in the window's normal Tab sequence: the controlled active value while
it names an enabled item, and otherwise the first enabled item. `ToolbarEntry::is_roving_stop`
reports which one, so an application can style it.

Each item answers its own arrow keys, so the focused item is always the one that moves. Install
`toolbar_key_bindings()` once. A horizontal toolbar answers Left and Right; a vertical toolbar
answers Up and Down. The other axis is left to the surrounding application, matching the tab-list
contract. Home and End move to the first and last enabled item, disabled items are skipped, and
`loop_focus(false)` stops at the edges instead of wrapping.

Clicking an item moves platform focus to it; call `ToolbarState::focus(value)` from the item's own
click listener so the roving stop follows the pointer.

The root projects the Toolbar role with its orientation. Give it an `.accessibility_label(...)` or
`.accessibility_labelled_by(...)`; a toolbar without a name is indistinguishable from any other
group.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_TOOLBAR_ITEMS` | 256 | Items retained in one toolbar declaration. |

Item values must be unique inside one toolbar.

### Parts

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Root | `Toolbar::root_with(element)` | the Toolbar role, orientation, and the single roving Tab stop |
| Button | `ToolbarEntry::button_with(element)` | the item contract plus the Button role |
| Link | `ToolbarEntry::link_with(element)` | the item contract plus the Link role |
| Input | `ToolbarEntry::input_with(element)` | the item contract, keeping whatever role the control already declares |
| Group | `Toolbar::group_with(element)` | a labelling and structural unit that creates no second focus scope |
| Separator | `Toolbar::separator_with(element)` | the Separator role across the toolbar's cross axis, never focusable |

`ToolbarItem::focusable_when_disabled(bool)` is Base UI's `focusableWhenDisabled`, and a toolbar
defaults it to `true`: skipping an unavailable item entirely hides it from keyboard users, who then
cannot discover that the command exists. The item still reports as disabled and still refuses
pointer focus; only keyboard reachability changes. `Element::focusable_when_disabled()` is the
underlying primitive.

## Toggle and toggle group

`Toggle` is a button that stays pressed. It is deliberately **not** a checkbox: assistive technology
reads it as a button with a pressed state, so it belongs to commands such as *Bold* rather than to
form data. Use [`Checkbox`](selection-controls.md) when the value is submitted with a form.

```rust
let bold = Toggle::new(self.bold);
bold.root().child("Bold").on_click(clicked).id("bold")
```

`ToggleGroup` composes those buttons with one roving Tab stop and a controlled pressed set.
`ToggleGroupState::single()` keeps at most one pressed item and releases it when it is pressed
again; `ToggleGroupState::multiple()` allows any number. `press`, `release`, `toggle`, and `clear`
all report whether the pressed set changed.

The group's Tab stop is the controlled active value, then the first pressed enabled item, then the
first enabled item. Install `toggle_group_key_bindings()` for arrow, Home, and End navigation; the
axis rules match the toolbar.

The group root projects a group role with its orientation, and each item projects a button with
exact pressed state. Label the group and each item; QuickGUI adds no text.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_TOGGLE_GROUP_ITEMS` | 32 | Items retained in one group, and pressed values retained inline. |

A multiple-selection group at the bound rejects further pressed values rather than growing.

## Toasts

`ToastManager` is a bounded queue. `push` returns a stable `ToastId`; reaching `MAX_TOASTS` drops
the oldest toast rather than growing the queue.

```rust
let id = self.toasts.push(
    Toast::new("Item deleted")
        .description("The item moved to the trash.")
        .action("Undo")
        .kind(ToastKind::Warning),
    Instant::now(),
);
```

`ToastViewport` decorates the caller's viewport, and `ToastViewport::toasts(&manager)` walks the
queue newest first, handing each toast its own descriptor. The root is labelled by its title and
described by its description when one is present.

### Parts

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Provider | `ToastManager` itself | the shared `timeout`, `limit`, swipe direction and threshold |
| Portal | `ToastViewport::portal_with(element)` | a window-level overlay that deliberately does not block pointer input |
| Viewport | `ToastViewport::viewport_with(element)` | the group the stack lives in |
| Positioner | `ToastParts::positioner_with(element)` | a stable identity for the wrapper that places one toast |
| Root | `ToastParts::root_with(element)` | the live region, the role, focus, and the title/description relationships |
| Content | `ToastParts::content_with(element)` | a stable identity for the inner content wrapper |
| Title / Description | `title_with` / `description_with` | the targets the root points at |
| Action / Close | `action_with` / `close_with` | button semantics with no visual defaults |

`ToastViewport::toast(entry)` still returns the parts for a single entry; it defaults to the top of
the stack because it has no queue to count against.

### The provider, the limit, and the stack

`ToastManager` is the provider. `timeout(...)` is the auto-dismiss duration a queued toast inherits
when it declares none, and `limit(n)` — three by default — leaves the newest `n` toasts ordinary and
flags every older one `ToastEntry::is_limited`. Limiting is presentation only: a limited toast is
still queued, still announces, and still counts down, so an application collapses it behind the
stack rather than hiding a message that will never come back.

`set_expanded(bool)` is Base UI's expanded stack, normally driven from the viewport's own hover or
focus. Each descriptor carries `index()` (Base UI's `data-index`, counting from the newest),
`is_limited()`, and `is_expanded()`. `offset(pitch)` turns the index into a stacking offset: Base UI
derives its own offset from measured heights, and QuickGUI never measures on the application's
behalf, so the caller passes the pitch it wants — a small peek while collapsed, a full row height
plus gap while expanded.

### The manager API

`add` and `push` queue a toast; `close(id)` and `dismiss(id)` remove one; `close_all()` and
`clear()` empty the queue. Each pair is the same operation under Base UI's name and QuickGUI's
original name, and both stay supported.

`update(id, toast, now)` replaces a queued toast's content and re-arms its countdown while keeping
its identity, its position in the stack, and any swipe in flight — so a message can change without
the toast jumping or re-announcing as new. `promise(loading, now)` queues a persistent
`ToastKind::Loading` toast and `resolve(id, toast, now)` turns it into its success or error result.
QuickGUI owns no future: the application drives both halves from the foreground task it already
spawned, and neither call schedules a timer.

### Swipe to dismiss

`swipe_direction(...)` and `swipe_threshold(...)` declare Base UI's swipe-to-dismiss, and
`apply_swipe(id, event)` takes the captured pointer events from a toast root. Motion is projected
onto the declared direction and clamped at zero, so a toast can never be dragged the wrong way, and
`ToastEntry::swipe_movement` exposes the distance for the application to translate the toast by.
Releasing past the threshold dismisses the toast and reports `dismissed`; releasing short resets the
movement to zero and leaves it queued. The gesture is pure pointer capture and schedules nothing.

### Announcements

`ToastKind::Info` and `Success` project a polite Status live region; `Warning` and `Error` project
an assertive Alert region that interrupts the current utterance. The viewport itself is an ordinary
group, so an unchanged queue announces nothing — only a newly mounted toast is announced, exactly
once.

### Dismissal and timing

The manager owns no timer. It reports the single next deadline:

```rust
let Some(deadline) = self.toasts.next_deadline() else { return }; // None when idle
// sleep exactly once with AsyncViewContext::sleep_until(deadline), then:
self.toasts.expire(now);
```

An empty or fully paused queue reports no deadline at all, so a settled window keeps zero idle
sources. `pause(id, now)` and `resume(id, now)` implement pause-on-hover and pause-on-focus by
converting the remaining time into a fresh deadline; a paused toast never expires. `Toast::persistent`
opts one toast out of auto-dismissal entirely.

`ToastParts::key_with` attaches focused Escape dismissal. Escape is handled only while focus is
inside that toast, so it never competes with a dialog, a popover, or the application's own Escape
handling.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_TOASTS` | 8 | Toasts retained in one queue. |
| `MAX_TOAST_TEXT_BYTES` | 512 | UTF-8 bytes retained by one title, description, or action label. |
| `MAX_TOAST_DURATION` | 60 s | Longest auto-dismiss duration retained by one toast. |
| `MAX_TOAST_SWIPE_THRESHOLD` | 512 | Longest swipe distance one toast may require, in logical pixels. |

Longer text is truncated on a character boundary instead of retained.

## Go components

`Toggle.Root` / `Indicator` declares the controlled pressed state and adopts the core's toggle-button
role.

`Toolbar.Root` / `Item` and `ToggleGroup.Root` / `Item` declare the ordered navigation model as one
bounded `Items` list plus the controlled active or pressed values. The hosted view reaches each
declared instance's retained `ToolbarState` or `ToggleGroupState` through a per-instance
[`StateAccessor`](view-api.md), so several toolbars and groups in one window keep their own roving
Tab stop. Wrapping arrow navigation, Home/End, disabled-item skipping, and single-versus-multiple
selection policy stay in the core; the moved stop and the pressed set travel back as one
asynchronous `OnActiveChange` or `OnValueChange` payload. Duplicate declared values keep the first
occurrence and overflow past the core's own item bound is dropped, so a declaration can never panic
the core.

`Toast.Viewport` / `Root` / `Title` / `Description` / `Action` / `Close` declares the queue itself:
pushing a toast is adding an entry with a new identifier to the bounded `Toasts` list, and dropping
one dismisses it. The hosted view reaches each declared viewport's retained `ToastManager` through a
per-instance [`StateAccessor`](view-api.md). The core owns the queue bound, live-region politeness,
title and description relationships, focused Escape dismissal, and the exact auto-dismiss deadline,
which the binding sleeps on with one `request_repaint_at` rather than a timer of its own. Every
dismissal the core decided — including the timed ones — travels back as one asynchronous
`OnDismiss` payload naming the caller's own declared identifiers. See
[Go components](go.md).

## Menubar

The in-window menubar has its own guide: see [menubar](menubar.md) for the `MenubarState` model and
its Go components. Native macOS menus remain the application menu; see
[popovers and popover menus](popovers.md) for the in-window `PopoverMenu` model.

## Resource contract

`ToolbarState`, `ToggleGroupState`, and every descriptor in this page are plain values.
`ToastManager` retains only its bounded queue of shared strings. None of them retains an item
registry, task, timer, observer, animation, GPU resource, or idle scheduler source. Navigation scans
the caller's own item slice on an explicit keypress and at no other time.

## Additional Go parts

`Toolbar` gained `Button`, `Link`, `Input`, `Group`, and `Separator`. All three item parts share the
one roving Tab stop and differ only in the role the core projects; the group and separator are
structural and take the toolbar's own axis. The core makes a disabled toolbar item discoverable by default: it keeps its place in the Tab sequence while disabled, so a keyboard user can still
discover the command; arrow navigation still skips it and it still refuses pointer focus.

```go
func ActionToolbar() *native.Node {
	toolbar1 := ui.NewToolbar(ui.ToolbarRootProps{Items: []ui.ComponentItem{{Value: "cut"}, {Value: "docs"}, {Value: "paste", Disabled: true}}})
	return toolbar1.Root().Children(func() *native.Node {
		return ui.Fragment([]*native.Node{toolbar1.Group(ui.PartProps{}).Children(func() *native.Node {
			return ui.Fragment([]*native.Node{toolbar1.Button(ui.ToolbarItemProps{Value: "cut"}).Children("Cut").NativeNode(), toolbar1.Link(ui.ToolbarItemProps{Value: "docs"}).Children("Docs").NativeNode()})
		}).NativeNode(), toolbar1.Separator(ui.PartProps{}).NativeNode(), toolbar1.Item(ui.ToolbarItemProps{Value: "paste"}).Children("Paste").NativeNode()})
	}).NativeNode()
}
```

`Toast` gained `Provider`, `Portal`, `Positioner`, and `Content` parts and a Base UI-shaped manager.
The provider owns the declared queue and the provider props — the inherited `Timeout`, the visible
stack `Limit`, the `Expanded` stack, the `SwipeDirection`, and the stack `Pitch` the core turns into
each toast's own offset:

```go
func Notices() *native.Node {
	toast1 := ui.NewToast(ui.ToastProviderProps{Timeout: 4000, Limit: 3})
	return toast1.Provider().Children(func() *native.Node {
		var children []*native.Node
		toasts := ui.UseToastManager()
		children = append(children, ui.Button().Child("Notify").OnClick(func() {
			toasts.Add(ui.ToastRequest{Title: "Saved", Type: ui.ToastSuccess})
		}).Node)
		children = append(children, toast1.Viewport(ui.ToastViewportProps{}).Children(func() *native.Node {
			return ui.KeyedFor(toasts.Stack, func(entry ui.ToastStackEntry) any {
				return entry.ID
			}, func(entry func() ui.ToastStackEntry, _ func() int) *native.Node {
				id := entry().ID
				return toast1.Positioner(ui.ToastPartProps{ToastID: id, PartProps: ui.PartProps{Style: ui.Style().Top(func() float64 {
					return entry().Offset
				})}}).Children(func() *native.Node {
					return toast1.Root(ui.ToastPartProps{ToastID: id}).Children(func() *native.Node {
						return toast1.Content(ui.ToastPartProps{ToastID: id}).Children(func() *native.Node {
							return ui.Fragment([]*native.Node{toast1.Title(ui.ToastPartProps{ToastID: id}).Children(func() string {
								for _, toast := range toasts.Toasts() {
									if toast.ID == id {
										return toast.Title
									}
								}
								return ""
							}).NativeNode(), toast1.Close(ui.ToastPartProps{ToastID: id}).Children("Dismiss").NativeNode()})
						}).NativeNode()
					}).NativeNode()
				}).NativeNode()
			}, nil)
		}).NativeNode())
		return ui.Fragment(children)
	}).NativeNode()
}
```

`Add`, `Update`, `Close`, and `CloseAll` change the declaration. For background work,
add a `ToastLoading` toast with a zero duration, then update that ID from the task's
completion callback. This preserves its identity and stack position; the application
owns the task and decides how errors are presented. The stack index, the `Limited` and `Expanded` flags, each toast's `Offset`, and the live
swipe displacement all come back from the core through `Stack()`. See
[Go components](go.md).
