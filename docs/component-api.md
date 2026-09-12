# Component and view API

[Documentation index](README.md) · [Go guide](go.md) · [Rust views](view-api.md)

Components expose named parts and ordinary fluent elements. Configure component
behavior on the instance, then compose and style its parts.

## Direct children

Go primitive control constructors take no content arguments: use `ui.Button().Child("Save")`, `ui.Input().Value(value)`, and `ui.VirtualList().Children(rows)`. Dedicated text helpers keep `ui.Text(value)` and `text(value)`.

Strings are children in both languages. Use a text element when that text needs
its own styles; a direct string inherits typography from its parent.

```rust
button().child("Save")
```

```go
ui.Button().Child("Save")
```

The same rule applies to every compound part:

```rust
let popover = Popover::new("help-trigger", "help-popup", open);
popover.root().child(popover.trigger().child("Help"))
```

```go
popover := ui.NewPopover()
return popover.Root().Child(popover.Trigger().Child("Help"))
```

## Rust parts

Short methods construct unstyled elements: `root()`, `trigger()`, `positioner()`,
`popup()`, `title()`, and `description()`. Chain layout, styles, listeners, and
children directly on the returned element. State and identity arguments remain
explicit where a part needs them, such as `dialog.trigger("open-dialog")`.

The corresponding `*_with(...)` methods decorate an existing element. This is
useful for an input, image, or another custom root with its own construction:

```rust
field.control_with(text_input(value).on_input(edit))
```

Accessor-based event wiring uses names such as `trigger_with_accessor`.
Popover and Dialog use `popup` for their accessible surface. Progress and Meter
expose numeric reads through `current_value()` and the display part through
`value()`.

Controlled state stays in the application. Mount optional parts when their state
allows it; existing `Option<Element>` parts retain that conditional behavior.

## Go instances

Create an instance with `ui.NewPopover()`, `ui.NewTabs()`, `ui.NewAccordion()`, or
the corresponding constructor. Configure component behavior on that instance,
and configure presentation on each returned `*ui.Element`:

```go
func AccountMenu() *ui.Element {
    popover := ui.NewPopover().Side("bottom").SideOffset(8)
    return popover.Root().Children(
        popover.Trigger().Padding(12).Child("Account"),
        popover.Positioner().Child(
            popover.Popup().Padding(16).Children(
                popover.Title().FontWeight(600).Child("Your account"),
                popover.Close().Child("Done"),
            ),
        ),
    )
}
```

Part identities such as tab values belong in their constructors:

```go
tabs := ui.NewTabs().DefaultValue("account")
return tabs.Root().Children(
    tabs.List().Children(
        tabs.Tab("account").Child("Account"),
        tabs.Tab("security").Child("Security"),
    ),
    tabs.Panel("account").Child("Account settings"),
    tabs.Panel("security").Child("Security settings"),
)
```

A part's `With` variant accepts its explicit props when it needs more than its
identity, for example `tabs.TabWith(ui.TabsTabProps{Value: "account", Index: &index})`.
Other parts accept one optional props value. Constructors accept optional initial
settings; instance methods configure individual settings before mounting.

Parts are declarations until inserted into their root. This allows ordinary Go
argument evaluation to construct an entire tree without provider callbacks. Return
the element, pass it to `.Child(...)`, or use `.NativeNode()` at a native boundary;
the `.Node` field is populated after insertion. Create one instance per mounted
component. Its state, child contexts, callbacks, and bindings are released on
unmount.

Configure instance settings before mounting. Controlled settings accept plain
values and accessors; direct reactive reads are retained by the Go view compiler,
just like primitive fluent properties. Event handlers installed with
`.OnClick(...)` or `.OnClickEvent(...)` run before the component's default action;
`PreventDefault()` cancels that action.

`Popover.Content` and `SystemPopover.Content` mount their contents while open.
Strings can be supplied directly. Use a child factory when the content constructs
nodes that must be recreated after closing:

```go
popover.Content().Padding(16).Child(func() *ui.Element {
    return popover.Close().Child("Done")
})
```

Fluent styles and refs on Content apply to the actual content root each time it
opens. Ordinary children are retained; updates to state or text do not reconstruct
the containing component.

## TypeScript JSX

`<View>some text</View>` accepts text directly. The intrinsic aliases `<div>` and
`<span>` map to `View` and `Text`, respectively, without component imports:

```tsx
<div style={{ flexCol: true, gap2: true }}>
  some text
  <span style={{ textLg: true, fontSemibold: true }}>Styled text</span>
</div>
```

Use the scaffold's `jsxImportSource: "@quickgui/solid"` configuration. The universal
compiler supplies its runtime imports. All visual declarations use camelCase keys
inside `style`; direct style attributes, `class`, and `className` are unsupported.

## Maintaining the API

The Go instance methods are generated from the typed part implementations. Run
`go -C go/ui run ../internal/cmd/componentsgen` after changing those implementations.
`bash scripts/check-go.sh` checks the generated output, SDK, and compiled examples.
Rust and Go documentation references are extracted from the public source APIs.
