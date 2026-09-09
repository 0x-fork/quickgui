# Collapsible and accordion

[Documentation index](README.md) · [Component roadmap](component-roadmap.md)

QuickGUI provides controlled, unstyled disclosure parts. `Collapsible` models one trigger and
panel. `Accordion` composes heading, trigger, and region parts for a set of panels, while
`AccordionState` is an optional bounded helper for single- or multiple-value open state.

The component layer adds no colors, typography, spacing, borders, icons, animation, or easing.
Those declarations remain application-owned. Its behavior follows the current
[Base UI Collapsible](https://base-ui.com/react/components/collapsible),
[Base UI Accordion](https://base-ui.com/react/components/accordion), and
[WAI-ARIA disclosure](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/) contracts.

## Collapsible

The application owns the `open` boolean and listener:

```rust
let disclosure = Collapsible::new("recovery", self.recovery_open);
let toggle = cx.listener(disclosure.trigger_id(), |view, cx| {
    view.recovery_open = !view.recovery_open;
    cx.invalidate();
});

disclosure
    .root_part(
        div().child(
            disclosure
                .trigger_part(div().child("Recovery keys"))
                .on_click(toggle),
        ),
    )
    .children(
        disclosure.panel_part(
            div().child("alien-bean-pasta · wild-irish-burrito"),
        ),
    )
```

`trigger_part` contributes button, focus, Enter/Space activation, `expanded`, and the mounted
panel relationship. It explicitly uses the desktop arrow cursor and opts out of hidden-inset
window dragging. The trigger references the panel only while open, matching Base UI and avoiding
a dangling native accessibility relation.

Closed panels are unmounted by default. `.keep_mounted(true)` instead returns the closed panel as
`display: none`. Because `panel_part` returns `Option<Element>`, pass it to `.children(...)`; the
same composition works for both policies. A retained closed panel contributes no layout, paint,
input, focus, or accessibility node.

## Accordion

`AccordionState` stores only stable IDs for currently open values. Single-value mode is the
default; opening another value replaces the previous value. Multiple mode keeps independent
values:

```rust
struct SettingsView {
    sections: AccordionState,
}

let accordion = Accordion::new("settings").heading_level(3);
let account = accordion.item_from_state("account", 0, &self.sections);

let toggle_account = cx.listener(account.trigger_id(), |view, cx| {
    view.sections.toggle("account").expect("bounded state");
    cx.invalidate();
});

accordion.root_part(
    div().child(
        account.root_part(
            div()
                .child(account.header_part(
                    div().child(
                        account
                            .trigger_part(div().child("Account"))
                            .on_click(toggle_account),
                    ),
                ))
                .children(account.panel_part(div().child("Account settings"))),
        ),
    ),
)
```

Each header projects the caller-selected heading level. Its trigger is an ordinary button with
exact `expanded` and mounted-panel `controls` state. A mounted panel is a named accessibility
region labelled by that trigger. Item and root disabled state prevent pointer and keyboard
activation without changing application presentation.

Current WAI-ARIA guidance keeps all enabled accordion triggers in normal Tab order. Base UI 1.7
deprecated its old orientation and loop-focus props after that guidance changed. QuickGUI does not
retain a roving-focus registry or intercept arrow keys: Tab and Shift-Tab visit headings, while
Enter and Space use the existing button path.

## Bounds and scheduling

`AccordionState` uses a sorted compact vector capped at 4,096 simultaneously open values. A fresh
empty state allocates nothing; capacity grows only when a value opens and is reused across later
toggles. Lookup is logarithmic, insertion/removal happens only on a user or application action,
and invalid replacement is atomic. Single mode retains at most one value. Multiple mode rejects
duplicates and over-limit replacement.

Descriptors are copyable declarations containing IDs and booleans. They create no task, timer,
observer, animation, registry, GPU resource, or idle frame. Mounting or unmounting content causes
only the application-requested invalidation. Any transition is explicitly application-owned.

Run the caller-styled gallery with:

```console
cargo run --release --example disclosures
```

The gallery includes a standalone collapsible, single-value accordion, multiple-value accordion,
retained closed panels, a disabled heading, and application-owned visuals.

## Go components

The `ui` package exposes `ui.Collapsible.Root`, `Trigger`, and `Panel`, plus
`ui.Accordion.Root`, `Item`, `Header`, `Trigger`, and `Panel`. Components declare
their children in callbacks so each part receives its enclosing component context.

```go
func ShippingDetails() *native.Node {
	open, setOpen := ui.CreateSignal([]string{"shipping"})
	return ui.Accordion.Root(
		ui.AccordionRootProps{
			Value:         open,
			OnValueChange: func(value []string, _ *native.Event) { setOpen(value) },
			Multiple:      true,
			HeadingLevel:  4,
		},
		func() *native.Node {
			return ui.Accordion.Item(
				ui.AccordionItemProps{Value: "shipping"},
				func() *native.Node {
					return ui.Fragment([]*native.Node{ui.Accordion.Header(
						ui.PartProps{},
						func() *native.Node {
							return ui.Accordion.Trigger(ui.PartProps{}, "Shipping")
						},
					),
						ui.Accordion.Panel(ui.PartProps{}, "Orders ship within two business days.")})
				},
			)
		},
	)
}
```

The signal owns the controlled value. Button semantics, expanded state, panel
relationships, heading level, and the decision to omit or retain a closed panel
with `KeepMounted` come from the core. Single mode replaces the open value;
multiple mode toggles within the bounded set. See the [Go guide](go.md).
