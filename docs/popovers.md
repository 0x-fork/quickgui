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

### Parts

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Root | `Popover::new(trigger_id, popup_id, open)` | copied descriptor, derived part identities, controlled open state |
| Trigger | `trigger_part(element)` / `trigger()` | button semantics, focus, `expanded`, `has-popup`, `controls` while open |
| Portal | `portal_part(element)` | the same retained overlay node as the positioner |
| Positioner | `positioner_part(element)`, `tracked_positioner_part(element, &handle)` | anchoring, flip/shift, side and align offsets, collision padding, sticky policy, placement reporting |
| Popup | `popover_part(element)` | role, dismissal, pointer blocking, focus restoration, modal focus containment, title/description relationships |
| Arrow | `arrow_part(element)` | absolute placement on the resolved edge and cross-axis centering on the anchor |
| Backdrop | `backdrop_part(element)` | full-viewport, accessibility-hidden pointer layer |
| Viewport | `viewport_part(element)` | stable identity and a scroll container inside the popup |
| Title | `title_part(element)` | the label target the popup points at |
| Description | `description_part(element)` | the description target the popup points at |
| Close | `close_part(label, element)` | button semantics and an accessible name |

`surface_part(element)` and `surface()` merge positioner and popover behavior onto one unstyled
root, and `tracked_surface_part(element, &handle)` does the same while publishing placement.
Separate parts are preferable when the application sizes or animates the positioner independently.

### Positioner props

`side(AnchorSide)` and `align(AnchorAlign)` are the two halves of `placement(...)`; all three keep
working. `side_offset(...)` and `collision_padding(...)` are the Base UI-named aliases of
`anchor_gap(...)` and `viewport_margin(...)` — both names write the same bounded value.
`align_offset(...)` shifts the surface along its cross axis before collision handling, so it can
slide a popup along its trigger but never push it off screen. `sticky(false)` opts out of
QuickGUI's default clamping so a popup pinned to a scrolling row leaves the viewport with it.
`anchor_element(id)` and `anchor_point(point)` position against something other than the trigger
without changing the trigger's own relationships; `anchor_trigger()` restores the default.
`modal(true)` contains Tab focus inside the popup and expects a mounted backdrop; QuickGUI adds no
dimming.

Every distance is clamped by `MAX_POPOVER_SIDE_OFFSET`, `MAX_POPOVER_ALIGN_OFFSET`,
`MAX_POPOVER_COLLISION_PADDING`, and `MAX_POPOVER_ARROW_SIZE`. A non-finite value falls back to the
declared default rather than to a bound.

### Resolved placement and the flip-aware arrow

A declared placement is a preference. QuickGUI flips the side and re-aligns the cross axis whenever
the preference does not fit, so an arrow drawn from the preference would point at nothing near a
window edge. Store an `AnchorPlacementHandle` next to the open flag, mount the positioner with
`tracked_positioner_part`, and read it back with `track_placement`:

```rust,ignore
let popover = Popover::new("trigger", "popup", self.open)
    .side(AnchorSide::Top)
    .align(AnchorAlign::Center)
    .side_offset(12.0)
    .arrow_size(12.0)
    .arrow_padding(10.0)
    .track_placement(&self.placement);
let state = popover.state();
```

The handle receives the resolved placement, the anchor rectangle, the placed popup rectangle, the
room left on the resolved side, and whether the anchor scrolled out of view. It is written during
the paint QuickGUI was already performing; when the resolved value changes, exactly one correcting
frame is requested, and an unchanged placement requests none, so a settled popover stays settled.
`Element::report_anchor_placement(handle)` binds the same reporting to any anchored element.

`LayoutBoundsHandle` is the same contract for painted geometry: `Element::report_bounds(handle)`
publishes an element's window-relative bounds during the paint already being performed, and a
changed size earns exactly one correcting frame. Behavior that must match real layout — a splitter
whose total is whatever flex gave its panes, a scroll area sized from its painted viewport and
content — reads `LayoutBoundsHandle::bounds` while declaring the next frame instead of guessing.

`arrow_part(element)` then pins the caller-owned arrow to the popup edge that faces the anchor and
centers it on the anchor along the cross axis, clamped by `arrow_padding`. Size, shape, rotation,
and color stay application-owned, and the arrow is hidden from assistive technology.

`Popover::state()` returns a copyable `PopoverPartState` carrying what Base UI exposes as `data-*`
attributes and CSS variables: `open`, `modal`, `side`, `align`, `anchor_hidden`, `anchor_width`,
`anchor_height`, `available_width`, and `available_height`. The measured fields stay zero — and
`is_measured()` returns `false` — until the popover has been painted once with a bound handle.

### Hover opening

`PopoverHoverState` is Base UI's Trigger `openOnHover`. The application still owns whether the
popover is open; the state decides when that changes:

```rust,ignore
let popover = Popover::new("trigger", "popup", self.hover.is_open());
let trigger = self.hover.trigger_part(cx, popover, |view| &mut view.hover, div());
let popup = self.hover.popup_part(cx, popover, |view| &mut view.hover, div());
```

`delay(...)` (Base UI's `delay`, 300 ms by default) and `close_delay(...)` (`closeDelay`, immediate
by default) are exact one-shot deadlines bounded by `MAX_POPOVER_HOVER_DELAY`; entering or leaving
cancels the outstanding task rather than polling, and a zero delay applies the change in the same
controlled update with no task at all. Because the popup is hoverable by default, the pointer can
cross the `side_offset` gap without closing the surface; `hoverable_popup(false)` closes as soon as
the pointer leaves the trigger. `open_now`, `close_now`, and `toggle` apply an immediate change and
cancel any outstanding deadline, so a click listener and a hover deadline cannot fight. Each method
has a `_with` counterpart taking a `StateAccessor` for a host that renders many declared popovers
through one view.

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


### Menu parts

`MenuState` is the surface around the [`PopoverMenu`] row model: the controlled open flag, Base
UI's `Menu.Root` props, the anchored in-window placement its popup uses, and the exact hover
deadlines an `openOnHover` trigger or a submenu trigger needs. It composes the in-window
[`Popover`], so flip-aware placement, dismissal, focus restoration, and modal containment are the
ones documented above.

| Base UI part | QuickGUI decorator | What QuickGUI owns |
| --- | --- | --- |
| Root | `MenuState::new(trigger_id, popup_id)`, `root_part(element)` | controlled open flag, Root props, derived part identities |
| Trigger | `trigger_part(cx, access, on_open_change, element)` | button semantics, `expanded`, `has-popup`, click toggling, `open_on_hover` deadlines |
| Portal / Positioner | `portal_part(element)`, `positioner_part(element)` | anchoring, flip/shift, side and align offsets, placement reporting |
| Backdrop | `backdrop_part(element)` | full-viewport, accessibility-hidden pointer layer |
| Popup | `popup_part(cx, access, on_open_change, element)` | menu role, Escape/outside dismissal, focus restoration, `close_parent_on_esc` |
| Arrow | `arrow_part(element)` | absolute placement on the resolved edge |
| SubmenuRoot | `submenu_root_part(element)` | the nested level's structural wrapper |
| SubmenuTrigger | `submenu_trigger_part(cx, access, on_open_change, element)` / `PopoverMenu::submenu_trigger_part(menu_id, index, open, element)` | `menuitem` semantics, `has-popup`, `expanded` |
| Item | `PopoverMenu::item_part(menu_id, index, element)` | `menuitem` role, disabled state, accessible name, derived identity |
| LinkItem | `PopoverMenu::link_item_part(menu_id, index, element)` | `menuitem` role over an item whose activation dispatches `OpenMenuLink` |
| CheckboxItem / RadioItem | `checkbox_item_part(...)`, `radio_item_part(...)` | kind-checked checkable-item roles and checked state |
| CheckboxItemIndicator / RadioItemIndicator | `PopoverMenu::checkbox_item_indicator_part(element)`, `radio_item_indicator_part(element)` | accessibility-hidden decoration |
| Group / GroupLabel | `group_part(element)`, `labeled_group_part(menu_id, label_index, element)`, `group_label_part(...)` | group role and the mounted `labelled-by` relationship |
| RadioGroup | `PopoverMenu::radio_group_part(element)`, `labeled_radio_group_part(menu_id, label_index, element)` | radio-group role and its mounted label relationship |
| Separator | `PopoverMenu::separator_part(menu_id, index, element)` | non-interactive divider role |

The kind-checked aliases (`checkbox_item_part`, `radio_item_part`, `link_item_part`,
`submenu_trigger_part`, `separator_part`, `group_label_part`) return `None` when the index is not
that kind, so a composition that mounts parts by name cannot silently attach the wrong semantics.
`item_part` remains the single unchecked entry point.

Root props map one-to-one: `with_open`, `modal(true)` (Tab containment plus a mounted backdrop),
`orientation(MenuOrientation::Horizontal)` for a menubar row, `loop_focus`, `close_parent_on_esc`,
and `disabled`. A horizontal `PopoverMenu` installs `POPOVER_MENU_HORIZONTAL_KEY_CONTEXT` instead
of `POPOVER_MENU_KEY_CONTEXT`, so Left and Right move the highlight and Down opens the highlighted
submenu; bind `popover_menu_horizontal_key_bindings()` alongside the vertical set.

`open_on_hover(true)` adds Base UI's Trigger `openOnHover` with `delay` (100 ms by default) and
`close_delay` (immediate by default) as exact one-shot deadlines bounded by
`MAX_POPOVER_HOVER_DELAY`: a pointer that returns before the deadline cancels it rather than
reopening, and a zero delay applies the change in the same controlled update with no task at all.
A closed menu owns no task, timer, observer, or idle scheduler source.

Escape reaches exactly one dismiss region — the topmost one. `close_parent_on_esc(true)` therefore
makes the dismissed level dispatch the `MenuCloseParent` typed action, which travels the ordinary
focus path to the level above and stops at the first level that is already closed.

`MenuState::state()` returns a copyable `MenuPartState` carrying `open`, `side`, `align`, and
`anchor_hidden` from the resolved placement, and `PopoverMenu::item_part_state(index, submenu_open)`
returns a copyable `MenuItemPartState` carrying `highlighted`, `disabled`, `checked`, and `open`.

`PopoverMenuItem::link(id, label, url)` is Base UI's `Menu.LinkItem`. QuickGUI has no document to
navigate, so activation dispatches the `OpenMenuLink` typed action through the same owner path as
every other menu command and the application decides what opening the destination means — usually
`EventContext::open_url`. One destination is bounded by `MAX_POPOVER_MENU_LINK_BYTES`.

`PopoverMenu::radio_value(group)` and `set_radio_value(group, item_id)` are Base UI's
`Menu.RadioGroup` `value` and `onValueChange`: setting a value unchecks every other item in the
group in the same update, so a group can never retain two checked values.

## Go components

The `ui` package exposes this model as `PopoverMenu.Root` / `Trigger` / `Popup`. Rows are declared
as one bounded JSON model through `Items` instead of mounted child nodes, so the core still owns validation, highlighting,
typeahead, toggle policy, submenu models, and every accessibility relationship, and no synchronous
question crosses the hosted boundary while a menu is open. `popover_menu_key_bindings()` and
`popover_menu_horizontal_key_bindings()` are installed once by the binding, so declared menus adopt
the same contextual navigation as Rust applications. See
[Go components](go.md).

`MenuState` is bound as the Base UI-shaped `Menu` compound, whose rows are native nodes created in child callbacks:
`Menu.Root` (logical) with `Open`, `Modal`, `Orientation`, `LoopFocus`, `CloseParentOnEsc`, and
`Disabled`; `Menu.Trigger` with `OpenOnHover`, `Delay`, and `CloseDelay`; `Menu.Portal`,
`Backdrop`, `Positioner`, `Popup`, and `Arrow`; and `Menu.Item`, `LinkItem`, `SubmenuRoot`,
`SubmenuTrigger`, `Group`, `GroupLabel`, `RadioGroup`, `RadioItem`, `RadioItemIndicator`,
`CheckboxItem`, `CheckboxItemIndicator`, and `Separator` as ordinary child nodes. The application
owns every pixel of a row; the core owns its derived identity, `menuitem` semantics, roving
highlight, typeahead, toggle policy, activation, and closing policy, and publishes `MenuPartState`
and `MenuItemPartState` through `UseMenuState()` and `UseMenuItemState()`. See
[Go components](go.md).

## Roles, placement, and nesting

`PopoverKind::{Dialog, Menu, ListBox, Tree, Grid}` maps both the surface role and trigger
`has-popup` value. Child roles and behavior remain application-owned; selecting `Menu` does not
silently install a second menu-item registry. Existing typed actions, keymaps, picker state, or
custom controls can be composed inside the surface.

Use `.placement(...)`, or `.side(...)` and `.align(...)`, to choose a preferred side and
alignment. `.anchor_gap(...)`/`.side_offset(...)` and `.viewport_margin(...)`/
`.collision_padding(...)` configure finite, bounded positioner geometry. Minimum width, padding,
border, radius, shadow, typography, and all other presentation belong on the caller's popover
element.

Nested in-window popovers use the same API. Because dismissal regions are ordered by actual overlay
paint order, Escape or an outside press dismisses only the topmost nested surface first. Nested
native popover panels use their bounded platform grab stack; menu commands can explicitly close the
complete chain with `close_popover_chain`. No second component-owned popover registry is retained.

## Resource behavior

A `Popover` descriptor is `Copy` and contains two declared IDs, controlled state, semantic kind,
bounded structural placement, dismissal policy, optional initial focus, and one optional copied
placement report. Derived part IDs are
computed without allocation. It owns no `Rc`, appearance token, component store, observer, task,
timer, animation loop, or native window. Closed content is unmounted; an open, settled popover adds
no deadline or idle frame. A mounted popover uses the existing one fixed relation table for its
optional title/description targets; native relationships are projected only when an accessibility
update is requested.

`AnchorPlacementHandle` retains one small allocation and no task, timer, or observer. A
`PopoverHoverState` owns at most one pending foreground task, and only while a hover deadline is
outstanding; it is cancelled on the next hover change or immediate open/close.

`SystemPopover` is also a copied descriptor and adds no observer, task, timer, or idle deadline.
While open, its child owns one native window and one WGPU surface but shares the application's GPU
device, queue, fonts, assets, and bounded caches. A passive popover adds no native mouse monitor;
menu-style popovers share the runtime's single bounded monitor pair.

Run the caller-styled edge placement, nesting, focus, menu-role, and parts gallery with:

```console
cargo run --release --example popovers
cargo run --release --example system_popover
```

## Go popover composition

The `ui` package binds the whole compound. `Popover.Root` is a logical coordinator, and the
trigger is the one part the core keeps mounted whether the popover is open or closed, so the
trigger node carries the declaration and every other part only repeats the compound scope:

```go
func AccountPopover() *native.Node {
	open, setOpen := ui.CreateSignal(false)
	modal, hover := true, true
	gap, margin := 8.0, 12.0
	return ui.Popover.Root(
		ui.PopoverRootProps{
			Open:         open,
			Modal:        &modal,
			OnOpenChange: func(value bool, _ ui.PopoverOpenChangeDetails) { setOpen(value) },
		},
		func() *native.Node {
			return ui.Fragment([]*native.Node{ui.Popover.Trigger(
				ui.PopoverTriggerProps{
					OpenOnHover: &hover,
					Delay:       300,
					CloseDelay:  100,
				},
				"Account",
			),
				ui.Popover.Positioner(
					ui.PopoverPositionerProps{
						Side:             "bottom",
						Align:            "end",
						SideOffset:       &gap,
						CollisionPadding: &margin,
					},
					func() *native.Node {
						return ui.Popover.Popup(
							ui.PopoverPopupProps{},
							func() *native.Node {
								return ui.Fragment([]*native.Node{ui.Popover.Arrow(ui.PartProps{}),
									ui.Popover.Title(ui.PartProps{}, "Account"),
									ui.Popover.Viewport(ui.PartProps{}, "Account settings"),
									ui.Popover.Close(ui.PartProps{}, "Done")})
							},
						)
					},
				)})
		},
	)
}
```

`Side`, `Align`, `SideOffset`, `AlignOffset`, `CollisionPadding`, `Sticky`, and `Anchor` are
declared on `Popover.Positioner` exactly as Base UI declares them, and `Popover.Root` accepts the
same names as defaults. `Anchor` takes another retained `*native.Node`. A closed
popover mounts no positioner, popup, arrow, viewport, or backdrop at all.

The declared side is a preference, not an outcome. `UsePopoverPlacement()` and `OnPlacementChange`
report the placement the retained tree really used — `Side`, `Align`, `AnchorHidden`, the measured
anchor size, and the room the popup was given — published during the paint QuickGUI was already
performing, so an application sizes and styles from the core's own answer rather than measuring
anything itself. `OpenOnHover` hands the open value to the core's exact hover deadlines and reports
the result back with a `"hover"` reason. `Popover.Content` remains the one-element shorthand for a
popover that needs no separate positioner. See
[Go components](go.md).
