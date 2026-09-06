# QuickGUI components

A sidebar gallery of every compound component `@quickgui/ui` binds: the left rail is a vertical
`Tabs` list with one entry per component, and the right pane shows that component's demo in a
scrollable content area.

Each demo is a *live* declaration rather than a sketch — you open it, change it, and the last line
of the panel prints the state the Rust core reported back. The core owns filtering, clamping,
snapping, virtualization, keyboard navigation, focus policy, placement, and every deadline;
JavaScript owns only the controlled value, the element tree, and the paint.

```console
bun run --cwd examples/components dev
```

## Appearance

Nothing here hardcodes a palette. The example reads the appearance the window is really rendering
in — `Window.getState().appearance`, kept current through `Window.onStateChange` and
`Appearance.onChange` from `@quickgui/native` — and derives one of two palettes from it. Light is
the default; dark is used only while the system is dark, and the switch is live. The window's own
extent (`WindowState.viewportSize`) is tracked the same way and printed in the header, because the
hosted boundary has no layout observer.

Every signal write in the file happens inside an event handler or an `on*Change` callback. No
component body and no memo ever writes a signal.

## The tabs

| Group | Tabs |
| --- | --- |
| Disclosure and layout | Accordion, Collapsible, Separator, Splitter, Tabs, Scroll Area |
| Overlays | Alert Dialog, Dialog, Drawer, Popover, Preview Card, Tooltip |
| Menus | Context Menu, Menu, Menubar, Navigation Menu, Toolbar |
| Option sources | Autocomplete, Combobox, Select |
| Selection | Checkbox, Checkbox Group, Radio, Switch, Toggle, Toggle Group |
| Text and forms | Field, Fieldset, Form, Input, OTP Field, Number Field |
| Dates | Calendar, Date Field, Time Field |
| Feedback | Meter, Progress, Toast |
| Identity | Avatar, Button |
| Collections | Table, Tree |

Notable details each demo exercises:

- **Scroll Area** declares `viewportSize` and `contentSize` (both optional now — the native side
  also measures them from painted bounds), applies the core's clamped offset to the content as a
  paint-only `translateY`, and prints `hasOverflowY` / `overflowYStart` / `overflowYEnd` alongside
  it. The thumb's own position and length are the native side's; the application declares only its
  width and paint.
- **Splitter** drives its pane sizes entirely from `onSizesChange`, with one collapsible pane.
- **Table** declares only the rows the core last reported through `onVisibleRangeChange` out of five
  thousand, with multiple selection, a sortable column, and reported column widths.
- **Tree** has one `pending` branch that asks for its children exactly once through
  `onLoadChildren`, answered with a single validated `setChildren` splice.
- **Toast** drives `useToastManager()` — `add`, `update`, `closeAll` — and prints each toast's
  stack index, `limited` flag, and core-computed offset.
- **Menu** uses the Base UI row parts, including a link row, a checkbox row, a radio group, and a
  hover-opening submenu, each styling itself from `useMenuItemState()`.
- **Popover** and **Tooltip** print the placement the retained tree really resolved to, not the one
  that was declared.
- **Slider**, **Number Field**, **Progress**, and **Meter** read `useSliderState()`,
  `useNumberFieldState()`, and `useGaugeState()` for the core's own `dragging`, `scrubbing`,
  `status`, and formatted `displayValue`.

## Known gaps this example ran into

- **`@quickgui/ui` binds no `Form` compound.** The Rust core has one, but there is no `form`
  part in `packages/native/protocol.ts` and no `Form` export in `packages/ui/index.ts`. The
  **Form** tab is therefore a `Fieldset` of `Field` roots whose `validationMode` defaults to
  `"onSubmit"`, plus the submit edge an `Input`/`Field.Control` reports through `onSubmit`.
- **`Slider.Thumb` is not positioned by the core.** The application places it absolutely from the
  value it declared, unlike `ScrollArea.Thumb`, which the core positions along its track.
