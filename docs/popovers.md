# Popover and SystemPopover

[Documentation index](README.md)

For cursor-point secondary-click composition, see [Context menus](context-menus.md).

`Popover` follows QuickGUI's [unstyled component contract](component-roadmap.md): the framework
owns positioning, focus, dismissal, portal, and accessibility behavior, while the application owns
every color, dimension, border, radius, shadow, icon, type style, and transition.

QuickGUI exposes two deliberately different hosts:

- `Popover` is a lightweight controlled overlay inside the current window. It can escape ancestor
  clips, but it cannot paint outside that window's WGPU surface.
- `SystemPopover` opens a parent-owned borderless native child window with its own WGPU surface.
  On macOS it can cross the parent edge and is constrained against the display work area instead.

There is no silent downgrade from the overflow-capable host to a parent-clamped overlay. Popover
menus, context menus, selects, and other dropdown components use the `SystemPopover` host on
macOS. Use the in-window host only when staying inside the current window is the desired behavior.

The application owns an in-window popover's open boolean. The framework owns geometry, topmost
light dismissal, pointer blocking, focus restoration, and accessibility projection:

```rust
use quickgui::{Popover, div, text};

let popover = Popover::new("account-trigger", "account-popover", self.account_open);
let toggle = cx.listener(popover.trigger_id(), move |view, cx| {
    view.account_open = !view.account_open;
    if view.account_open {
        popover.focus_surface(cx);
    } else {
        popover.focus_trigger(cx);
    }
    cx.invalidate();
});
let dismiss = cx.dismiss_listener(popover.surface_id(), |view, cx| {
    view.account_open = false;
    cx.invalidate();
});

let mut root = div().child(
    popover
        .trigger_part(div())
        .on_click(toggle)
        .accessibility_label("Account options")
        .child("Account"),
);

if popover.is_open() {
    let popover = popover
        .popover_part(
            div()
                .w(280.0)
                .p_3()
                .rounded_lg()
                .bg(app_colors.popover),
        )
        .on_dismiss(dismiss)
        .children([
            popover.title_part(text("Account options")),
            popover.description_part(text("Choose an account action.")),
        ]);
    root = root.child(
        popover
            .positioner_part(div().child(popover)),
    );
}
```

Mount the positioner/popover only while the controlled value is open. This removes closed content
from layout, hit testing, focus traversal, accessibility, and retained overlay state instead of
hiding a second framework-owned copy.

## Trigger, positioner, and popover contract

`trigger_part(element)` decorates a caller-owned root with stable button semantics, focus,
hidden-inset drag exclusion, desktop-arrow cursor behavior, controlled `expanded` state, and the
exact `has-popup` kind. While content is mounted it also exposes a native `controls` relationship
to the popover; a closed trigger never publishes a dangling AccessKit node reference. `trigger()` is
an unstyled shorthand that chooses `button()` as the root.

`positioner_part(element)` combines the portal and positioner boundary. A retained QuickGUI
overlay is already detached from ancestor clipping, so a separate full-window portal wrapper would
only create an incorrect pointer blocker. The positioner owns only structural geometry:

- bottom-start placement with flip-before-shift viewport fitting;
- a six-point trigger gap and eight-point viewport collision margin by default;
- stable positioner identity and exact retained trigger anchoring; and
- no colors, size, padding, typography, shadow, transition, or popover semantics.

`popover_part(element)` decorates the application-presented content root with:

- exact stable popover ID and role;
- Escape and outside-primary-press dismissal;
- pointer blocking inside the surface and no click-through on the dismissing press;
- focus restoration to the paired trigger;
- explicit `app-region: no-drag` behavior inside hidden-inset titlebars;
- a focusable content root for same-turn opening focus; and
- mounted title/description relationships without copying visible text.

The popover emits `Event::Dismiss(surface_id)` even without a typed listener. Use
`cx.dismiss_listener(...)` for the usual local callback or handle that event in `View::event`.
`dismiss_on_escape(false)` and `dismiss_on_pointer_outside(false)` configure those paths
independently while the popover continues to block click-through.

Opening focus is explicit because applications may prefer the surface root, a search field, or a
specific menu item. Declare `.initial_focus(id)` when appropriate; `focus_surface(cx)` targets it
and otherwise targets the popover root through QuickGUI's existing one-rebuild deferred focus
request. Calling it in the same listener that mounts the popover needs no next-frame task. Escape and
outside dismissal restore trigger focus automatically; call `focus_trigger(cx)` when a content
action closes the controlled popover directly.

`title_part`, `description_part`, and `close_part` decorate application-owned visible parts with
stable relationships and behavior but no presentation. `backdrop_part` supplies an optional
full-viewport, accessibility-hidden pointer layer for a caller-painted backdrop. For compact
composition, `surface_part(element)` and `surface()` merge positioner and popover behavior onto one
unstyled root. Separate parts are preferable when the application sizes or animates the
positioner independently.

## SystemPopover

`SystemPopover` takes an explicit child-surface size and resolves the trigger's latest retained
bounds by stable element ID at the event boundary:

```rust
use quickgui::SystemPopover;

let open = cx.listener("actions-trigger", |view, cx| {
    if let Some(previous) = view.actions.take() {
        cx.close_window_handle(previous);
        cx.invalidate();
        return;
    }
    view.actions = Some(
        SystemPopover::new(244.0, 178.0)
            .gap(6.0)
            .open(cx, "actions-trigger", "Actions", ActionsMenu)
            .expect("the trigger is mounted in this window"),
    );
});

let trigger = quickgui::button()
    .on_click(open)
    .child("Actions");
```

Retain the returned handle for replacement or programmatic close, and synchronize it through the
parent view's declarative lifecycle callback:

```rust
cx.on_any_child_window_closed(|view, closed, cx| {
    if view.actions == Some(closed) {
        view.actions = None;
        cx.invalidate();
    }
});
```

This listener can be declared before `actions` exists, so even an open-and-close sequence in one
effect turn cannot leave stale controlled state. Direct-child callbacks are bounded per window and
reuse runtime ownership teardown; they install no native observer or scheduling source.

The trigger rectangle is not copied into application state and no layout observer is installed.
The child view owns its presentation, so the host is fully unstyled. `placement`, `gap`, `offset`,
`constraint_adjustment`, and `grab` configure structural behavior. The default `FIT` constraints
flip and slide without shrinking; resize constraints remain opt-in through
`PopoverConstraintAdjustment`.

On macOS this host is a transparent nonactivating `NSPanel` at popover-menu level, attached to its
parent with `addChildWindow`. Placement is resolved once before creation and again before the first
visible frame, using the `NSScreen.visibleFrame` that contains the anchor. A menu-style grab closes
on Escape or outside press; passive `grab(false)` surfaces open without taking key focus and
install no event monitor. When the outside press lands on the active anchor itself, AppKit consumes
that press after requesting dismissal, so a controlled trigger closes instead of receiving a later
click that reopens the popover. An interactive passive child may subsequently receive AppKit key focus.
When it belongs to a grabbing popover chain, attached descendants count as inside for both focus and
outside-click dismissal, so moving focus between menu levels does not close their root. The popover
may therefore extend beyond the parent while still avoiding the menu bar, Dock, and physical
display edge.

Popover size is explicit in this first contract, matching the underlying platform popover API. A
future intrinsic measure-first convenience must preserve the same hidden-first-frame and bounded
resource behavior rather than briefly showing a guessed rectangle.

## Unstyled popover menus

`PopoverMenu` is the retained behavior model that composes with `SystemPopover`; it is deliberately
separate from the native application-menu type named `Menu`. Items retain stable IDs and typed
actions, while `element` and `element_with_submenus` decorate caller-owned roots and rows with menu
roles, active-descendant focus, pointer behavior, keyboard bindings, and typeahead.
`element_with_submenus_and_hover` lets an overflow host supply delayed hover/menu-aim policy without
changing the model or row presentation. These APIs add no background, border, padding, typography,
indicator, icon, or animation.

```rust
use quickgui::{PopoverMenu, PopoverMenuItem};

let menu = PopoverMenu::new([
    PopoverMenuItem::group_label("File"),
    PopoverMenuItem::action("open", "Open…", OpenFile).shortcut("⌘O"),
    PopoverMenuItem::separator(),
    PopoverMenuItem::checkbox_with("sidebar", "Show sidebar", sidebar, SetSidebar),
])?;
```

`item_part` assigns exact `menuitem`, checkable/radio item, label, and separator semantics while
remaining visually inert. AccessKit names its platform-neutral separator role `Splitter`; without
value-changing actions QuickGUI projects it as the native non-interactive divider. For a
caller-composed group, keep the visible label mounted and relate it without copying text:

```rust
let label = menu.item_part("file-menu", 0, render_group_label())?;
let open = menu.item_part("file-menu", 1, render_open_row())?;
let group = menu.labeled_group_part(
    "file-menu",
    0,
    div().flex_col().children([label, open]),
)?;
```

`labeled_group_part` adds only the native group role and a mounted `labelled-by` relationship. The
generic `Element::accessibility_labelled_by` primitive omits dangling and self-referential targets.
Its compact relation table adds this third relationship without increasing the storage previously
used by `controls` and `active-descendant`; none of these relationships install an observer or
idle task.

Up/Down, Home/End, Enter/Space, Left/Right, Escape, pointer hover, and alphanumeric text navigation
share the ordinary key/action/listener system. Focus loops by default. Checkbox and radio items
remain open by default; ordinary command items close by default. A caller can override either
policy. Submenu activation returns another already-validated `PopoverMenu` plus the exact derived row
anchor, so the caller opens a right-start `SystemPopover` without copying geometry.

Commands from a popover or any nested submenu preserve their original concrete `AnyAction` payload
and dispatch to the nearest non-popover owner window's focused action path. No callback registry or
serialization bridge is involved. `cx.close_popover_chain()` closes the first popover and lets normal
parent ownership tear down all descendants child-first after a command.

One menu tree retains at most 2,048 entries, eight levels, 16 KiB per label/shortcut/search label,
4 MiB total text, and a 256-byte typeahead prefix. Typeahead expiry is checked only on the next key;
there is no timer, task, observer, polling pass, or idle redraw. The cursor-point context adapter
uses one cancellable exact timer per menu level only while hover intent or a diagonal safe corridor
is pending, then returns to sleep. The styled `system_popover` and `tooltips_context_menu` examples
are executable composition references.

## Roles, placement, and nesting

`PopoverKind::{Dialog, Menu, ListBox, Tree, Grid}` maps both the surface role and trigger
`has-popup` value. Child roles and behavior remain application-owned; selecting `Menu` does not
silently install a second menu-item registry. Existing typed actions, keymaps, picker state, or
custom controls can be composed inside the surface.

Use `.placement(...)` to choose a preferred side/alignment. `.anchor_gap(...)` and
`.viewport_margin(...)` configure finite, bounded positioner geometry. Minimum width, padding,
border, radius, shadow, typography, and all other presentation belong on the caller's popover
element.

Nested in-window popovers use the same API. Because dismissal regions are ordered by actual overlay
paint order, Escape or an outside press dismisses only the topmost nested surface first. Nested
native popover panels use their bounded platform grab stack; menu commands can explicitly close the
complete chain with `close_popover_chain`. No second component-owned popover registry is retained.

## Resource behavior

A `Popover` descriptor is `Copy` and contains two declared IDs, controlled state, semantic kind,
bounded structural placement, dismissal policy, and optional initial focus. Derived part IDs are
computed without allocation. It owns no `Rc`, appearance token, component store, observer, task,
timer, animation loop, or native window. Closed content is unmounted; an open, settled popover adds
no deadline or idle frame. A mounted popover uses the existing one fixed relation table for its
optional title/description targets; native relationships are projected only when an accessibility
update is requested.

`SystemPopover` is also a copied descriptor and adds no observer, task, timer, or idle deadline.
While open, its child owns one native window and one WGPU surface but shares the application's GPU
device, queue, fonts, assets, and bounded caches. A passive popover adds no native mouse monitor;
menu-style popovers share the runtime's single bounded monitor pair.

Run the caller-styled edge placement, nesting, focus, menu-role, and parts gallery with:

```console
cargo run --release --example popovers
cargo run --release --example system_popover
```
