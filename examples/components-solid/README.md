# QuickGUI Solid components

One window showing every compound component `@quickgui/solid` binds. Each is a *declaration*: the
Rust core owns filtering, clamping, snapping, virtualization, keyboard navigation, focus policy,
and every deadline, and reports what it decided back as an asynchronous payload.

- **Option sources** — `Select`, `Combobox`, and `Autocomplete`, each opening a native popover
  window the core renders from the declared `appearance`.
- **Range, ordering, and roving focus** — `Slider`, `NumberField`, `Splitter`, `Toolbar`, and
  `ToggleGroup`, all named by one `scope` per instance.
- **Virtual table** — a five-thousand-row `Table` that declares only the rows the core last
  reported as visible, with multiple selection and a sortable, resizable column.
- **Lazy tree** — a `Tree` whose one `pending` branch asks for its children exactly once through
  `onLoadChildren`, answered with a single validated `setChildren` splice.
- **Dates and times** — `DateField`, `TimeField`, and `Calendar` over ISO civil values with no time
  zone.
- **Menus** — an in-window `Menubar` whose menus are declared `PopoverMenu` surfaces, plus a
  `ContextMenu` opening at the exact secondary-click point.
- **Tabs, dialogs, and toasts** — `Tabs` with automatic activation, an in-window `Dialog` with the
  core's own focus trap, and a `Toast` queue whose timed dismissals report through `onDismiss`.

```console
cd examples/components-solid
bun run dev
```
