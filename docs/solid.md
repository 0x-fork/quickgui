# Solid 2 renderer

QuickGUI includes an experimental Bun host split into two unstyled packages:

- `@quickgui/native` owns the N-API boundary, application loop, per-window retained trees, binary
  mutation batches, window-routed event queue, and renderer-owned `Window` lifecycle.
- `@quickgui/solid` owns Solid 2 JSX compilation, fine-grained reactive updates, and the host
  components `View`, `Text`, `Button`, `Input`, `TextArea`, `Markdown`, `VirtualList`,
  `Popover`/`SystemPopover`, and the compound `Tooltip`, `Checkbox`, `Radio`, `RadioGroup`,
  `Switch`, `Tabs`, `Collapsible`, `Accordion`, `Field`, `Fieldset`, `Dialog`, and `AlertDialog`
  parts, plus the `createRenderer` adapter passed to a native `Window`.

The renderer does not use a webview or virtual DOM. Solid updates the affected retained native
nodes, and one binary batch crosses N-API before QuickGUI invalidates the WGPU window.

## Run an application

For application development, the preferred path is the [QuickGUI CLI](cli.md):

```console
quickgui dev
```

On macOS this runs a real signed `.app` with AppKit/Winit on the process main thread and the compiled
project TSX in a Bun Worker. Source changes build a candidate `.app` and replace the previous
process only after its first native window is ready. The CLI owns the Solid compiler setup, so
applications do not need a Bun preload or a special start command.

```tsx
import { app, Dialog, Window } from "@quickgui/native";
import { Button, Popover, SystemPopover, Text, View, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

function Counter() {
  const [count, setCount] = createSignal(0);

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        gap: 12,
      }}
    >
      <Text>Count: {count()}</Text>
      <Button onClick={() => setCount((value) => value + 1)}>Increment</Button>
    </View>
  );
}

function openMainWindow() {
  return new Window({
    title: "Counter",
    width: 480,
    height: 320,
    renderer: createRenderer(() => <Counter />),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});

await app.whenReady();
const mainWindow = openMainWindow();
```

The singleton `app` establishes the application context for its JavaScript Worker while the CLI
host owns the platform event loop. Applications depend on both packages directly: lifecycle,
platform APIs, and
`Window` come from `@quickgui/native`; Solid components and `createRenderer` come from
`@quickgui/solid`. Await `app.whenReady()` before constructing the first window; constructing one
earlier throws instead of silently staging it. The native application loop is ready when that
promise resolves, so the window begins opening immediately; application code does not call
`app.run()`. The renderer's initial mutation batch is committed in Rust before native creation, so
the first visible frame cannot race an empty Solid tree. Windows constructed later from event
handlers follow the same path:

```tsx
function openSettings() {
  new Window({
    title: "Settings",
    width: 520,
    height: 420,
    renderer: createRenderer(() => {
      const window = Window.getCurrentWindow();
      return <Settings close={() => window.close()} />;
    }),
  });
}
```

Closing a `Window` automatically disposes its Solid renderer root. With the default quit mode,
macOS keeps the application resident after its last window closes, while Windows and Linux end the
JavaScript host run. A macOS `reopen` listener must construct a replacement window and renderer
root when the user clicks the Dock icon.

`Window.getCurrentWindow()` returns the window whose renderer or native event callback is running.
Read it during component setup and retain the result for asynchronous work; it intentionally throws
outside window-bound rendering or event dispatch instead of guessing from focus or creation order.
The binding projects the same identity exposed by Rust's `ViewContext::window_handle` and
`EventContext::window_handle`.

Use the `Dialog` namespace for operating-system prompts. Passing a `Window` first attaches a native
sheet; omit it to present application-modal UI. Alert dialogs resolve with the zero-based index of
the selected button, and semantic roles preserve the platform's default and cancel keyboard
behavior:

```tsx
const response = await Dialog.showAlertDialog(mainWindow, {
  level: "warning",
  message: "Save changes before closing?",
  detail: "Your edits will be lost if you close without saving.",
  buttons: [
    { label: "Save", role: "default" },
    "Don't Save",
    { label: "Cancel", role: "cancel" },
  ],
});
```

This is a native alert sheet, separate from QuickGUI's caller-styled in-window `Dialog`
composition. `Dialog.showAlertDialog(options)` uses the same API without a parent window. The
runnable Solid example is in
[`examples/alert-dialog-solid`](../examples/alert-dialog-solid).

For Electron-style native file panels, call `Dialog.showOpenDialog` or `Dialog.showSaveDialog`.
The first `Window` argument is optional. Cancellation is explicit and choosing a save destination
does not write the file:

```tsx
const opened = await Dialog.showOpenDialog(mainWindow, {
  title: "Open notes",
  defaultPath: process.cwd(),
  filters: [
    { name: "Notes", extensions: ["md", "txt"] },
    { name: "All files", extensions: ["*"] },
  ],
  properties: ["openFile", "multiSelections"],
});

const saved = await Dialog.showSaveDialog({
  title: "Save report",
  defaultPath: `${process.cwd()}/report.txt`,
  filters: [{ name: "Text", extensions: ["txt"] }],
});

if (!opened.canceled) console.log(opened.filePaths);
if (!saved.canceled) console.log(saved.filePath);
```

The file APIs work on macOS, Windows, and Linux. QuickGUI retains its
AppKit panel backend on macOS and uses `rfd` elsewhere. Linux uses XDG Desktop Portal with Zenity
as fallback, so packaged applications need a GTK, GNOME, or KDE portal backend plus Zenity.
`buttonLabel` and hidden-file visibility currently apply only to the macOS backend. Selecting both
files and directories in one open dialog is also macOS-only; use separate calls for portable apps.
Filter-extension enforcement differs between native backends, so validate the returned save path.

The runnable version, including directory selection, is in
[`examples/file-dialog-solid`](../examples/file-dialog-solid).

`SystemPopover` uses compound `Root`, `Trigger`, and `Content` parts. `Root` is a logical coordinator
that renders no native element, `Trigger` stays in the owner window, and only `Content` mounts
through a separate Solid renderer in the native child window. Unmounting the content or root
disposes that renderer and closes the window. Placement resolves against the display work area, so
the surface can extend beyond its owner and flips or slides back on screen when needed:

```tsx
const [systemOpen, setSystemOpen] = createSignal(false);

<SystemPopover.Root open={systemOpen()} onOpenChange={setSystemOpen}>
  <SystemPopover.Trigger>Provider settings</SystemPopover.Trigger>
  <SystemPopover.Content
    width={420}
    height={280}
    placement="bottom-end"
    gap={8}
    viewportMargin={12}
  >
    <ProviderSettings close={() => setSystemOpen(false)} />
  </SystemPopover.Content>
</SystemPopover.Root>
```

Whenever controlled or uncontrolled state transitions from open to closed, the Rust core restores
focus to the trigger retained by the popover relation. Applications and renderer bindings do not
retain a trigger ref or call `focus()` when closing content explicitly.

Pressing an open `SystemPopover.Trigger` dismisses the active native popover and consumes that anchor
press. The trigger does not receive a later click against already-dismissed controlled state, so the
compound state settles closed instead of immediately reopening it.

For a normal popover inside the current window, use the same compound parts under `Popover.Root`.
It uses QuickGUI core's retained overlay plane and remains physically bounded by the owner window.
An enabled Escape or outside press invokes `onOpenChange(false, details)` on either root:

```tsx
const [open, setOpen] = createSignal(false);

<Popover.Root open={open()} onOpenChange={setOpen}>
  <Popover.Trigger>Toggle details</Popover.Trigger>
  <Popover.Content
    width={320}
    height={180}
    placement="bottom-start"
    gap={8}
    viewportMargin={12}
  >
    <ProviderDetails />
  </Popover.Content>
</Popover.Root>
```

The runnable [`popover-solid` example](../examples/popover-solid) places `SystemPopover` and
`Popover` side by side so their renderer ownership and window-edge behavior are visible.

### Base UI popover parts

`Popover.Content` above is QuickGUI's one-element shorthand. The full Base UI compound is also
available on the same root, and the core owns every part of it:

| Part | Declared props | Core guide |
| --- | --- | --- |
| `Popover.Root` | `open`/`defaultOpen`, `onOpenChange`, `modal`, `openOnHover`, `delay`, `closeDelay`, `dismissOnEscape`, `dismissOnPointerOutside`, `onPlacementChange`, and the positioning defaults below | [popovers](popovers.md) |
| `Popover.Trigger` | `openOnHover`, `delay`, `closeDelay` | [popovers](popovers.md) |
| `Popover.Portal` / `Popover.Positioner` | `side`, `align`, `sideOffset`, `alignOffset`, `collisionPadding`, `sticky`, `anchor` | [popovers](popovers.md) |
| `Popover.Backdrop`, `Popover.Popup`, `Popover.Arrow`, `Popover.Viewport`, `Popover.Title`, `Popover.Description`, `Popover.Close` | — | [popovers](popovers.md) |

```tsx
<Popover.Root open={open()} onOpenChange={setOpen} modal>
  <Popover.Trigger openOnHover delay={300} closeDelay={100}>Account</Popover.Trigger>
  <Popover.Positioner side="bottom" align="end" sideOffset={8} collisionPadding={12}>
    <Popover.Popup>
      <Popover.Arrow />
      <Popover.Title>Account</Popover.Title>
      <Popover.Viewport>
        <AccountSettings />
      </Popover.Viewport>
      <Popover.Close>Done</Popover.Close>
    </Popover.Popup>
  </Popover.Positioner>
</Popover.Root>
```

Every part except the trigger is mounted by the core only while the popover is open, so a closed
popover contributes no overlay, layout, paint, input, or accessibility node. The trigger is the one
part that stays mounted either way, which is why it carries the whole declaration: `Popover.Root`
and `Popover.Positioner` route their props onto it, and the core reports back through it.

A declared `side` and `align` are only a preference. The retained tree flips the side and re-aligns
the cross axis whenever the popup does not fit, and publishes the real answer during the paint it
was already performing. `usePopoverPlacement()` and `onPlacementChange` report it, which is how an
application styles from the real placement the way Base UI styles from `data-side` and `data-align`:

```tsx
const placement = usePopoverPlacement();

<Popover.Popup
  style={{
    maxHeight: placement().availableHeight,
    opacity: placement().anchorHidden ? 0 : 1,
  }}
/>
```

`anchor` accepts another node or one `{ x, y }` logical point, which is Base UI's virtual element.
`openOnHover` hands the open value to the core's own exact deadline — a hovered trigger opens after
`delay` and closes after `closeDelay` once neither the trigger nor a hoverable popup is hovered —
and the resulting open value comes back through `onOpenChange` with a `"hover"` reason.

### Tooltips

`Tooltip` is the composable counterpart of the framework-owned `tooltip` prop every native node
already accepts. It carries the same contract as the popover compound: the trigger is the part that
stays mounted, so it carries the declaration, and every deadline belongs to the core.

| Part | Declared props | Core guide |
| --- | --- | --- |
| `Tooltip.Provider` | `delay`, `closeDelay`, `timeout` | [input](input.md) |
| `Tooltip.Root` | `open`/`defaultOpen`, `onOpenChange`, `disabled`, `hoverable`, `trackCursorAxis`, `onPlacementChange`, positioning defaults | [input](input.md) |
| `Tooltip.Trigger` | `delay`, `closeDelay`, `closeOnClick`, `element` | [input](input.md) |
| `Tooltip.Portal` / `Tooltip.Positioner` | `side`, `align`, `sideOffset`, `collisionPadding` | [input](input.md) |
| `Tooltip.Popup`, `Tooltip.Arrow` | — | [input](input.md) |

```tsx
<Tooltip.Provider delay={600} closeDelay={200} timeout={400}>
  <Tooltip.Root trackCursorAxis="x">
    <Tooltip.Trigger>Save</Tooltip.Trigger>
    <Tooltip.Positioner side="top" sideOffset={7}>
      <Tooltip.Popup>
        <Text>Save the current draft</Text>
        <Tooltip.Arrow />
      </Tooltip.Popup>
    </Tooltip.Positioner>
  </Tooltip.Root>
</Tooltip.Provider>
```

One shared provider makes an adjacent trigger open instantly while the group stays warm; that warm
window is itself one exact deadline, so a settled group owns no task or timer. `useTooltipPlacement()`
reports the side and alignment the core really used. Unlike Base UI's DOM-less provider, QuickGUI's
`Tooltip.Provider` is one ordinary element, which is also where the group's deadlines are declared.

## Selection, tab, disclosure, and field parts

The unstyled Rust part descriptors are exposed as Base-UI-shaped compound components. Each part is
one ordinary native node that declares which core descriptor to rebuild; the Rust core keeps
ownership of part identity, roles, keyboard behavior, accessibility relationships, and whether an
inactive panel is mounted at all. JavaScript owns only the controlled value and the element tree,
so nothing about a component is decided by a synchronous callback across N-API.

| Component | Parts | Controlled props | Core guide |
| --- | --- | --- | --- |
| `Checkbox` | `Root`, `Indicator` | `checked` (`true`/`false`/`"indeterminate"`), `defaultChecked`, `onCheckedChange`, `readOnly`, `parent` with `childrenChecked` | [selection controls](selection-controls.md) |
| `Radio` | `Root`, `Indicator` | `value`, or standalone `checked`/`onCheckedChange`, `readOnly` | [selection controls](selection-controls.md) |
| `RadioGroup` | `Root` | `value`, `defaultValue`, `onValueChange`, `readOnly`, `required` | [selection controls](selection-controls.md) |
| `Switch` | `Root`, `Thumb` | `checked`, `defaultChecked`, `onCheckedChange`, `readOnly` | [selection controls](selection-controls.md) |
| `Tabs` | `Root`, `List`, `Tab`, `Indicator`, `Panel` | `value`, `defaultValue`, `onValueChange`, `orientation`, `activation`, `loop`, `keepMounted`, `onTabsStateChange`; `index` on a tab, `placement` on the indicator | [tabs](tabs.md) |
| `Collapsible` | `Root`, `Trigger`, `Panel` | `open`, `defaultOpen`, `onOpenChange`, `disabled`, `keepMounted` | [disclosures](disclosures.md) |
| `Accordion` | `Root`, `Item`, `Header`, `Trigger`, `Panel` | `value`, `defaultValue`, `onValueChange`, `multiple`, `headingLevel`, `keepMounted`, `disabled`; `index` on an item | [disclosures](disclosures.md) |
| `Field` | `Root`, `Item`, `Label`, `Control`, `Validity`, `Description`, `Error` | `disabled`, `invalid`, `required`, `touched`, `dirty`, `filled`, `validationMessage`, `validationMode`, `validationDebounceTime`, `onValidationChange` | [text and forms](text-and-forms.md) |
| `Fieldset` | `Root`, `Legend`, `Description`, `Control` | `disabled` | [text and forms](text-and-forms.md) |
| `Dialog`, `AlertDialog` | `Root`, `Trigger`, `Portal`, `Backdrop`, `Popup`, `Viewport`, `Title`, `Description`, `Close` | `open`, `defaultOpen`, `onOpenChange`, `dismissOnEscape`, `dismissOnBackdrop`, `enterDuration`, `exitDuration`, `onOpenChangeComplete` | [dialogs](dialogs.md) |

A `Tabs.Tab index` is how the core knows which way the selection travelled, so
`useTabsState().activationDirection` — Base UI's `data-activation-direction` — is the core's answer
rather than an index comparison in JavaScript. Declaring a `placement` on `Tabs.Indicator` keeps it
anchored to the tab that is really active and publishes that tab's laid-out box back as
`useTabsState().indicator`, measured during the paint QuickGUI was already performing.

A `Field.Root validationMode` — `"onSubmit"` (the default), `"onBlur"`, or `"onChange"` — and a
bounded `validationDebounceTime` are answered by the core, which reports which triggers validate and
how long it waits before each one through `onValidationChange`. `Field.Item` is the structural row
and `Field.Validity` the live readout.

A `Checkbox.Root parent` inside a `CheckboxGroup.Root` is the group's derived parent checkbox;
standalone, `childrenChecked` declares the children's booleans and the core folds them into on,
mixed, or off with no registry at all. `readOnly` is the web's `readonly` rather than `disabled`: the
control keeps its place in the Tab sequence and its value in the accessible name while refusing
changes.

A dialog's `exitDuration` is the exact deadline the core holds the surface mounted for so the
application's own exit transition can finish; `onOpenChangeComplete` reports when it did.

```tsx
const [tab, setTab] = createSignal("overview");

<Tabs.Root value={tab()} onValueChange={setTab} activation="automatic">
  <Tabs.List style={{ display: "flex", gap: 4 }}>
    <Tabs.Tab value="overview">
      Overview
      <Tabs.Indicator style={{ height: 2, backgroundColor: "#2563eb" }} />
    </Tabs.Tab>
    <Tabs.Tab value="usage" disabled>
      Usage
    </Tabs.Tab>
  </Tabs.List>
  <Tabs.Panel value="overview">
    <Text>Overview content</Text>
  </Tabs.Panel>
  <Tabs.Panel value="usage">
    <Text>Usage content</Text>
  </Tabs.Panel>
</Tabs.Root>
```

`Tabs.Panel` is not mounted at all while its tab is inactive, so it contributes no layout, paint,
input, or accessibility node. Declare `keepMounted` on `Tabs.Root` to retain inactive panels as
`display: none` instead. `Collapsible.Panel` and `Accordion.Panel` follow the same core policy.
`Tabs.Indicator` mounts only for the active tab and uses the enclosing `Tabs.Tab` value, or an
explicit `value` prop when positioned in the list instead.

Selection roots are controlled. `onCheckedChange` and `onValueChange` receive the next value and
the originating `QuickGuiEvent`; the core supplies the click, Space activation, arrow navigation,
focus, cursor, window-drag exclusion, and toggle/selected accessibility state:

```tsx
const [notify, setNotify] = createSignal<boolean | "indeterminate">("indeterminate");

<Checkbox.Root checked={notify()} onCheckedChange={setNotify}>
  <Checkbox.Indicator>{notify() === true ? "✓" : "–"}</Checkbox.Indicator>
  <Text>Email me about releases</Text>
</Checkbox.Root>

<RadioGroup.Root value={theme()} onValueChange={setTheme}>
  <Radio.Root value="light">
    <Radio.Indicator />
    <Text>Light</Text>
  </Radio.Root>
  <Radio.Root value="dark">
    <Radio.Indicator />
    <Text>Dark</Text>
  </Radio.Root>
</RadioGroup.Root>
```

A field's parts all derive their identity from one bounded scope key the renderer allocates, so the
core resolves label, description, error, and control relationships without a JavaScript registry.
`Field.Control` *is* the control, not a wrapper around one, because the core part owns that
element's identity; `element` selects which native element it renders and defaults to `input`:

```tsx
<Fieldset.Root disabled={saving()}>
  <Fieldset.Legend>Account</Fieldset.Legend>
  <Field.Root invalid={!valid()} required validationMessage="Enter an address">
    <Field.Label>Email</Field.Label>
    <Field.Control value={email()} placeholder="you@example.com" onInput={update} />
    <Field.Description>We never share it.</Field.Description>
    <Field.Error>Enter an address</Field.Error>
  </Field.Root>
</Fieldset.Root>
```

A `Field.Root` inside a `Fieldset.Root` inherits the group's disabled state. `Field.Label` forwards
its clicks to the control it names; add `passive` for button-like controls that should be named but
not activated. `Field.Error` is removed from layout by the core while the controlled field is
valid.

Scope keys and item values are bounded to 256 bytes and the renderer throws a `TypeError` rather
than sending an oversized declaration across N-API.

## Declared popover and context menus

`PopoverMenu` and `ContextMenu` bind the Rust core's [`PopoverMenu` model](popovers.md) and
[cursor-point context adapter](context-menus.md). Menu rows are *declared data*, not JSX children:
one bounded JSON model crosses N-API ahead of time so the core can decide highlighting, typeahead,
checkbox and radio policy, submenu opening, work-area placement, and native surface lifetime without
ever waiting on JavaScript. The core also renders the rows, because the cursor-point surface is a
separate native window that a hosted retained tree cannot reach synchronously.

| Component | Parts | Declared props | Core guide |
| --- | --- | --- | --- |
| `PopoverMenu` | `Root`, `Trigger`, `Popup` | `items`, `appearance`, `open`, `defaultOpen`, `onOpenChange`, `onSelect`, `placement`, `gap`, `viewportMargin`, `dismissOnEscape`, `dismissOnPointerOutside` | [popovers](popovers.md) |
| `ContextMenu` | `Root`, `Trigger` | `items`, `appearance`, `onSelect` | [context menus](context-menus.md) |

```tsx
const [open, setOpen] = createSignal(false);
const [sidebar, setSidebar] = createSignal(true);

<PopoverMenu.Root
  open={open()}
  onOpenChange={setOpen}
  items={[
    { type: "group", label: "File" },
    { id: "open", label: "Open…", shortcut: "⌘O" },
    { type: "separator" },
    { type: "checkbox", id: "sidebar", label: "Show sidebar", checked: sidebar() },
    { id: "recent", label: "Recent", items: [{ id: "one", label: "quickgui.md" }] },
  ]}
  appearance={{ width: 240, background: "#101014", highlightBackground: "#2563eb" }}
  onSelect={(item) => {
    if (item.id === "sidebar") setSidebar(item.checked === true);
  }}
>
  <PopoverMenu.Trigger>Actions</PopoverMenu.Trigger>
  <PopoverMenu.Popup style={{ backgroundColor: "#101014", borderRadius: 8 }} />
</PopoverMenu.Root>
```

One entry is an `action` by default, or a `submenu` when it declares nested `items`. `type` selects
`"action"`, `"checkbox"`, `"radio"`, `"submenu"`, `"separator"`, or `"group"`. Every interactive
entry needs a stable `id`; that id — never a positional index — is what `onSelect` reports, so an
atomic model replacement cannot misroute a command. A checkbox reports the value the core computed
in `checked`; a radio reports `checked: true`; a submenu row reports `submenu: true` instead of a
command. Entries that cannot form a core item (no `id`, no `label`, or an identifier over 256 bytes)
are skipped rather than discarding the whole menu, and a declaration over 512 KiB throws a
`RangeError` in JavaScript before it can cross N-API.

`appearance` carries the structural measurements the core needs to size its own surface —`width`,
`itemHeight`, `separatorHeight`, `groupLabelHeight`, `verticalPadding`, `padding`, `fontSize`,
`radius` — plus the row colors (`background`, `color`, `highlightBackground`, `highlightColor`,
`mutedColor`) it paints. Colors accept the same CSS-like values as every other property and are
packed before they cross N-API. `loop: false` stops arrow navigation from wrapping.

Up/Down, Home/End, Enter/Space, Left/Escape, alphanumeric typeahead, hover highlighting, and the
checkbox/radio close policy all come from the core's own contextual key bindings, which the binding
installs on the application once. `PopoverMenu.Popup` is an in-window anchored surface, so it is
mounted only while the menu is open and while a trigger exists to anchor it, and it declares
`has-popup: menu`, `expanded`, and a validated `controls` relationship on the trigger.

```tsx
<ContextMenu.Root
  items={[
    { id: "cut", label: "Cut", shortcut: "⌘X" },
    { id: "copy", label: "Copy", shortcut: "⌘C" },
    { type: "separator" },
    { id: "paste", label: "Paste", shortcut: "⌘V", disabled: !canPaste() },
  ]}
  onSelect={(item) => run(item.id)}
>
  <ContextMenu.Trigger style={{ flex: 1 }}>
    <Text>Right-click anywhere</Text>
  </ContextMenu.Trigger>
</ContextMenu.Root>
```

`ContextMenu.Trigger` opens the core's separate cursor-point surface at the exact secondary-click
point, so the menu can extend past the owner window while native work-area constraints flip and
slide it. Submenu hover intent, diagonal safe corridors, child-first teardown, and exact close
synchronization are all core behavior. A window owns exactly one context-menu state, which matches
the native invariant that opening a menu anywhere replaces the one already open.

Because the surface is core-rendered, a context menu has no JavaScript-rendered rows and therefore
no `Popup` part; style it through `appearance` instead.

## In-window dialogs

`Dialog` and `AlertDialog` compose the Rust core's caller-styled modal surface, which is separate
from the operating-system alert and file panels in the `Dialog` namespace of `@quickgui/native`.
`Dialog.Root` is a logical coordinator that creates no native element, and `Dialog.Portal` is the
viewport overlay root the core mounts only while the dialog is open:

```tsx
const [open, setOpen] = createSignal(false);

<AlertDialog.Root open={open()} onOpenChange={setOpen}>
  <AlertDialog.Trigger>Delete project</AlertDialog.Trigger>
  <AlertDialog.Portal>
    <AlertDialog.Backdrop style={{ backgroundColor: "#0f172a80" }} />
    <AlertDialog.Popup>
      <AlertDialog.Title>Delete project?</AlertDialog.Title>
      <AlertDialog.Description>This cannot be undone.</AlertDialog.Description>
      <AlertDialog.Close aria-label="Cancel">Cancel</AlertDialog.Close>
    </AlertDialog.Popup>
  </AlertDialog.Portal>
</AlertDialog.Root>
```

The dismissal policy is declared ahead of time through `dismissOnEscape` and `dismissOnBackdrop`
rather than answered by a callback; `AlertDialog` keeps Escape and blocks backdrop dismissal by
default. `onOpenChange` reports `"trigger-press"`, `"close-press"`, or `"dismiss"`. Explicit
initial-focus and focus-restoration targets are not bridged yet; the core's focus trap and
`restore_previous_focus` default apply.

## CSS Grid, transitions, images, and shaders

`display: "grid"` selects the Rust core's retained Taffy grid. Track lists accept the CSS string,
an array of tracks, or a plain count of equal `1fr` tracks, and item placement accepts the CSS
`grid-column` / `grid-row` shorthands:

```tsx
<View
  style={{
    display: "grid",
    gridTemplateColumns: ["200px", "1fr", "minmax(120px, 2fr)"],
    gridTemplateRows: "repeat(3, auto)",
    gridAutoFlow: "row dense",
    gap: 12,
  }}
>
  <View style={{ gridColumn: "1 / span 2", gridRow: 1 }} />
</View>
```

Tracks understand `auto`, `min-content`, `max-content`, `<n>px`, `<n>%`, `<n>fr`,
`minmax(<px>, <n>fr)`, `fit-content(<px>)`, and `repeat(<count>, <tracks>)`. A track the core cannot
form becomes `auto` rather than discarding the template, and one template materializes at most 512
tracks. `grid-column: 2 / span 3` is sent as the equivalent lines `2` and `5`, because that is the
placement the core exposes. `gridAutoColumns` and `gridAutoRows` have no core equivalent yet.

`transition` accepts the CSS shorthand, a plain duration in milliseconds, or an object:

```tsx
<Button
  style={{
    transition: { properties: ["background-color", "opacity"], duration: "150ms", easing: "ease-out" },
    hoverBackgroundColor: "#2563eb",
  }}
/>
```

The core transitions `background-color`, `border-color`, `border-width`, `border-radius`, `color`,
`box-shadow`, and `opacity`; `all` selects every one. Declaring any other property throws a
`TypeError` instead of silently animating nothing. Easing selects one of the core's own curves —
`linear`, `ease-in`, `ease-out`, and `ease-in-out` (`ease` is an alias). `transitionMaxFps` caps the
repaint cadence while the transition runs. Transitions are paint-only in the Rust core, so there is
no `delay`: a non-zero delay throws rather than being ignored.

| Element | Declared props |
| --- | --- |
| `Image` | `source` (path, `file://`, or base64 `data:` URL), `fit` (`fill`, `contain`, `cover`, `scale-down`, `none`) |
| `Shader` | `source` (bounded WGSL), `shaderParameters` (up to sixteen floats in four vectors) |

```tsx
<Image source="/assets/hero.png" fit="cover" style={{ width: 320, height: 180 }} />

<Shader
  source={`fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {
    return vec4<f32>(input.uv, quickgui_parameters[0].x, 1.0);
  }`}
  shaderParameters={[[0.5, 0, 0, 1]]}
  style={{ width: 300, height: 150 }}
/>
```

A path stays a lazy core `ImageResource` so decoding runs on the core's bounded worker pool; a
`data:` URL is decoded once and retained until its declaration changes, and an animated format keeps
its frames and repeat policy inside the core decoder. A source the core cannot decode contributes a
hidden element rather than failing the tree. Explicit animated playback controls and loading or
fallback children are not bound yet because the core exposes no play/pause or load-state API.

Shader WGSL is validated by the core before it reaches the GPU: it must declare exactly the
`quickgui_fragment` entry point, no bind-group resources, and no pipeline overrides. Invalid WGSL
renders nothing instead of panicking.

## Range and feedback parts

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `Progress` | `Root`, `Track`, `Indicator`, `Label`, `Value` | `value`, `max`, `indeterminate`, `valueText`, `format` | `onStatusChange(state, event)` and `useGaugeState()` | [range and feedback](range-and-feedback.md) |
| `Meter` | `Root`, `Track`, `Indicator`, `Label`, `Value` | `value`, `min`, `max`, `low`, `high`, `optimum`, `format` | the same | [range and feedback](range-and-feedback.md) |
| `Toggle` | `Root`, `Indicator` | `pressed`, `defaultPressed`, `onPressedChange` | `onPressedChange` | [toolbar and toast](toolbar-and-toast.md) |

```tsx
<Progress.Root value={done()} max={total()} valueText={`${done()} of ${total()} files`}>
  <Progress.Indicator style={{ width: `${(done() / total()) * 100}%` }} />
</Progress.Root>

<Toggle.Root pressed={bold()} onPressedChange={setBold}>
  <Text>B</Text>
</Toggle.Root>
```

A progress root without a `value`, or with `indeterminate`, declares the core's indeterminate value
range. `valueText` is never rendered; assistive technology prefers it over the raw number. A meter
reports a level inside a known range rather than task progress, and `low`/`high`/`optimum` let the
application color the gauge without the framework inventing thresholds. A toggle is a button that
stays pressed, not a checkbox, so the core exposes it as a toggle button.

Declaring a `format` — `"percent"` or `"fraction"` — asks the core to produce the text a
`Progress.Value` renders, and `useGaugeState()` reports it alongside the core's own derived
`status`, which is Base UI's `data-progressing`, `data-complete`, and `data-indeterminate`:

```tsx
function Upload() {
  const gauge = useGaugeState();
  return (
    <Progress.Root value={done()} max={total()} format="fraction">
      <Progress.Label><Text>Uploading</Text></Progress.Label>
      <Progress.Value><Text>{gauge().displayValue}</Text></Progress.Value>
      <Progress.Track>
        <Progress.Indicator style={{ width: `${(gauge().completion ?? 0) * 100}%` }} />
      </Progress.Track>
    </Progress.Root>
  );
}
```

## Range, ordering, and roving-focus components

Every part of one instance carries the same `scope`, which is how the Rust binding finds the one
retained state a thumb, handle, or item belongs to. The core owns clamping, step snapping, thumb
ordering, splitter size conservation, wrapping arrow navigation, disabled-item skipping, and the
single roving Tab stop; JavaScript declares the state and receives whatever the core decided as one
asynchronous payload. Nothing in this table is re-implemented in TypeScript.

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `Slider` | `Root`, `Label`, `Value`, `Control`, `Track`, `Range`/`Indicator`, `Thumb` | `scope`, `value`/`defaultValue` (one entry per thumb), `min`, `max`, `step`, `largeStep`, `minStepsBetweenValues`, `thumbAlignment`, `format`, `orientation`, `disabled` | `onValueChange(values, event)`, `onValueCommitted(values, event)`, `useSliderState()` | [range and feedback](range-and-feedback.md) |
| `Splitter` | `Root`, `Pane`, `Handle` | `scope`, `value`/`defaultValue` (one size per pane), `panes` (`min`, `collapsible`), `step`, `orientation` | `onSizesChange(sizes, event)` | [range and feedback](range-and-feedback.md) |
| `Toolbar` | `Root`, `Item`, `Button`, `Link`, `Input`, `Group`, `Separator` | `scope`, `items` (`value`, `disabled`, `focusableWhenDisabled`), `active`/`defaultActive`, `orientation`, `loopFocus` | `onActiveChange(active, event)` | [toolbar and toast](toolbar-and-toast.md) |
| `ToggleGroup` | `Root`, `Item` | `scope`, `items`, `value`/`defaultValue`, `variant` (`"single"`/`"multiple"`), `active`, `orientation`, `loopFocus` | `onValueChange(values, event)` | [toolbar and toast](toolbar-and-toast.md) |

A `Slider.Thumb` and a `Splitter.Pane` or `Splitter.Handle` name their position with `itemIndex`; a
`Toolbar.Item` and a `ToggleGroup.Item` name their entry with `partValue`, matching a `value` in the
group's `items`.

```tsx
<Slider.Root
  scope="volume"
  value={volume()}
  min={0}
  max={100}
  step={5}
  onValueChange={([next]) => setVolume([next])}
>
  <Slider.Track scope="volume">
    <Slider.Range scope="volume" />
  </Slider.Track>
  <Slider.Thumb scope="volume" itemIndex={0} />
</Slider.Root>

<Toolbar.Root scope="actions" items={[{ value: "cut" }, { value: "copy" }]}>
  <Toolbar.Item scope="actions" partValue="cut">Cut</Toolbar.Item>
  <Toolbar.Item scope="actions" partValue="copy">Copy</Toolbar.Item>
</Toolbar.Root>
```

A single-thumb slider answers arrows, Page keys, Home, and End on its root; a range slider answers
them on the focused thumb, so the thumb the user sees is the one that moves. `Slider.Track` carries
the core's captured pointer arithmetic, which uses the track's own laid-out size as the core
measured it — the binding never re-derives geometry that layout already decided.

`Slider.Value` renders whatever the declared `format` produced, and `useSliderState()` reports it
together with the core's own `dragging` flag — Base UI's `data-dragging` — so a thumb can be styled
while a captured drag is in flight. `onValueCommitted` is the core's own pointer boundary: it fires
on the frame the gesture released, never on a debounce invented in JavaScript.

A `Toolbar.Button`, `Toolbar.Link`, and `Toolbar.Input` are the same roving-focus item with the role
the core projects for each; `Toolbar.Group` and `Toolbar.Separator` are structural and take the
toolbar's own axis. An item declared `focusableWhenDisabled` keeps its place in the Tab sequence
while disabled, so a keyboard user can still discover the command exists; arrow navigation still
skips it, and it still refuses pointer focus.

A change never overwrites a controlled declaration on its own: the core keeps the value it decided
until the application commits the matching prop, and each declared instance reports independently,
so two sliders in one window never disturb each other. `componentChangeFromEvent(event)` decodes a
raw `componentchange` payload when an application wants to handle it directly. A malformed
declaration is bounded rather than fatal: unparsable JSON declares no values, duplicate item values
keep the first occurrence, and a list past `MAX_COMPONENT_VALUES` (64) or `MAX_COMPONENT_ITEMS`
(256) is truncated instead of reaching the core.

## Option sources: select, combobox, and autocomplete

`Select`, `Combobox`, and `Autocomplete` each own a native popover window of their own. JavaScript
declares the option source, the controlled value, and one bounded `appearance` block; the core
opens the window, filters, moves the highlight, answers typeahead, places the surface, and paints
every row from that appearance — exactly the way a declared `PopoverMenu` paints its rows. No row
can wait on the hosted runtime while the core is deciding what a keystroke means.

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `Select` | `Root`, `Option` | `scope`, `items` or child `Option` nodes, `value`/`defaultValue`, `appearance`, `filterMode`, `ariaLabel`, `disabled` | `onValueChange(value, event)`, `onOpenChange(open, event)`, `onCommit(details, event)` | [select and autocomplete](select-and-autocomplete.md) |
| `Combobox` | `Root`, `Option` | the same, plus `inputValue` and `placeholder` | the same, plus `onInputValueChange(value, event)` | [select and autocomplete](select-and-autocomplete.md) |
| `Autocomplete` | `Root`, `Option` | `scope`, `items`, `inputValue`, `placeholder`, `appearance`, `filterMode` | `onInputValueChange(value, event)`, `onOpenChange`, `onCommit` | [select and autocomplete](select-and-autocomplete.md) |

`Select.Root` renders the trigger; `Combobox.Root` and `Autocomplete.Root` render the input. The
options are either one bounded `items` array or child `Option` nodes, which contribute no element of
their own:

```tsx
<Select.Root
  scope="theme"
  ariaLabel="Theme"
  value={theme()}
  items={[
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark", detail: "⌘D" },
  ]}
  appearance={{ width: 240, rowHeight: 28, highlightBackground: "#2f6feb" }}
  onValueChange={setTheme}
>
  <Text>{theme() ?? "Choose a theme"}</Text>
</Select.Root>

<Combobox.Root scope="fruit" value={fruit()} onValueChange={setFruit}>
  <Combobox.Option partValue="apple" label="Apple" group="Common" />
  <Combobox.Option partValue="banana" label="Banana" />
</Combobox.Root>
```

`appearance` carries only geometry and paint: `width`, `rowHeight`, `maxVisibleRows`, `anchorGap`,
`fontSize`, `radius`, `padding`, `verticalPadding`, `background`, `color`, `highlightBackground`,
`highlightColor`, `selectedBackground`, and `mutedColor`. `filterMode` selects the core's own
bounded fuzzy matcher (`"fuzzy"`, the default) or no filtering at all (`"none"`) for a source an
application or service already filtered.

`inputValue` seeds the core's retained editing text; every edit after that belongs to the core,
which reports the exact value it holds. `onCommit` is an edge, not a value: committing the same
option twice reports twice while the controlled value never moves.

Because the core's replacement mutators close a live native popover — something a render pass
cannot do — the binding rebuilds a picker's retained state only when the declaration itself changes
and the popover is closed. A declaration committed while the surface is open is applied as soon as
the core closes it, which is the frame the close already schedules.

## Virtual tables and trees

`Table` and `Tree` are on-demand: the rows JavaScript declares are the rows the core last reported
as visible, so a million-row table declares only the window on screen. Column widths, display
order, sort direction, selection ranges, expansion, the lazy-children request, and the inline-edit
lifetime all live in the core.

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `Table` | `Root`, `Header`, `Row`, `Cell` | `scope`, `columns`, `rowCount`, `rowHeight`, `headerHeight`, `selectionMode`, `selection`, `sort`, `editing` | `onVisibleRangeChange`, `onSelectionChange`, `onSortChange`, `onActiveCellChange`, `onColumnResize`, `onColumnReorder`, `onEditEnd`, `onActivate` | [data collections](data-collections.md) |
| `Tree` | `Root`, `Row` | `scope`, `nodes`, `expanded`/`defaultExpanded`, `value`, `rowHeight`, `loadingLabel`, `disclosure`, `setChildren` | `onVisibleRangeChange`, `onExpandedChange`, `onValueChange`, `onLoadChildren`, `onActivate` | [data collections](data-collections.md) |

```tsx
const [range, setRange] = createSignal({ start: 0, end: 0 });

<Table.Root
  scope="files"
  rowCount={rows().length}
  columns={[
    { id: "name", label: "Name", width: 220, sortable: true },
    { id: "size", label: "Size", track: "1fr", align: "end" },
  ]}
  selectionMode="multiple"
  onVisibleRangeChange={setRange}
  onSelectionChange={setSelection}
>
  <Table.Header column="name"><Text>Name</Text></Table.Header>
  <Table.Header column="size"><Text>Size</Text></Table.Header>
  <For each={visible(range())}>
    {(row, index) => (
      <Table.Row index={range().start + index()}>
        <Table.Cell column="name"><Text>{row.name}</Text></Table.Cell>
        <Table.Cell column="size"><Text>{row.size}</Text></Table.Cell>
      </Table.Row>
    )}
  </For>
</Table.Root>
```

A declared `Header`, `Row`, or `Cell` is content, not identity: the core assigns the exact grid,
tree-item, and active-descendant identity, appends the resize handle it owns, and attaches the row
and cell interaction itself, so these nodes register no listener of their own. Put an interactive
control inside a cell as an ordinary child node instead.

`selection` is a list of inclusive `[start, end]` row ranges, which is exactly the shape the core
retains and reports. `editing` opens the inline editor over one cell; the core reports the end of
that edit — with its own commit decision — through `onEditEnd`.

A `Tree` node source declares `id`, `label`, `disabled`, `pending`, and nested `children`. A
`pending` branch asks for its children exactly once, through `onLoadChildren`; supplying them is a
declaration too:

```tsx
<Tree.Root
  scope="explorer"
  nodes={nodes()}
  expanded={expanded()}
  setChildren={loaded()}
  onExpandedChange={setExpanded}
  onLoadChildren={(id) => void fetchChildren(id).then((children) => setLoaded({ id, children }))}
>
  <For each={visibleNodes()}>
    {(node) => <Tree.Row nodeId={node.id}><Text>{node.label}</Text></Tree.Row>}
  </For>
</Tree.Root>
```

The core hands the binding a behavior-only disclosure control for every branch; `disclosure`
decides whether it is mounted before (`"leading"`, the default), after (`"trailing"`), or not at
all (`"none"`) inside the declared row.

## Number fields, date and time fields, month grids, menubars, and toasts

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `NumberField` | `Root`, `Group`, `Input`, `Increment`, `Decrement`, `ScrubArea`, `ScrubAreaCursor` | `scope`, `value`/`defaultValue`, `min`, `max`, `step`, `smallStep`, `largeStep`, `precision`, `snapOnStep`, `allowWheelScrub`, `readOnly`, `required`, `scrubDirection`, `scrubSensitivity`, `disabled` | `onValueChange(value, valid, event)`, `onValueCommitted(value, event)`, `useNumberFieldState()`, `onCommit(details, event)` on the input | [range and feedback](range-and-feedback.md) |
| `DateField` | `Root`, `Segment` | `scope`, `value`/`defaultValue`, `min`, `max`, `format` (`"ymd"`/`"dmy"`/`"mdy"`), `disabled` | `onValueChange(value, event)` | [date and time](date-and-time.md) |
| `TimeField` | `Root`, `Segment` | `scope`, `value`/`defaultValue`, `min`, `max`, `hour12`, `showSeconds`, `disabled` | `onValueChange(value, event)` | [date and time](date-and-time.md) |
| `Calendar` | `Root`, `Week`, `Day` | `scope`, `value`/`defaultValue`, `min`, `max`, `firstWeekday`, `disabled` | `onValueChange`, `onFocusChange(day, event)`, `onMonthChange(month, event)` | [date and time](date-and-time.md) |
| `Menubar` | `Root`, `Item` | `scope`, `count`, `open`/`defaultOpen`, `disabled` | `onOpenChange(index, event)`, `onActiveChange(index, event)` | [menubar](menubar.md) |
| `Toast` | `Provider`, `Portal`, `Viewport`, `Positioner`, `Root`, `Content`, `Title`, `Description`, `Action`, `Close` | `timeout`, `limit`, `expanded`, `swipeDirection`, `pitch` on the provider; `toasts` on the viewport | `onDismiss(ids, event)`, `onStackChange(stack, event)`, `useToastManager()` | [toolbar and toast](toolbar-and-toast.md) |

Civil values are ISO strings with no time zone: `YYYY-MM-DD` for a date or a calendar day,
`HH:MM` or `HH:MM:SS` for a time. The core owns segment arithmetic, digit entry, leap years, month
and year movement, and each field's validity.

```tsx
<NumberField.Root scope="qty" value={quantity()} min={0} max={99} step={1} onValueChange={setQuantity}>
  <NumberField.Decrement scope="qty"><Text>−</Text></NumberField.Decrement>
  <NumberField.Input scope="qty" onCommit={({ value }) => setQuantity(value as number)} />
  <NumberField.Increment scope="qty"><Text>+</Text></NumberField.Increment>
</NumberField.Root>

<DateField.Root scope="due" value={due()} format="mdy" onValueChange={setDue}>
  <DateField.Segment scope="due" segment="month" />
  <DateField.Segment scope="due" segment="day" />
  <DateField.Segment scope="due" segment="year" />
</DateField.Root>
```

Holding a `NumberField` stepper repeats on the core's own exact deadlines; the binding arms the
repeat on press, releases it on lift, and owns no timer of its own. Return commits: the core clamps
into range and reformats before reporting, and it refuses to submit a control whose text does not
parse into range. `readOnly` is the web's `readonly` rather than `disabled` — the control keeps its
place in the Tab sequence and its value in the accessible name while refusing every change.

A `NumberField.ScrubArea` turns a captured drag into whole steps at the declared
`scrubSensitivity`, keeping the unconverted remainder for the gesture so a slow drag moves one step
at a time. `useNumberFieldState().scrubbing` is the core's own `data-scrubbing`, which is what a
caller-drawn `NumberField.ScrubAreaCursor` styles from. Alt takes `smallStep` and Shift takes
`largeStep` on the keyboard, the wheel, and the scrub alike.

Pushing a toast is adding an entry to the declared `toasts` list, and dropping one dismisses it.
The core owns the queue bound, live-region politeness, focused-Escape dismissal, and the exact
auto-dismiss deadline, and it reports every dismissal — including the timed ones — through
`onDismiss`:

```tsx
<Toast.Viewport scope="toasts" toasts={toasts()} onDismiss={(ids) => setToasts((queue) => queue.filter((toast) => !ids.includes(toast.id)))}>
  <For each={toasts()}>
    {(toast) => (
      <Toast.Root scope="toasts" toastId={toast.id}>
        <Toast.Title scope="toasts" toastId={toast.id}><Text>{toast.title}</Text></Toast.Title>
        <Toast.Close scope="toasts" toastId={toast.id}><Text>×</Text></Toast.Close>
      </Toast.Root>
    )}
  </For>
</Toast.Viewport>
```

`Toast.Provider` owns the declared queue and Base UI's provider props: the inherited auto-dismiss
`timeout`, the visible stack `limit` (three by default, which flags older toasts without silencing
them), the `expanded` stack, the `swipeDirection`, and the stack `pitch` the core turns into each
toast's own offset. `useToastManager()` is the Base UI-shaped API over that declaration:

```tsx
function Notices() {
  const toasts = useToastManager();
  return (
    <Toast.Viewport>
      <For each={toasts.stack()}>
        {(entry) => (
          <Toast.Positioner toastId={entry.id} style={{ top: entry.offset }}>
            <Toast.Root toastId={entry.id} style={{ opacity: entry.limited ? 0.6 : 1 }}>
              <Toast.Content toastId={entry.id}>
                <Toast.Title toastId={entry.id}><Text>{entry.type}</Text></Toast.Title>
                <Toast.Close toastId={entry.id}><Text>×</Text></Toast.Close>
              </Toast.Content>
            </Toast.Root>
          </Toast.Positioner>
        )}
      </For>
    </Toast.Viewport>
  );
}
```

`add`, `update`, `close`, and `closeAll` change the declared list; `promise` queues a persistent
`"loading"` toast and turns it into its result while keeping the same identity and stack position,
because QuickGUI owns no future and the application drives both halves from the task it already
spawned. Everything the core decided — the stack index, the `limited` and `expanded` flags, each
toast's `offset`, and the live swipe displacement — comes back through `stack()`.

A `Menubar` owns which menu is open and which one holds the bar's single Tab stop; each menu's
surface is an ordinary declared `PopoverMenu` anchored to the matching `Menubar.Item`.

Every declaration on this page is bounded exactly as the Rust binding bounds it: an option source
past `MAX_OPTIONS_JSON_BYTES` (512 KiB) or a column, node, selection, or toast declaration past
`MAX_COLLECTION_JSON_BYTES` (2 MiB) is refused at the boundary instead of silently truncated, and
malformed JSON declares nothing at all rather than reaching a core constructor that would panic on
it. Duplicate option values, duplicate column identifiers, and duplicate node identifiers keep
their first occurrence; anything past `MAX_DECLARED_OPTIONS` (4096), `MAX_TABLE_COLUMNS` (512),
`MAX_DECLARED_TREE_NODES` (65 536), `MAX_TOASTS` (8), or `MAX_MENUBAR_MENUS` (64) is dropped.

`componentChangeFromEvent(event)` and `commitFromEvent(event)` decode a raw payload when an
application wants to handle one directly.

## Separators, avatars, checkbox groups, and preview cards

Every component in this section and the next declares the Rust core's own
[Base UI parity parts](base-ui-components.md). The compound names and prop names follow Base UI
wherever the core supports them; each one allocates a scope internally, so the parts of one
instance find each other with no `scope` to repeat and no registry.

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `Separator` | `Root` | `orientation` (`"horizontal"`/`"vertical"`) | — | [Base UI components](base-ui-components.md) |
| `Avatar` | `Root`, `Image`, `Fallback` | `ariaLabel` on the root, `src` on the image, `delay` (ms) on the fallback | `onLoadingStatusChange(status, event)` | [Base UI components](base-ui-components.md) |
| `CheckboxGroup` | `Root` plus `Checkbox.Root value` / `Checkbox.Root parent` children | `allValues`, `value`/`defaultValue`, `disabled` | `onValueChange(values, event)` | [Base UI components](base-ui-components.md) |
| `PreviewCard` | `Root`, `Trigger`, `Portal`, `Backdrop`, `Positioner`, `Popup`, `Arrow` | `open`/`defaultOpen`, `placement`, `gap`, `viewportMargin` on the root; `delay` and `closeDelay` (ms) on the trigger | `onOpenChange(open, event)` | [Base UI components](base-ui-components.md) |

A separator retains nothing at all: it is one node that gains the Separator role and an
orientation, and it never joins the Tab sequence.

An avatar's load status is application-reported in the core, so the binding drives it from the
declared image source's own load outcome and reports every transition. The core decides which of
the image and the fallback is mounted, so an avatar is announced exactly once however it renders,
and `Avatar.Fallback delay` is the exact deadline that keeps a fast decode from flashing initials.

```tsx
<Avatar.Root ariaLabel="Ada Lovelace" onLoadingStatusChange={setStatus}>
  <Avatar.Image src="./ada.png" />
  <Avatar.Fallback delay={120}><Text>AL</Text></Avatar.Fallback>
</Avatar.Root>
```

Inside a `CheckboxGroup.Root`, an ordinary `Checkbox.Root` becomes a member of the group:
`value` names the declared value it toggles and `parent` makes it the group's parent checkbox,
whose on/mixed/off state is derived from the children rather than retained separately. The core
owns the click behavior for both, so neither declares an `onClick` of its own.

```tsx
<CheckboxGroup.Root allValues={["red", "green", "blue"]} value={colors()} onValueChange={setColors}>
  <Checkbox.Root parent><Text>All colours</Text></Checkbox.Root>
  <For each={["red", "green", "blue"]}>
    {(value) => (
      <Checkbox.Root value={value}>
        <Text>{value}</Text>
        <Checkbox.Indicator />
      </Checkbox.Root>
    )}
  </For>
</CheckboxGroup.Root>
```

A preview card's trigger is a **Link**, not a button: it previews a destination. Hover arms the
core's exact open deadline, leaving both the trigger and the popup arms the close deadline, and
focus opens it at once. A closed card mounts no positioner, popup, or arrow at all; the popup's
Escape and outside-press dismissal belong to the core, so it declares no `onDismiss` of its own.

## Scroll areas, OTP fields, drawers, and navigation menus

| Component | Parts | Declared props | Reported through | Core guide |
| --- | --- | --- | --- | --- |
| `ScrollArea` | `Root`, `Viewport`, `Content`, `Scrollbar`, `Thumb`, `Corner` | `viewportSize`, `contentSize`, `overflowEdgeThreshold` on the root; `orientation` and `keepMounted` on a scrollbar | `onScrollStateChange(state, event)` and `useScrollAreaState()` | [Base UI components](base-ui-components.md) |
| `OtpField` | `Root`, `Input`, `Separator` | `value`/`defaultValue`, `length`, `validationType`, `mask`, `disabled`, `readOnly`, `required`, `autoSubmit`; `index` on a slot | `onValueChange(value, event)`, `onComplete(value, event)` | [Base UI components](base-ui-components.md) |
| `Drawer` | `Root`, `Trigger`, `Portal`, `Backdrop`, `Viewport`, `Popup`, `Content`, `Title`, `Description`, `Close`, `SwipeArea` | `open`/`defaultOpen`, `modal`, `swipeDirection`, `snapPoints`, `snapPoint`, `disablePointerDismissal` | `onOpenChange`, `onSnapPointChange(index, event)`, `onSwipeChange(swipe, event)` and `useDrawerSwipe()` | [Base UI components](base-ui-components.md) |
| `NavigationMenu` | `Root`, `List`, `Item`, `Trigger`, `Icon`, `Content`, `Link`, `Portal`, `Positioner`, `Popup`, `Viewport`, `Arrow`, `Backdrop` | `value`/`defaultValue`, `orientation`, `delay`, `closeDelay`, `loopFocus`, `placement`, optional `items`; `value` on an item | `onValueChange(value, event)`, `onActivationDirectionChange(direction, event)` | [Base UI components](base-ui-components.md) |

A scroll area is the caller-styled alternative to QuickGUI's built-in overlay scrollbars, which
stay the default on any ordinary `overflowY: "scroll"` container. The core owns the clamped
offsets, the derived overflow flags, the thumb arithmetic, and the captured pointer contract for
thumb drags and track presses; the viewport is clipped and the application applies the reported
offset as a paint-only transform, so the two never fight over the same wheel event.

QuickGUI has no layout observer at the hosted boundary, so the extents the arithmetic needs are
declared ahead of the core's decision like every other bounded property: `viewportSize` and
`contentSize` are the boxes the application laid out. Everything else comes back:

```tsx
<ScrollArea.Root
  viewportSize={{ width: 260, height: 160 }}
  contentSize={{ width: 260, height: rows.length * 22 }}
  onScrollStateChange={setScroll}
>
  <ScrollArea.Viewport>
    <ScrollArea.Content style={{ transform: `translateY(${-(scroll()?.offset.y ?? 0)}px)` }}>
      {/* rows */}
    </ScrollArea.Content>
  </ScrollArea.Viewport>
  <ScrollArea.Scrollbar orientation="vertical">
    <ScrollArea.Thumb />
  </ScrollArea.Scrollbar>
</ScrollArea.Root>
```

`useScrollAreaState()` reads the same `data-`-like render state inside the subtree: `scrolling`,
`hovering`, `hasOverflowX` / `hasOverflowY`, and the four overflow-edge flags. A scrollbar for an
axis that cannot scroll is not mounted at all unless `keepMounted` is declared; the core keeps
that flag per scroll area, so one kept scrollbar keeps the whole area's scrollbars and corner
mounted, and a mounted-but-useless scrollbar is hidden from assistive technology.

An OTP field's slots are the core's own text inputs. Accepted characters fill and advance, typing
over a filled slot replaces it, a paste distributes across consecutive slots, Backspace clears in
place and then walks back, Delete clears without moving, and the arrows plus Home and End move
between slots — every one of those inside the core, so a slot declares no `onInput` of its own.
`autoSubmit` routes through the core's own form submission, validating exactly as Return would.

```tsx
<OtpField.Root length={6} value={code()} onValueChange={setCode} onComplete={verify}>
  <For each={[0, 1, 2, 3, 4, 5]}>{(index) => <OtpField.Input index={index} />}</For>
</OtpField.Root>
```

A drawer reuses the core's dialog machinery for focus containment, Escape, backdrop dismissal, and
focus restoration; `modal` selects `true` (contain focus and project modal semantics), `"trap-focus"`
(contain focus only), or `false`. A snap point at or below `1` is a fraction of the viewport extent
and a larger one is an absolute pixel extent. The swipe the core decides — `swiping` and a never
negative `swipeOffset` — comes back through `onSwipeChange` and `useDrawerSwipe()`, and the
application applies the offset as a paint-only transform so a drag never relayouts. A closed drawer
contributes no overlay, focus trap, backdrop, or swipe surface at all.

A navigation menu's root is a Navigation landmark, its list carries the List role and orientation,
and each trigger is a Button with `has-popup`, expanded state, and a controls relationship to its
open panel. Every part inside a `NavigationMenu.Item` inherits that item's `value`, so a
composition never repeats it. The ordered model comes from the mounted `Item` children in
declaration order; declare `items` on the root when a disabled item or a model the children do not
spell out is needed.

Hovering a trigger opens after `delay` when every panel is closed and switches immediately when one
is already open; leaving both the trigger and the popup arms `closeDelay`. Arrow keys along the
orientation plus Home and End move the bar's single Tab stop between enabled triggers, Enter and
Space open, and Escape closes without leaving the bar. `onActivationDirectionChange` reports
`"left"`, `"right"`, `"up"`, `"down"`, or `null`, so an application can slide its panel the way the
user's attention travelled; QuickGUI never animates the panel itself.

Every declaration here is bounded exactly as the Rust binding bounds it:
`MAX_CHECKBOX_GROUP_VALUES` (256), `MAX_OTP_LENGTH` (12), `MAX_DRAWER_SNAP_POINTS` (8),
`MAX_NAVIGATION_MENU_ITEMS` (64), `MAX_AVATAR_FALLBACK_DELAY_MS` / `MAX_PREVIEW_CARD_DELAY_MS` /
`MAX_NAVIGATION_MENU_DELAY_MS` (10 000 ms each), and `MAX_SCROLL_AREA_OVERFLOW_THRESHOLD` (256 px).
A malformed declaration declares nothing at all rather than reaching a core constructor that would
panic on it, and a part whose activation, editing, gesture, or dismissal the core owns ignores a
declared listener for that same edge rather than registering it twice.

## Extended text styling

Every text-bearing node inherits these through its subtree exactly as the Rust core's own
typography does. The core owns shaping, case mapping, break opportunities, and the retained shaping
key; JavaScript only names the value.

| Prop | Values | Notes |
| --- | --- | --- |
| `textAlign` | `left`, `center`, `right`, `justify`, `start`, `end` | `start` and `end` stay logical across the boundary and resolve against the inherited `direction` in the core |
| `letterSpacing` | number or `<n>px` | Clamped by the core to ±256 logical pixels |
| `wordSpacing` | number or `<n>px` | Extra advance after every space character |
| `textTransform` | `none`, `uppercase`, `lowercase`, `capitalize` | Non-editable text only; selection, copy, and accessibility keep the original string, and an `Input` is never transformed |
| `textShadow` | `"x y blur color"`, `{ offsetX, offsetY, blur?, color? }`, or `none` | Offset and color are exact; blur is the core's bounded approximation. An omitted color adopts the element's own `color` |
| `textDecoration` / `textDecorationLine` | space-separated `underline`, `line-through`, `overline`, or `none` | Combinable |
| `textDecorationColor` | color | Applied to whichever lines were declared |
| `textDecorationStyle` | `solid`, `double`, `wavy` | `double` selects the core's two-line underline; `wavy` its GPU-rendered spell-checker underline |
| `textDecorationThickness` | number or `<n>px` | Adopts the closest native thickness the core exposes: 0, 1, 2, 4, or 8 |
| `wordBreak` | `normal`, `break-all`, `keep-all` | |
| `overflowWrap` | `normal`, `anywhere`, `break-word` | |
| `hyphens` | `none`, `manual`, `auto` | The core renders author-placed soft hyphens (`U+00AD`) and never hyphenates from a dictionary, so `auto` adopts the same manual behavior |
| `textDirection` | `auto`, `ltr`, `rtl` | Base paragraph direction used while shaping, without mirroring layout |

```tsx
<Text
  style={{
    textAlign: "center",
    letterSpacing: 1.5,
    textTransform: "uppercase",
    textShadow: "0 3px 10px #38bdf8aa",
    textDecoration: "underline",
    textDecorationStyle: "wavy",
    textDecorationColor: "#f97316",
  }}
>
  Release notes
</Text>
```

## Direction-relative layout, sticky positioning, and scroll snapping

`direction` is inherited by the whole subtree. In an RTL subtree the core mirrors in-flow positions,
physical horizontal insets, and margins for paint, hit testing, and accessibility together, and
horizontal scrolling starts at the right edge. Padding and borders stay physical; the logical props
below follow the direction instead.

| Prop | Values | Notes |
| --- | --- | --- |
| `direction` | `ltr`, `rtl` | Inherited by every descendant that does not declare its own |
| `paddingStart` / `paddingEnd` | number or `<n>px` | Also spelled `paddingInlineStart` / `paddingInlineEnd` |
| `marginStart` / `marginEnd` | number or `<n>px` | Also spelled `marginInlineStart` / `marginInlineEnd` |
| `borderStartWidth` / `borderEndWidth` | number or `<n>px` | Also spelled `borderInlineStartWidth` / `borderInlineEndWidth` |
| `position: "sticky"` | with `top`, `right`, `bottom`, `left` | Insets become the core's sticky offsets: the element keeps its space in flow and pins inside the nearest scroll container, so scrolling never relayouts |
| `overflowX` | `visible`, `hidden`, `auto`, `scroll` | `auto` and `scroll` both select the core's horizontal scroll container; declaring it alongside a scrolling `overflowY` selects both axes |
| `scrollSnapType` | `x` / `y` / `both`, optionally with `mandatory` or `proximity` | An axis without a strictness declares `proximity`, matching CSS |
| `scrollSnapAlign` | `start`, `center`, `end` | Declared on the snap child |
| `scrollSnapStop` | `normal`, `always` | `always` forbids a gesture from passing over this child |

```tsx
<View style={{ height: 240, overflowY: "scroll" }}>
  <View style={{ position: "sticky", top: 0, height: 28 }}>
    <Text>Inbox</Text>
  </View>
</View>

<View style={{ overflowX: "scroll", scrollSnapType: "x mandatory", flexDirection: "row" }}>
  <View style={{ width: 200, flexShrink: 0, scrollSnapAlign: "start", scrollSnapStop: "always" }} />
</View>
```

Snapping resolves at a native momentum end phase, at a bounded settle deadline for wheels without
phases, at scrollbar release, or programmatically, then animates to the target on exact deadlines
and leaves the window settled. Sticky elements are bounded per window by the core's own
`MAX_STICKY_ELEMENTS_PER_WINDOW`, and snap containers and points by
`MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW` and `MAX_SCROLL_SNAP_POINTS_PER_WINDOW`.

## Gradients, outlines, filters, transforms, and blending

`background` accepts a color, a CSS gradient function, or the declared object form; the Rust binding
parses whichever arrives into one bounded core `Gradient` and the renderer clears the other
property, so a declaration is never ambiguous. `backgroundColor` stays color-only.

| Prop | Values | Notes |
| --- | --- | --- |
| `background` / `backgroundGradient` | color, `linear-gradient(…)`, `radial-gradient(…)`, `conic-gradient(…)`, or `GradientDeclaration` | At most eight stops; extra stops are dropped in source order |
| `borderRadius` | number, `<n>px`, or a one-to-four value shorthand | More than four radii throws a `TypeError` |
| `borderTopLeftRadius` and the other three corners | number or `<n>px` | Override the shorthand per corner |
| `borderStyle` | `solid`, `dashed`, `dotted` | Dash geometry is analytic arc length along the rounded outline |
| `outline` | CSS shorthand such as `2px dashed #38bdf8`, a plain width, or `none` | Split by the renderer into the width, style, and color the core declares separately |
| `outlineWidth`, `outlineColor`, `outlineOffset`, `outlineStyle` | | Painted outside the border box; never affects layout |
| `backgroundImage` | path, `file://`, or base64 `data:` URL | Decoded once per declaration and retained until it changes; a source that cannot be decoded paints nothing |
| `backgroundSize` | `auto`, `cover`, `contain`, or `<w> <h>` | |
| `backgroundRepeat` | `no-repeat`, `repeat`, `repeat-x`, `repeat-y` | Tiles are capped by the core's `MAX_BACKGROUND_IMAGE_TILES` |
| `backgroundPosition` | keywords or percentages, such as `right bottom` or `50% 0%` | |
| `filter` | CSS filter-function list, or an array of them | `brightness`, `contrast`, `saturate`, `grayscale`, `invert`, `sepia`, `hue-rotate`, `opacity`, `blur`, `drop-shadow` |
| `backdropFilter` | the same list | Colour filters plus one blur applied to what is already painted behind the element |
| `transform` | CSS transform-function list, an array, or `{ a, b, c, d, tx, ty }` | `translate`, `translateX/Y`, `scale`, `scaleX/Y`, `rotate`, `skew`, `skewX/Y`, `matrix` |
| `transformOrigin` | keywords or percentages, such as `left top` | Fraction of the border box; defaults to the centre |
| `mixBlendMode` | `normal`, `multiply`, `screen`, `darken`, `lighten`, `overlay`, `difference`, `exclusion`, `hard-light`, `color-dodge`, `color-burn` | |

```tsx
<View
  style={{
    background: "linear-gradient(135deg, #1d4ed8, #38bdf8 60%, #a855f7)",
    borderRadius: "22px 6px 22px 6px",
    borderWidth: 2,
    borderStyle: "dashed",
    borderColor: "#f97316",
    outline: "2px solid #a855f7",
    outlineOffset: 4,
    filter: "saturate(1.4) blur(2px)",
    transform: "rotate(-3deg) scale(1.02)",
    mixBlendMode: "multiply",
  }}
/>

<View
  style={{
    background: { type: "radial", shape: "circle", center: { x: 0.3, y: 0.2 }, stops: ["#1d4ed8", "#0f172a"] },
  }}
/>
```

A colour-only `filter` chain is one per-primitive matrix and allocates nothing. Adding `blur()` or
`drop-shadow()`, any transform beyond a whole-pixel translation, a `backdropFilter`, or a
non-`normal` `mixBlendMode` promotes the element to a compositing group, whose per-frame cost and
bounds are described in [graphics](graphics.md). Transforms are paint-only: layout, measurement, and
reported bounds never move, and pointer positions are inverse-mapped so clicks follow the painted
pixels.

Hover, active, and focus variants exist for exactly the three things the core's `ElementStateStyle`
can swap — the gradient, the outline ring, and the transform — alongside the existing colors:

| State | Props |
| --- | --- |
| hover | `hoverBackgroundColor`, `hoverColor`, `hoverBackground`, `hoverOutline`, `hoverTransform` |
| active | `activeBackgroundColor`, `activeColor`, `activeBackground`, `activeOutline`, `activeTransform` |
| focus | `focusBackgroundColor`, `focusColor`, `focusBackground`, `focusOutline`, `focusTransform` |

```tsx
<Button
  style={{
    hoverBackground: "linear-gradient(90deg, #1d4ed8, #38bdf8)",
    hoverTransform: "scale(1.03) translate(0, -2px)",
    focusOutline: "2px solid #60a5fa",
    outlineOffset: 3,
  }}
>
  Publish
</Button>
```

A state outline uses the element's own `outlineOffset`, because that is the one offset the core's
state style carries. Gradients are swapped rather than interpolated: only the transitionable solid
background, border, radius, shadow, opacity, and text color animate.

Every declaration on this page is bounded before it crosses N-API. A gradient, filter, transform,
outline, or text-shadow string past `MAX_STYLE_DECLARATION_BYTES` (4 096) throws a `TypeError` in
JavaScript, and a declaration the Rust grammar does not cover — an unknown filter function, a color
outside the supported CSS grammar, an unparsable angle — declares nothing at all instead of reaching
a core constructor. Colors inside these strings accept the same `#rgb`, `#rrggbb`, `#rrggbbaa`,
`rgb()`, `rgba()`, `transparent`, `black`, and `white` grammar the renderer packs everywhere else.

Two examples cover all of this end to end:

```console
cd examples/styling-solid
bun run dev

cd examples/components-solid
bun run dev
```

## Tooltips

Any host component accepts a `tooltip` string plus `tooltipPlacement`, `tooltipDelay` (milliseconds,
clamped by the core to ten seconds), `tooltipGap`, and `tooltipViewportMargin`. The core lays the
detached tooltip tree out only after the hover delay expires and exposes the same text as the
trigger's native accessibility description:

```tsx
<Button tooltip="Rename this workspace" tooltipPlacement="top" tooltipDelay={250}>
  Rename
</Button>
```

Tooltip text is bounded to 1024 bytes. A tooltip whose content is an arbitrary element tree rather
than text is not bridged yet; the retained mutation protocol has no detached-subtree opcode.

All Solid host components are intentionally unstyled. Their `style` prop uses web-shaped names
for the currently bridged QuickGUI layout, text, paint, overflow, cursor, positioning, and
`appRegion` properties. Native events are flushed at a Solid 2 event boundary before the retained
mutation batch is submitted.

Borders can be set per edge, and shadows use CSS `box-shadow` order and syntax:

```tsx
<View
  style={{
    borderColor: "#dfe5ed",
    borderTopWidth: 1,
    borderRightWidth: 0,
    borderBottomWidth: 2,
    borderLeftWidth: 0,
    boxShadow: "0 18px 45px -24px rgba(15, 23, 42, 0.35), inset 0 1px white",
  }}
/>
```

`borderTopWidth`, `borderRightWidth`, `borderBottomWidth`, and `borderLeftWidth` independently
override `borderWidth` and share `borderColor`. `boxShadow` accepts up to eight comma-separated
drop or `inset` shadows with two to four numeric or `px` lengths; an omitted color uses the
element's current text color. Use `boxShadow: "none"` to clear the list.

`Input` and `TextArea` are controlled native editors. Update their `value` from `event.value` in
`onInput`; Return on a single-line `Input` invokes `onSubmit`. Use `<Input type="password">` for a
masked secure field, then switch the controlled `type` to `"text"` for an explicit reveal action.
`Markdown` is QuickGUI core's retained native renderer, accepts controlled `content`/`source` and
`streaming` props, and is documented in the [Markdown guide](markdown.md).

Use the unstyled `VirtualList` for long variable-height collections. Its direct children are the
logical rows; native layout mounts only the visible range and overscan, measures real wrapped row
heights, preserves the scroll anchor, and can follow an appended chat tail:

```tsx
<VirtualList estimatedItemHeight={180} overscan={1} followMode="tail">
  <For each={messages()} keyed={(message) => message.id}>
    {(message) => <MessageCard message={message()} />}
  </For>
</VirtualList>
```

The runnable source and CLI configuration are in
[`examples/solid`](../examples/solid).

The [sidebar vibrancy example](../examples/sidebar-vibrancy-solid) keeps the Solid root and sidebar
transparent and switches among every Electron-compatible macOS semantic material through the
Rust-core window API while leaving the main content pane opaque. Its dividers use individual edge
borders, and its content card uses `boxShadow`.

The [Solid AI chat example](../examples/ai-chat-solid) adds Vercel AI SDK/DeepSeek streaming, a
`SystemPopover` provider-settings surface with a revealable password field, controlled input,
cancellation, paced updates, and core Markdown rendering.

The [system API example](../examples/system-api-solid) keeps renderer-neutral desktop APIs in
`@quickgui/native` while Solid owns only the UI. It exercises the Rust core's application
environment, rich clipboard, displays, permissions, preferences, power, menus, notifications,
desktop integrations, native file icons, and imperative window controls.

## Keyboard, mouse, gesture, and drag events

Every input listener is *declared* — the property says a listener exists, and the Rust core's own
targeting, capture, bubbling, multi-click counting, accelerator parsing, and drag promotion decide
when it runs. Payloads travel back as bounded asynchronous JSON, so no synchronous question ever
crosses the hosted boundary.

| Prop | Native event | Payload |
| --- | --- | --- |
| `onKeyDown`, `onKeyUp` | `keydown`, `keyup` | `key`, `text`, `repeat`, modifiers |
| `onMouseDown`, `onMouseUp` | `mousedown`, `mouseup` | `x`, `y`, `button`, `clickCount`, modifiers |
| `onMouseMove` | `mousemove` | `x`, `y`, `pressedButton`, modifiers |
| `onDoubleClick` | `dblclick` | the second press of one exact native sequence |
| `onWheel` | `wheel` | `deltaX`, `deltaY`, `precise`, `phase`, modifiers |
| `onContextMenu` | `contextmenu` | `x`, `y`, modifiers |
| `onPinch`, `onRotate` | `pinch`, `rotate` | `x`, `y`, `delta`, `phase`, modifiers |
| `onSmartMagnify` | `smartmagnify` | `x`, `y`, modifiers |
| `onPressure` | `pressure` | `x`, `y`, `pressure`, `stage`, modifiers |
| `onFocus`, `onBlur` | `focus`, `blur` | no payload |
| `onAction` | `action` | the declared `keymap` binding id |
| `onDragStart`, `onDragEnd` | `dragstart`, `dragend` | drag geometry, then the native operation |
| `onDrop`, `onFilesDropped` | `drop`, `filesdropped` | `id`/`source`, or absolute `paths` |

`keyEventFromEvent`, `mouseEventFromEvent`, `wheelEventFromEvent`, `gestureEventFromEvent`,
`dropEventFromEvent`, and `actionFromEvent` decode a payload; a malformed one yields `undefined`
rather than throwing inside a listener.

```tsx
<View
  tabIndex={0}
  keymap={{ "CmdOrCtrl+S": "save", "CmdOrCtrl+Shift+P": "palette" }}
  onAction={(event) => run(actionFromEvent(event))}
  onKeyDown={(event) => {
    const key = keyEventFromEvent(event);
    if (key?.key === "Escape") close();
  }}
  onDoubleClick={() => openInspector()}
  onWheel={(event) => zoom(wheelEventFromEvent(event)?.deltaY ?? 0)}
/>
```

Focused key, action, and focus events need a focusable element. A declared `tabIndex` makes an
ordinary container focusable, exactly as the web attribute does; buttons, inputs, and core component
parts are already focusable.

`keymap` maps Electron-shaped accelerators to binding ids, parsed by the core's own
`Accelerator::parse`. A matched accelerator becomes one `action` event carrying the id and consumes
the keystroke, so JavaScript never interprets modifiers itself. An accelerator the core rejects is
skipped rather than silencing the whole table, one element retains at most 256 bindings, and the
declaration is bounded to 64 KiB.

Drag sources declare their payload ahead of the gesture, and drop targets declare the payload kinds
they accept so the core can decide compatibility while the pointer is still moving:

```tsx
<View
  draggable={{ id: row.id, text: row.title, files: [{ path: row.path }] }}
  onDragStart={() => setDragging(true)}
  onDragEnd={() => setDragging(false)}
/>

<View
  dropKinds={["local", "files"]}
  onDrop={(event) => move(dropEventFromEvent(event)?.id)}
  onFilesDropped={(event) => open(dropEventFromEvent(event)?.paths ?? [])}
/>
```

An application-local drop reports the declared `id`, the originating node, and whether the payload
stayed inside this window, crossed between QuickGUI windows, or came from another application. A
`files` drop reports absolute paths. `onDragOver` is not bound: the Rust core reports drag hovering
on the window rather than through a per-element listener.

Physical `code` values are not reported. The core normalizes keys to a layout-independent command
identity plus the printable character, which is what accelerators and keymaps match on.

## Lifecycle vetoes in JavaScript

A hosted JavaScript listener can never veto a native decision synchronously, because the hosted
boundary never blocks the native main thread. Vetoes are therefore declared ahead of time and the
held decision is completed later by an explicit call.

### Close interception

Registering an `onCloseRequested` listener declares interception for that window. While at least one
listener exists the core answers `Event::CloseRequested` by preventing the close and emitting a
`closeRequested` event; with no listener left, native closes proceed as before.

```ts
import { Dialog, Window } from "@quickgui/native";

const window = new Window({ renderer, title: "Editor" });

const stopIntercepting = window.onCloseRequested(async ({ window }) => {
  if (!hasUnsavedChanges()) {
    window.close();
    return;
  }
  const choice = await Dialog.showAlertDialog(window, {
    message: "Save before closing?",
    buttons: ["Save", "Discard", "Cancel"],
  });
  if (choice === 0) await save();
  if (choice !== 2) window.destroy();
});

window.on("closed", ({ window }) => console.log("closed", window.nativeId));
```

`window.close()` completes a held request; `window.destroy()` also drops every registered
`closeRequested` listener first, so it can never be intercepted again on its way out. Calling
`stopIntercepting()` withdraws the declaration and lets the operating system close the window
directly. `window.on("closed", …)` is the event-map spelling of `window.onClose(…)`.

### Quit interception

Registering a `beforeQuit` listener declares quit interception for the application. The Rust
`on_before_quit` hook then prevents the quit and reports the reason to JavaScript; `willQuit` is a
notification delivered by the final phase.

```ts
import { app } from "@quickgui/native";

app.on("beforeQuit", async ({ reason }) => {
  console.log("quit requested because", reason);
  if (await everyWindowSaved()) await app.quit({ force: true });
});

app.on("willQuit", ({ reason }) => flushTelemetry(reason));
```

`reason` is `"explicit"`, `"relaunch"`, `"last-window-closed"`, or `"operating-system"`.
`app.quit()` asks for a preventable quit that runs both phases, so it is held by an interception
just like a native Command-Q. `app.quit({ force: true })` completes a held quit, and
`app.exit(code)` force-quits and reports `code` from `app.run()` and the `quit` event — the core's
native event loop carries no application-chosen exit status, so the code is reported by the
JavaScript host rather than invented natively.

`quitMode` semantics are unchanged: `"last-window-closed"` still begins a quit with the final
window, and an interception simply holds that quit (with reason `"last-window-closed"`) until
JavaScript completes it.

## Native menus, accelerators, and window commands in JavaScript

`MenuActionItem` and `MenuRoleItem` accept `accelerator` (Electron accelerator syntax, parsed by the
Rust core's `Accelerator::parse`) and `hidden`. Unparsable or oversized accelerators are rejected by
the core with an error rather than silently dropped.

```ts
import { Menu } from "@quickgui/native";

Menu.setApplicationMenu([
  {
    label: "File",
    items: [
      { label: "Save As…", accelerator: "CmdOrCtrl+Shift+S", click: saveAs },
      { type: "system-menu", label: "Open Recent", menu: "recent-documents" },
      { type: "separator" },
      { type: "role", label: "Close Window", role: "close-window" },
      { label: "Reload Fixtures", hidden: !import.meta.env?.DEV, click: reload },
    ],
  },
  {
    label: "Edit",
    items: [
      { type: "role", label: "Paste and Match Style", role: "paste-and-match-style" },
      { type: "role", label: "Delete", role: "delete" },
      { type: "role", label: "Start Speaking", role: "start-speaking" },
    ],
  },
]);
```

`MenuRole` also covers `"select-next-tab"`, `"select-previous-tab"`, `"merge-all-windows"`,
`"move-tab-to-new-window"`, `"toggle-tab-bar"`, and `"toggle-tab-overview"`.

Window tab commands, the character palette, and the Find pasteboard are imperative fire-and-forget
window and application commands:

```ts
window.setTabbingIdentifier("documents");
window.selectNextTab();
window.selectPreviousTab();
window.selectTab(0);
window.mergeAllWindows();
window.moveTabToNewWindow();
window.toggleTabBar();
window.toggleTabOverview();
window.showCharacterPalette();

const state = await window.getState();
console.log(state.nativeTabs.count, state.nativeTabs.selectedIndex);
```

`Clipboard` adds Electron-shaped format helpers over the core's typed entries:
`availableFormats()`, `has(format)`, `readBuffer(format)`, `writeBuffer(format, data)`, and the
macOS Find pasteboard through `readFindText()`/`writeFindText()`.

## Window lifecycle events in JavaScript

`window.on(type, listener)` returns a disposer and covers the core's window notifications:

```ts
import { Window, app } from "@quickgui/native";

const window = new Window({ renderer, visible: false });

window.on("readyToShow", () => window.show());
window.on("minimize", () => pauseAnimations());
window.on("restore", () => resumeAnimations());
window.on("maximize", ({ window }) => remember(window));
window.on("unmaximize", () => remember(window));
window.on("enterFullScreen", () => hideChrome());
window.on("leaveFullScreen", () => showChrome());
window.on("occlusionChange", ({ occluded }) => setRenderingPaused(occluded));
window.on("levelChange", ({ level }) => console.log(level));
window.on("resize", ({ size }) => layout(size));
window.on("move", ({ position }) => remember(position));
window.on("focus", () => setActive(true));
window.on("blur", () => setActive(false));
window.on("appearanceChange", ({ appearance }) => setTheme(appearance));
window.on("closed", ({ window }) => forget(window));

app.on("activate", () => refresh());
app.on("deactivate", () => flushDrafts());
```

`willResize` and `willMove` are **notifications**. The core has to answer the window manager on the
application thread, and the hosted JavaScript boundary never blocks it, so the narrowing itself is
declared ahead as a policy:

```ts
window.setResizePolicy({
  aspectRatio: 16 / 9,
  minimum: { width: 640, height: 360 },
  maximum: { width: 3840, height: 2160 },
  snap: { width: 8, height: 8 },
});
window.setMovePolicy({ keepOnScreen: true });

window.on("willResize", ({ size }) => console.log("proposed", size));

// Withdraw either policy with null.
window.setResizePolicy(null);
window.setMovePolicy(null);
```

The core applies the grid step first, then the aspect ratio, then the minimum and maximum, and
answers `Event::WillResize` with `constrain_resize`. `keepOnScreen` clamps the proposed origin into
the work area of the display that contains it through `constrain_move`.

## Window stacking, input policy, and restore state in JavaScript

```ts
window.setAlwaysOnTop(true, "screenSaver"); // Electron level names, or QuickGUI's kebab-case ones
window.setAlwaysOnTop(false); // back to "normal"
window.moveTop();
window.moveAbove(other);

window.setIgnoreMouseEvents(true, { forward: true }); // clicks pass through, hover still arrives
window.setEnabled(false); // visible and rendering, but no native input at all
window.setAspectRatio({ width: 16, height: 9 });
window.setAspectRatio(null);
window.setWindowButtonVisibility(false);
window.setHasShadow(true);

const restoreState = await window.getRestoreState();
localStorage.setItem("window", JSON.stringify(restoreState));
```

`getRestoreState()` returns the windowed restore rectangle, so a maximized or fullscreen window
still persists the size it returns to, plus the display identity. Hand it back on the next launch:

```ts
const stored = localStorage.getItem("window");
new Window({
  renderer,
  ...(stored ? { restoreState: JSON.parse(stored) } : {}),
});
```

The core re-validates every field, so a stale or hostile value can never place a window off every
connected display. `window.getState()` also reports `windowLevel`, `ignoreMouseEvents`,
`aspectRatio`, and the other stacking and input fields.

## Application shell in JavaScript

```ts
import { Shell, SpellChecker, app } from "@quickgui/native";

await app.setActivationPolicy("accessory"); // "regular" | "accessory" | "prohibited"
app.focus({ steal: true });
app.hide();
app.show();
app.setSecureKeyboardEntryEnabled(true);
Shell.beep();

const bounce = await app.dock.bounce("critical");
app.dock.cancelBounce(bounce);
await app.dock.hide();
await app.dock.show();
app.dock.isVisible(); // the last visibility this process asked for
app.dock.setBadge("3");
app.dock.setIcon("./assets/dock.png");
app.dock.setMenu({ label: "Dock", items: [{ label: "New Window", click: openWindow }] });

if (app.isPackaged && !(await app.isInApplicationsFolder())) {
  if (await app.moveToApplicationsFolder()) await app.relaunch();
}

await app.exit(2); // completes a held quit and exits with status 2

SpellChecker.learnWord("quickgui");
SpellChecker.ignoreWord("quickgui");
```

Mutations that the operating system applies immediately (`focus`, `hide`, `show`,
`setSecureKeyboardEntryEnabled`, `Shell.beep`, `dock.cancelBounce`, `SpellChecker.*`) are
fire-and-forget: nothing waits on the native main thread. Operations whose outcome the operating
system reports (`setActivationPolicy`, `dock.bounce`, `dock.hide`/`dock.show`,
`moveToApplicationsFolder`) return promises resolved by an asynchronous native event.

## Popup menus and per-window menus in JavaScript

`Menu.popup` reuses the application-menu item grammar, including roles, marks, icons, accelerators,
and `hidden`. It resolves once the popup closes — whether an item ran or the user dismissed it — and
releases the item callbacks with it.

```ts
import { Menu } from "@quickgui/native";

await Menu.popup(
  [
    { label: "Copy", role: "copy" },
    { type: "separator" },
    { label: "Inspect", click: () => inspect(node) },
  ],
  { window, x: event.x, y: event.y },
);
```

Omit `x` and `y` to open at the current cursor position; supplying only one of them is rejected.

```ts
window.setMenu([{ label: "Document", items: [{ label: "Export…", click: exportDocument }] }]);
window.setMenu(null); // inherit the application menu again
```

On macOS a window menu becomes the process menu bar while that window is active. Both commands are
resolved inside the runtime's own effect cycle, where the `EventContext`-scoped popup and
window-menu commands exist, and both keep the core's `MAX_PENDING_NATIVE_POPUP_MENUS` bound.

## Current boundary

This vertical slice supports dynamically created independent native windows, controlled system
and retained in-window popovers, native alert and file dialogs with optional window
ownership, retained view/text/button/input/Markdown nodes, variable-height virtual lists, password
inputs, reactive properties and text, click/hover/input/submit/dismiss events, core-backed app and
window lifecycle, window lifecycle events with declared-ahead resize and move policies, window
stacking, input, and restore-state commands, the application shell and Dock, native popup and
per-window menus, native menus and desktop services, web-shaped Flexbox styling, hidden-inset
titlebars, traffic-light positioning, declared close and quit interception, menu accelerators and
system submenus, native window-tab commands, controlled selection controls, tab sets, disclosures,
and field/fieldset composition, controlled in-window dialogs and alert dialogs, delayed native
tooltips, declared popover and context menus, CSS Grid layout, complete paint transitions, retained
images and application shaders, extended text styling, direction-relative layout, sticky
positioning and scroll snapping, gradients, per-corner radii, border and outline rings, raster
backgrounds, filters, backdrop effects, transforms, and blend modes, progress/meter/toggle parts,
sliders, range sliders, splitters,
toolbars, and toggle groups, declared option sources with core-rendered popover rows, virtual
tables and trees, number fields, date and time fields, month grids, in-window menubars, declared
toast queues, separators, avatars, checkbox groups, preview cards, caller-styled scroll areas, OTP
fields, drawers, and navigation menus, the Base UI-aligned popover and tooltip compounds with
resolved-placement reporting, slider label/value/control parts with the core's commit boundary,
number-field scrub areas, progress and meter track/label/value parts, the toast provider and
manager, tab activation direction and indicator geometry, toolbar button/link/input/group/separator
parts, field item and validity parts, read-only selection controls, and a dialog viewport with its
own exit transition, declared keyboard, mouse, gesture, accelerator, and drag-and-drop events, a
stable real-`.app` development host, and self-contained production packaging on the current macOS
target. It is not yet the full Rust rendering API surface: animated-image playback control, native
child views, accessibility actions, every native binary target, and dedicated JavaScript
performance gates still need bindings and acceptance.

Two boundaries inside the newly bound components are worth naming. A picker's retained core state
is rebuilt rather than mutated when its declaration changes, because every replacement mutator the
core offers closes a live native popover and a render pass owns no `EventContext`; a declaration
committed while the surface is open therefore lands on the frame the close already schedules. And a
declared table or tree header, row, and cell carries content only — the core assigns their identity
and interaction — so an interactive control belongs inside a cell as an ordinary child node.

A fourth belongs to the Base UI-aligned compounds. A popover's and a tooltip's trigger is the one
part the core keeps mounted whether the surface is open or closed, so the trigger node carries the
whole declaration and `Popover.Root`, `Popover.Positioner`, `Tooltip.Root`, and
`Tooltip.Positioner` route their props onto it; a positioner that is not mounted cannot declare
anything for a surface that has not opened yet. The resolved placement is published during paint
and read on the correcting frame the core requests when it changes, so it is always one frame
behind the declaration — which is exactly what makes it the core's answer rather than a
measurement in JavaScript. And a slider's `onValueCommitted` is the core's captured-pointer
boundary only: a keyboard change reports through `onValueChange` without a separate commit,
because the core exposes no keyboard commit of its own.

A third belongs to the Base UI parity set: a scroll area declares the viewport and content extents
it laid out, because the hosted boundary has no layout observer and the core's offset, overflow,
and thumb arithmetic must be answered without asking JavaScript a synchronous question. Everything
derived from those extents stays the core's, and the scrollbar track defaults to the declared
viewport extent along its axis until a press reports the exact size the core laid out.

Return to the [documentation index](README.md).
