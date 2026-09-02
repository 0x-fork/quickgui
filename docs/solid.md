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
tooltips, a stable real-`.app` development host, and self-contained production packaging on the
current macOS target. It is not yet the full Rust rendering API surface: popover arrows and
backdrops, context and popover menus, select/combobox/autocomplete, tables and trees, images and
shaders, CSS Grid layout, keyboard, gesture, and drag-and-drop events, native child views,
accessibility actions, every native binary target, and dedicated JavaScript performance gates
still need bindings and acceptance.

Return to the [documentation index](README.md).
