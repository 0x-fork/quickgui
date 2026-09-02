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

## Current boundary

This vertical slice supports dynamically created independent native windows, controlled system
and retained in-window popovers, native alert and file dialogs with optional window
ownership, retained view/text/button/input/Markdown nodes, variable-height virtual lists, password
inputs, reactive properties and text, click/hover/input/submit/dismiss events, core-backed app and
window lifecycle, native menus and desktop services, web-shaped Flexbox styling, hidden-inset
titlebars, traffic-light positioning, controlled selection controls, tab sets, disclosures, and
field/fieldset composition, controlled in-window dialogs and alert dialogs, delayed native
tooltips, a stable real-`.app` development host, and self-contained production packaging on the
current macOS target. It is not yet the full Rust rendering API surface: popover arrows, context
and popover menus, select/combobox/autocomplete, tables and trees, images and shaders, CSS Grid
layout, keyboard, gesture, and drag-and-drop events, native child views, accessibility actions,
every native binary target, and dedicated JavaScript performance gates still need bindings and
acceptance.

Return to the [documentation index](README.md).
