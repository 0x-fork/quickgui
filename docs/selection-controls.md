# Selection controls

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI's checkbox, radio, radio-group, and switch contracts are unstyled. The framework owns
semantics and interaction; the application owns every visible pixel, including layout, indicators,
thumbs, labels, hover/focus/disabled paint, and motion.

Every value is controlled by the owning view. A component declaration never retains a second copy
of application state:

```rust
use quickgui::{Checkbox, div, text};

let toggle = cx.listener("auto-save", |view, cx| {
    view.auto_save = !view.auto_save;
    cx.invalidate();
});
let checkbox = Checkbox::new(self.auto_save);

checkbox
    .root_part(
        div()
            .flex_row()
            .items_center()
            .gap_2()
            .child(checkbox.indicator_part(
                div().size(16.0, 16.0).children(
                    self.auto_save.then(|| text("✓")),
                ),
            ))
            .child("Save automatically"),
    )
    .id("auto-save")
    .on_click(toggle)
```

`root_part` decorates the caller's `Element` in place. It does not add children, dimensions,
spacing, colors, borders, radii, state paint, transitions, or other appearance. The separate
indicator/thumb parts only hide decorative content from the accessible name.

## Checkbox and mixed state

`Checkbox::new(bool)` covers an ordinary controlled checkbox. Pass `ToggleState::Mixed` for a
collection summary equivalent to a web checkbox's `indeterminate` state:

```rust
use quickgui::{Checkbox, ToggleState, div};

let checkbox = Checkbox::new(ToggleState::Mixed);
checkbox
    .root_part(
        div()
            .child(checkbox.indicator_part(div().child("−")))
            .child("Select visible files"),
    )
    .id("select-visible")
    .on_click(cycle_selection)
```

The root projects the exact AccessKit checkbox role and `off`, `on`, or `mixed` toggle state.
Custom checkbox-like roots can also use `.checked(bool)`, `.toggle_state(...)`, or
`.indeterminate(bool)` directly.

For a root without a distinct indicator part, `checkbox(state)` is shorthand for
`Checkbox::new(state).root_part(div())`.

## Radio groups

Decorate an application-owned group with `RadioGroup::root_part`, then place controlled `Radio`
roots inside it:

```rust
let group = RadioGroup::new();
let compact = Radio::new(self.density == Density::Compact);
let comfortable = Radio::new(self.density == Density::Comfortable);

group.root_part(
    div()
        .accessibility_label("Editor density")
        .flex_col()
        .children([
            compact
                .root_part(
                    div()
                        .child(compact.indicator_part(div()))
                        .child("Compact"),
                )
                .id("compact")
                .on_click(select_compact),
            comfortable
                .root_part(
                    div()
                        .child(comfortable.indicator_part(div()))
                        .child("Comfortable"),
                )
                .id("comfortable")
                .on_click(select_comfortable),
        ]),
)
```

Tab enters at the checked option, or the first enabled option when none is checked. Left/Up and
Right/Down move, focus, and activate the previous or next enabled option with wraparound. Disabled
options are skipped and nested groups remain independent. The mounted tree is scanned only for a
Tab-index rebuild or an actual arrow key; no separate item registry is retained.

`radio(selected)` and `radio_group()` are unstyled semantic-root shorthands.

## Switch

The switch root doubles as the application-owned track or can contain a separate track element.
Only the decorative thumb has a dedicated part:

```rust
let control = Switch::new(self.sync_settings);

control
    .root_part(
        div()
            .flex_row()
            .child(
                div()
                    .size(30.0, 18.0)
                    .child(control.thumb_part(div().size(14.0, 14.0))),
            )
            .child("Sync settings"),
    )
    .id("sync-settings")
    .on_click(toggle_sync)
```

The root projects the native switch role and exact boolean toggle state. `switch(checked)` is the
unstyled root shorthand.

## Shared interaction and resource contract

All selection roots are clickable, keyboard-focusable, excluded from hidden-inset window drag
regions, non-text-selectable, and use the desktop arrow cursor. Add a stable `.id(...)` and
`.on_click(...)` listener to change controlled state. Space activates the focused root through the
ordinary button path. `.disabled(true)` removes pointer, keyboard, focus, and native accessibility
actions; the application decides whether and how disabled state looks via `.disabled_style(...)`.

Descriptors are copy-only values. They allocate no state, create no GPU asset, and retain no task,
timer, observer, registry entry, transition, or idle scheduler source. Application-authored paint
transitions remain event-driven and terminate normally.

Run the caller-styled light/dark, mixed-state, disabled-state, mouse, Tab, and arrow-key gallery:

```console
cargo run --release --example selection_controls
```

## QuickGUI UI

`readOnly` is bound on `Checkbox.Root`, `Radio.Root`, `RadioGroup.Root`, and `Switch.Root`, and
`RadioGroup.Root` also accepts `required`. It is the web's `readonly` rather than `disabled`: the
control keeps its place in the Tab sequence and its value in the accessible name while the core
refuses every change.

A `Checkbox.Root parent` inside a `CheckboxGroup.Root` is the group's derived parent checkbox and
needs nothing else. Standalone, `childrenChecked` declares the children's booleans and the core
folds them into on, mixed, or off — a registry-free derivation, so no JavaScript decides what
"mixed" means:

```tsx
<Checkbox.Root parent childrenChecked={[true, false, true]}>
  <Checkbox.Indicator />
</Checkbox.Root>
```


`@quickgui/ui` exposes these descriptors as Base-UI-shaped compound parts: `Checkbox.Root` /
`Checkbox.Indicator`, `Radio.Root` / `Radio.Indicator` inside `RadioGroup.Root`, and `Switch.Root` /
`Switch.Thumb`. Each part is one native node that declares which core descriptor to rebuild, so the
role, exact on/off/mixed toggle state, click and Space activation, focus, cursor, and window-drag
exclusion all come from this Rust layer rather than a JavaScript reimplementation.

```tsx
<Checkbox.Root checked={notify()} onCheckedChange={setNotify}>
  <Checkbox.Indicator>{notify() === true ? "✓" : "–"}</Checkbox.Indicator>
  <Text>Email me about releases</Text>
</Checkbox.Root>
```

`checked` accepts `true`, `false`, or `"indeterminate"` and maps onto `ToggleState`.
`onCheckedChange` receives the next value and the originating event. See the
[QuickGUI UI renderer guide](ui.md) for the full binding table.

## Read-only controls and parent checkboxes

`Checkbox::read_only`, `Radio::read_only`, `RadioGroup::read_only`, and `Switch::read_only` are Base
UI's `readOnly`. Unlike `disabled`, a read-only control stays focusable and stays in the Tab
sequence: it shows a value the user may read and copy but not change. The state is projected to the
native accessibility tree through `Element::accessibility_read_only`, and the transition itself is
refused through the descriptor:

```rust,ignore
// A read-only control answers `None`, so the listener writes nothing at all.
match Switch::new(view.audit_logging).read_only(true).next_checked() {
    Some(checked) => view.audit_logging = checked,
    None => {}
}
```

`Checkbox::next_state()` returns the tri-state transition a click should make — mixed and off both
move to on — or `None` when the checkbox refuses. `Radio::accepts_selection()` answers the same
question for a radio, and `Switch::next_checked()` for a switch. Asking before writing is what makes
`readOnly` a framework contract rather than a styling hint.

`Checkbox::parent(children)` is Base UI's parent checkbox: it derives on, off, or mixed from the
checked values the application is already rendering, and `parent_next_checked()` reports the value
activating the parent moves the whole group to. QuickGUI retains no child registry, so the
derivation stays a pure function of the caller's own data.
