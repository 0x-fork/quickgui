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
    .root_part(div())
    .accessibility_label("Document commands");
for item in items {
    let entry = toolbar.item(item.value()).expect("declared item");
    root = root.child(entry.key_part(
        cx,
        entry.item_part(div().child("Undo").on_click(clicked)),
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

## Toggle and toggle group

`Toggle` is a button that stays pressed. It is deliberately **not** a checkbox: assistive technology
reads it as a button with a pressed state, so it belongs to commands such as *Bold* rather than to
form data. Use [`Checkbox`](selection-controls.md) when the value is submitted with a form.

```rust
let bold = Toggle::new(self.bold);
bold.root_part(div().child("Bold").on_click(clicked)).id("bold")
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

`ToastViewport` decorates the caller's viewport, and `ToastViewport::toast(entry)` returns the parts
for one queued toast: `root_part`, `title_part`, `description_part`, `action_part`, and
`close_part`. The root is labelled by its title and described by its description when one is
present.

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

`ToastParts::key_part` attaches focused Escape dismissal. Escape is handled only while focus is
inside that toast, so it never competes with a dialog, a popover, or the application's own Escape
handling.

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_TOASTS` | 8 | Toasts retained in one queue. |
| `MAX_TOAST_TEXT_BYTES` | 512 | UTF-8 bytes retained by one title, description, or action label. |
| `MAX_TOAST_DURATION` | 60 s | Longest auto-dismiss duration retained by one toast. |

Longer text is truncated on a character boundary instead of retained.

## JavaScript bindings

`Toggle.Root` / `Indicator` declares the controlled pressed state and adopts the core's toggle-button
role. `ToggleGroup`, `Toolbar`, and `Toast` are not bound yet: their roving focus and toast lifetime
run through retained state reached with a non-capturing `fn(&mut V) -> &mut State` accessor, which
one hosted view cannot provide per declaring node. See
[Solid 2 renderer](solid.md#range-and-feedback-parts).

## Menubar

An in-window menubar is not implemented. Native macOS menus remain the application menu; see
[popovers and popover menus](popovers.md) for the in-window `PopoverMenu` model and
[the roadmap](component-roadmap.md) for the current status.

## Resource contract

`ToolbarState`, `ToggleGroupState`, and every descriptor in this page are plain values.
`ToastManager` retains only its bounded queue of shared strings. None of them retains an item
registry, task, timer, observer, animation, GPU resource, or idle scheduler source. Navigation scans
the caller's own item slice on an explicit keypress and at no other time.
