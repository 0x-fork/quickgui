# Solid 2 renderer

QuickGUI includes an experimental Bun host split into two unstyled packages:

- `@quickgui/native` owns the N-API boundary, application loop, per-window retained trees, binary
  mutation batches, window-routed event queue, and renderer-owned `Window` lifecycle.
- `@quickgui/solid` owns Solid 2 JSX compilation, fine-grained reactive updates, and the host
  components `View`, `Text`, `Button`, `Input`, `TextArea`, `Markdown`, `VirtualList`,
  `Popover`/`SystemPopover`, and the compound `Checkbox`, `Radio`, `RadioGroup`, `Switch`, `Tabs`,
  `Collapsible`, `Accordion`, `Field`, `Fieldset`, `Dialog`, and `AlertDialog` parts, plus the
  `createRenderer` adapter passed to a native `Window`.

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

## Selection, tab, disclosure, and field parts

The unstyled Rust part descriptors are exposed as Base-UI-shaped compound components. Each part is
one ordinary native node that declares which core descriptor to rebuild; the Rust core keeps
ownership of part identity, roles, keyboard behavior, accessibility relationships, and whether an
inactive panel is mounted at all. JavaScript owns only the controlled value and the element tree,
so nothing about a component is decided by a synchronous callback across N-API.

| Component | Parts | Controlled props | Core guide |
| --- | --- | --- | --- |
| `Checkbox` | `Root`, `Indicator` | `checked` (`true`/`false`/`"indeterminate"`), `defaultChecked`, `onCheckedChange` | [selection controls](selection-controls.md) |
| `Radio` | `Root`, `Indicator` | `value`, or standalone `checked`/`onCheckedChange` | [selection controls](selection-controls.md) |
| `RadioGroup` | `Root` | `value`, `defaultValue`, `onValueChange` | [selection controls](selection-controls.md) |
| `Switch` | `Root`, `Thumb` | `checked`, `defaultChecked`, `onCheckedChange` | [selection controls](selection-controls.md) |
| `Tabs` | `Root`, `List`, `Tab`, `Indicator`, `Panel` | `value`, `defaultValue`, `onValueChange`, `orientation`, `activation`, `loop`, `keepMounted` | [tabs](tabs.md) |
| `Collapsible` | `Root`, `Trigger`, `Panel` | `open`, `defaultOpen`, `onOpenChange`, `disabled`, `keepMounted` | [disclosures](disclosures.md) |
| `Accordion` | `Root`, `Item`, `Header`, `Trigger`, `Panel` | `value`, `defaultValue`, `onValueChange`, `multiple`, `headingLevel`, `keepMounted`, `disabled` | [disclosures](disclosures.md) |
| `Field` | `Root`, `Label`, `Control`, `Description`, `Error` | `disabled`, `invalid`, `required`, `touched`, `dirty`, `filled`, `validationMessage` | [text and forms](text-and-forms.md) |
| `Fieldset` | `Root`, `Legend`, `Description`, `Control` | `disabled` | [text and forms](text-and-forms.md) |
| `Dialog`, `AlertDialog` | `Root`, `Trigger`, `Portal`, `Backdrop`, `Popup`, `Title`, `Description`, `Close` | `open`, `defaultOpen`, `onOpenChange`, `dismissOnEscape`, `dismissOnBackdrop` | [dialogs](dialogs.md) |

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

| Component | Parts | Declared props | Core guide |
| --- | --- | --- | --- |
| `Progress` | `Root`, `Indicator` | `value`, `max`, `indeterminate`, `valueText` | [range and feedback](range-and-feedback.md) |
| `Meter` | `Root`, `Indicator` | `value`, `min`, `max`, `low`, `high`, `optimum` | [range and feedback](range-and-feedback.md) |
| `Toggle` | `Root`, `Indicator` | `pressed`, `defaultPressed`, `onPressedChange` | [toolbar and toast](toolbar-and-toast.md) |

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
images and application shaders, progress/meter/toggle parts, declared keyboard, mouse, gesture,
accelerator, and drag-and-drop events, a stable real-`.app` development host, and self-contained
production packaging on the current macOS target. It is not yet the full Rust rendering API surface:
popover arrows and backdrops, select/combobox/autocomplete, tables and trees, sliders, number
fields, splitters, toolbars, toggle groups, and toasts, animated-image playback control, native
child views, accessibility actions, every
native binary target, and dedicated JavaScript performance gates still need bindings and acceptance.

The unbound interaction models above all reach their retained state through a non-capturing
`fn(&mut V) -> &mut State` accessor. One hosted view renders every declaring node, so it cannot
supply a distinct accessor per instance; those components need a closure- or entity-based accessor
in the Rust core before they can be bound.

Return to the [documentation index](README.md).
