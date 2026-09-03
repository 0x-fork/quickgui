# Base UI components: separator, avatar, checkbox group, preview card, scroll area, OTP field, drawer, and navigation menu

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

These eight components close the remaining gaps against the
[Base UI](https://base-ui.com/react/overview/quick-start) catalog. They follow QuickGUI's usual
contract exactly: a `*State` value for controlled behavior where a component retains anything, plus
caller-owned `*_part(element)` decorators named after Base UI's compound parts. QuickGUI owns roles,
relationships, state, keyboard and pointer behavior, focus, dismissal, placement, and accessibility;
the application owns every colour, radius, size, icon, and transition.

Nothing here starts a timer, an observer, or an animation source. Every delay is an exact one-shot
deadline reported by `next_deadline()` and applied by `poll(now)`, in the same shape as
[`ToastManager::next_deadline`](toolbar-and-toast.md); a settled component reports no deadline and
the window stays asleep.

Run the caller-styled gallery with:

```console
cargo run --release --example base_ui_components
```

## Separator

`Separator` projects the Separator role with an orientation and nothing else — no thickness, no
colour, no inset. It is never focusable and never contributes to a neighbouring accessible name.

```rust
use quickgui::{Color, Separator, SeparatorOrientation, div, separator};

let rule = Separator::horizontal().root_part(div().h(1.0).bg(Color::rgb8(220, 220, 220)));
let inline = separator(SeparatorOrientation::Vertical);
```

A horizontal separator divides vertically stacked content; a vertical one divides horizontally
arranged content, matching Base UI and ARIA. Adjustable dividers remain
[`Splitter`](range-and-feedback.md), which is a different, focusable, value-carrying control.

## Avatar

`AvatarState` holds the load status and the one exact deadline that keeps a fast image from
flashing initials on screen. `Avatar` supplies the root's Image role and accessible name plus
accessibility-hidden image and fallback parts, so an avatar is announced exactly once however it
renders.

```rust
use std::time::{Duration, Instant};
use quickgui::{Avatar, AvatarLoadingStatus, AvatarState, div, img, text};

let mut state = AvatarState::new().delay(Duration::from_millis(200));
state.set_loading_status(AvatarLoadingStatus::Loading, Instant::now());

let avatar = Avatar::new("member", "Ada Lovelace");
let mut root = avatar.root_part(div().size(48.0, 48.0).rounded_full());
if state.shows_image() {
    root = root.child(avatar.image_part(img(source).size_full()));
}
if state.shows_fallback() {
    root = root.child(avatar.fallback_part(text("AL")));
}
```

`Fallback.delay` is `AvatarState::delay`, bounded by `MAX_AVATAR_FALLBACK_DELAY` (10 s). While the
delay is armed, `AvatarState::next_deadline` reports the single instant a repaint is needed;
`Avatar::schedule(cx, &state)` asks for exactly that repaint.

QuickGUI decodes images on a worker pool and has no per-element completion event, so the
application reports what it already knows — from its own `cx.spawn` load, an `ImageResource`
loader, or a hosted renderer — through `Avatar::apply_loading_status`, which is the
`onLoadingStatusChange` counterpart: it applies the status, re-arms the deadline, and runs the
owner callback only for a real transition.

## Checkbox group

`CheckboxGroupState` owns a bounded declared universe of values, the checked subset, and the
group's disabled flag. `MAX_CHECKBOX_GROUP_VALUES` is 256 for both. The parent checkbox has no
retained value of its own: its on/mixed/off state is derived from the children, so the two can
never disagree.

```rust
use quickgui::{CheckboxGroup, CheckboxGroupState, div};

let state = CheckboxGroupState::new(["red", "green", "blue"]).checked(["green"]);
let group = CheckboxGroup::new("colors", &state);

let parent = group.on_parent_click(cx, |view: &mut Form| &mut view.colors, |view, values, _| {
    view.summary = format!("{} checked", values.len());
});
let mut root = group
    .root_part(div())
    .child(group.parent_part(div()).on_click(parent));
for value in ["red", "green", "blue"] {
    let click = group.on_checkbox_click(cx, value, |view: &mut Form| &mut view.colors, |_, _, _| {});
    root = root.child(group.checkbox_part(value, div()).on_click(click));
}
```

Items reuse the existing [`Checkbox`](selection-controls.md) descriptor, so the role, exact
on/off/mixed semantics, click behavior, and desktop pointer contract are identical to a standalone
checkbox. Checked values keep the declared order regardless of click order; a value outside the
declared universe is ignored; a disabled group refuses every mutation. `on_value_change` receives
the whole new value set, matching Base UI.

## Preview card

`PreviewCardState` owns the two exact deadlines a hover card needs: `delay` (600 ms by default)
before opening once the pointer rests on the trigger, and `close_delay` (300 ms) before closing
once the pointer has left both the trigger and the popup. Focus opens it immediately, because a
keyboard user has no way to rest a pointer. Both are bounded by `MAX_PREVIEW_CARD_DELAY` (10 s).

```rust
use quickgui::{PreviewCard, PreviewCardState, div, text};

let card = PreviewCard::from_state("profile-link", "profile-card", &self.card);
let hover = card.on_trigger_hover(cx, |view: &mut Page| &mut view.card, |_, _, _| {});
let dismiss = card.on_dismiss(cx, |view: &mut Page| &mut view.card, |_, _, _| {});

let mut root = card
    .root_part(div().relative())
    .child(card.trigger_part(text("@ada")).on_hover(hover));
if card.is_open() {
    root = root.child(
        card.positioner_part(div()).child(
            card.popup_part(div())
                .on_dismiss(dismiss)
                .child(card.arrow_part(div())),
        ),
    );
}
```

The trigger projects the **Link** role rather than Button: a preview card previews a destination
rather than opening a menu. The parts compose the existing in-window
[`Popover`](popovers.md), so side/align placement, collision handling, Escape and outside-press
dismissal, and focus restoration are the popover's. QuickGUI's retained overlay node is itself the
portal, so `portal_part` and `positioner_part` decorate the same boundary — mount exactly one.

Escape clears the retained hover flags as well as the open value, so a card dismissed under the
pointer does not immediately schedule itself open again.

## Scroll area

`ScrollAreaState` is a controlled geometry value: the viewport and content extents the application
laid out, the clamped offsets, the derived overflow flags, the thumb arithmetic, and the captured
pointer contract for thumb drags and track presses. The arithmetic mirrors the built-in overlay
scrollbar in `src/ui_tree/pointer.rs` — a wheel delta subtracts from the offset, the thumb length is
the viewport-to-content ratio with a `MIN_SCROLL_AREA_THUMB_LENGTH` (24 px) minimum, and the thumb
travels the remaining track.

```rust
use quickgui::{ScrollArea, ScrollAreaOrientation, ScrollAreaState, Size, div};

self.log.set_geometry(Size::new(260.0, 160.0), Size::new(260.0, 900.0));
let area = ScrollArea::new("log");
let style = self.log.style_state();

area.root_part(div())
    .child(
        area.viewport_part(div().on_scroll_wheel(wheel))
            .child(area.content_part(div()).translate(0.0, -self.log.offset().y)),
    )
    .child(
        area.scrollbar_part(&self.log, ScrollAreaOrientation::Vertical, div().on_pointer(track))
            .child(area.thumb_part(ScrollAreaOrientation::Vertical, div().on_pointer(drag))),
    )
```

`ScrollAreaStyleState` is the `data-`-like render state an application styles from: `scrolling`,
`hovering`, `has_overflow_x` / `has_overflow_y`, and `overflow_x_start` / `overflow_x_end` /
`overflow_y_start` / `overflow_y_end`. The edge flags use `overflow_edge_threshold`, which defaults
to 1 logical pixel and is bounded by `MAX_SCROLL_AREA_OVERFLOW_THRESHOLD` (256). Every field is
derived on read, so a settled scroll area does no work at all.

`keep_mounted` keeps a scrollbar mounted while its axis cannot scroll; the mounted-but-useless
scrollbar is hidden from assistive technology so it is never announced.

**Relationship to the built-in scrollbars.** QuickGUI's native-style overlay scrollbars remain the
default on any ordinary `overflow_y_scroll` container and are untouched. `ScrollArea`'s viewport is
a clipped `overflow_hidden` box whose offset the application applies as a paint-only transform, so
the two never fight over the same wheel event and a product that wants to draw its own scrollbars
can, without losing the retained arithmetic.

## OTP field

`OtpFieldState` owns the per-slot characters, the declared length (bounded by `MAX_OTP_LENGTH`,
12), the accepted character class, the focused slot, and the disabled/read-only/required policy.
Each slot composes the existing [`text_input()`](text-and-forms.md); `src/text_input.rs` itself is
unchanged.

```rust
use quickgui::{OtpField, OtpFieldState, OtpValidationType, div, text_input};

let field = OtpField::new("code").auto_submit("verify");
let mut root = field.root_part(&self.code, div().flex_row());
for index in 0..self.code.length() {
    root = root.child(field.slot_part(
        cx,
        &self.code,
        index,
        text_input(self.code.slot_text(index)),
        |view: &mut Verify| &mut view.code,
        |view, value, _| view.value = value.to_owned(),
        |view, value, _| view.completed = Some(value.to_owned()),
    ));
}
```

`slot_part` decorates the slot and attaches its whole behavior: accepted characters fill the slot
and advance, typing over a filled slot replaces it, a paste distributes across consecutive slots
from the focused index, Backspace clears in place and then walks back, Delete clears without moving,
and the arrows plus Home/End move between slots. Install `otp_field_key_bindings()` once on the
application keymap; QuickGUI matches contextual bindings before an ordinary text input's own
editing, so the OTP field owns those keys while every other key still reaches the composed input.

`validation_type` selects `Numeric` (the default), `Alpha`, `Alphanumeric`, or `None`; `mask`
presents the code the way a password input is presented; `required` drives the field's validity,
which projects on the root group and on every slot. `auto_submit(form)` routes through
`EventContext::submit_form`, so completing the code validates and submits the named form exactly as
a Return press would. `separator_part` is decorative and hidden, so it never interrupts the
announced code.

## Drawer

`DrawerState` owns the open value, bounded snap points (`MAX_DRAWER_SNAP_POINTS`, 8), the active
snap point, and the captured swipe: its live offset, whether a swipe is in progress, and the flick
velocity that decides between snapping and dismissing. A snap point at or below `1.0` is a fraction
of the viewport extent; a larger one is an absolute logical-pixel extent. Values are sorted,
de-duplicated, and clamped to `MAX_WINDOW_LOGICAL_DIMENSION`.

```rust
use quickgui::{Drawer, DrawerModality, DrawerState, SwipeDirection, div};

let state = DrawerState::new(SwipeDirection::Down).snap_points(&[0.45, 1.0]);
let drawer = Drawer::from_state("filters", &state)
    .modal(DrawerModality::Modal)
    .initial_focus("sheet-first");

drawer
    .portal_part(div())
    .child(drawer.backdrop_part(div()))
    .child(drawer.viewport_part(div().flex_col().justify_end()).child(
        drawer
            .popup_part(div())
            .translate(0.0, state.swipe_offset())
            .child(drawer.swipe_area_part(div().on_pointer(swipe)))
            .child(drawer.title_part(text("Filters")))
            .child(drawer.description_part(text("Narrow the results")))
            .child(drawer.content_part(div()))
            .child(drawer.close_part("Close filters", div())),
    ))
```

Focus containment and restoration reuse [`Dialog`](dialogs.md)'s machinery, so Escape closes, the
backdrop dismisses unless `disable_pointer_dismissal` is set, and focus returns to the declared
control on every dismissal path. `DrawerModality` selects `Modal` (contain focus, project modal
semantics, dismiss from the backdrop), `TrapFocus` (contain focus only), or `NonModal` (neither).
The portal covers the window viewport on every modality, so placement and native-view occlusion stay
framework-owned; the modality selects focus containment and modal semantics, not pointer reach.

`DrawerState::apply_pointer(event, extent, now)` takes a captured pointer event and the viewport
extent along the swipe axis, and returns a `DrawerGesture` reporting whether anything the
application paints changed, whether the active snap point moved (the owner's
`on_snap_point_change`), and whether the gesture dismissed the drawer (the owner's
`on_open_change`). `Drawer::on_swipe` wires all of that for callers who do not need the raw
gesture. `swipe_offset` is never negative — dragging away from the dismissing edge holds the sheet
in place — and the application applies it as a paint-only transform, so a drag never relayouts.

Nested drawers declare their depth with `Drawer::depth`, which panics at or beyond
`MAX_NESTED_DRAWERS` (8) so a recursive composition fails at the declaration rather than by
exhausting focus scopes and overlay planes.

## Navigation menu

`NavigationMenuState` owns which item is open, which trigger holds the bar's single Tab stop, the
exact hover open/close deadlines (50 ms each by default, bounded by `MAX_NAVIGATION_MENU_DELAY`),
and the activation direction. `NavigationMenu` borrows the caller's ordered item list — bounded by
`MAX_NAVIGATION_MENU_ITEMS` (64), with distinct values — and keeps no registry.

```rust
use quickgui::{NavigationMenu, NavigationMenuItem, NavigationMenuState, div};

let items = [
    NavigationMenuItem::new("products"),
    NavigationMenuItem::new("solutions"),
    NavigationMenuItem::new("support").disabled(true),
];
let menu = NavigationMenu::new("main-nav", &self.nav, &items);
let mut list = menu.list_part(div());
for item in items.iter() {
    let entry = menu.entry(item.value()).expect("declared item");
    let click = entry.on_trigger_click(cx, |view: &mut Site| &mut view.nav, |_, _, _| {});
    let hover = entry.on_trigger_hover(cx, |view: &mut Site| &mut view.nav, |_, _, _| {});
    let trigger = entry.key_part(
        cx,
        entry.trigger_part(div()).on_click(click).on_hover(hover),
        |view: &mut Site| &mut view.nav,
    );
    list = list.child(entry.item_part(div()).child(trigger).child(entry.icon_part(div())));
}
menu.root_part(div()).child(list)
```

The root is a Navigation landmark, the list carries the List role and the menu's orientation, each
item is a ListItem with its position in the set, and each trigger is a Button with `has-popup`,
expanded state, and a controls relationship to its open panel. Each panel mounts through the
existing [`Popover`](popovers.md) parts — `portal_part` / `positioner_part` are the same merged
boundary, then `popup_part`, `viewport_part`, `content_part`, `arrow_part`, and `backdrop_part`.
`link_part(id, active, element)` decorates a navigation link and projects `active` as the native
selected state.

Hovering a trigger opens after `delay` when every panel is closed and switches **immediately** when
one is already open, which is the behavior a menu bar and a navigation menu share. Leaving both the
trigger and the popup arms the exact `close_delay`. `activation_direction()` reports `Left`,
`Right`, `Up`, or `Down` so the application can slide its panel the way the user's attention
travelled; QuickGUI never animates the panel itself.

Install `navigation_menu_key_bindings()` once on the application keymap. Arrow keys along the
menu's orientation plus Home and End move the bar's single Tab stop between enabled triggers,
skipping disabled items and wrapping unless `loop_focus(false)` is set. Enter and Space reach the
caller's click listener through ordinary button activation. Escape closes the open panel without
leaving the bar.

Keyboard movement deliberately does **not** swap which panel is open: unmounting the previous panel
restores focus to its own trigger, which would immediately undo the move the user just made. Pointer
hover and click switch panels; the keyboard moves focus and opens with Enter or Space.

## Solid

The [Solid renderer](solid.md) binds all eight through the same declared-part scheme every other
component uses, with Base UI's own compound and prop names: `Separator`, `Avatar`, `CheckboxGroup`,
`PreviewCard`, `ScrollArea`, `OtpField`, `Drawer`, and `NavigationMenu`. Each root allocates one
bounded scope internally, so the parts of an instance resolve to the same core identity with no
registry and nothing to repeat.

```tsx
import { Avatar, CheckboxGroup, Checkbox, Drawer, NavigationMenu, OtpField,
  PreviewCard, ScrollArea, Separator, Text } from "@quickgui/solid";

<Avatar.Root ariaLabel="Ada Lovelace" onLoadingStatusChange={setStatus}>
  <Avatar.Image src="./ada.png" />
  <Avatar.Fallback delay={120}><Text>AL</Text></Avatar.Fallback>
</Avatar.Root>

<CheckboxGroup.Root allValues={["red", "green", "blue"]} value={colors()} onValueChange={setColors}>
  <Checkbox.Root parent><Text>All colours</Text></Checkbox.Root>
  <Checkbox.Root value="red"><Text>Red</Text></Checkbox.Root>
</CheckboxGroup.Root>
```

The hosted boundary never waits: every result the core decides — the avatar load status, the
checked value set, a preview card's open value, a scroll area's clamped offset and derived
overflow flags, the OTP code and its completion edge, a drawer's open value, snap point, and live
swipe, and a navigation menu's open item and activation direction — travels back as one
asynchronous `componentchange` event and reaches the application through the matching
`on*Change` prop. Nothing the core must decide synchronously is asked of JavaScript: bounds,
values, deadlines, snap points, and the scroll area's laid-out extents are all declared ahead
through bounded properties.

Two things follow from that boundary and differ from the Rust API:

- **A scroll area declares its geometry.** QuickGUI has no layout observer at the hosted boundary,
  so `viewportSize` and `contentSize` are declared `{ width, height }` extents rather than
  measured. Everything derived from them stays the core's.
- **A part whose edge the core owns ignores a declared listener for that same edge.** A checkbox
  inside a group, a navigation-menu trigger, an OTP slot's input, a scroll area's scrollbar, thumb
  and wheel, a drawer's swipe area, and every popup's dismissal register exactly one listener each,
  which is the core's.

`useScrollAreaState()` and `useDrawerSwipe()` read the same reported state anywhere inside their
subtree, so an application styles a fade, a shadow, or a dragged sheet without an observer, a
timer, or a measurement of its own.

Run the Solid gallery, which includes all eight, with:

```console
bun run --cwd examples/components-solid dev
```

## Bounds

| Constant | Value | What it bounds |
| --- | --- | --- |
| `MAX_AVATAR_FALLBACK_DELAY` | 10 s | Longest avatar fallback deadline |
| `MAX_CHECKBOX_GROUP_VALUES` | 256 | Declared and checked values per checkbox group |
| `MAX_PREVIEW_CARD_DELAY` | 10 s | Preview-card open and close delays |
| `MAX_SCROLL_AREA_OVERFLOW_THRESHOLD` | 256 px | Scroll-area overflow edge threshold |
| `MIN_SCROLL_AREA_THUMB_LENGTH` | 24 px | Smallest reported scrollbar thumb |
| `MAX_OTP_LENGTH` | 12 | Slots retained by one OTP field |
| `MAX_DRAWER_SNAP_POINTS` | 8 | Snap points retained by one drawer |
| `MAX_NESTED_DRAWERS` | 8 | Declared drawer nesting depth |
| `MAX_DRAWER_DISMISS_VELOCITY` | 32 px/ms | Largest flick speed a drawer may require |
| `MAX_NAVIGATION_MENU_ITEMS` | 64 | Top-level items in one navigation menu |
| `MAX_NAVIGATION_MENU_DELAY` | 10 s | Navigation-menu open and close delays |

Return to the [documentation index](README.md).
