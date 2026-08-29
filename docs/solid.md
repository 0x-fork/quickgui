# Solid 2 renderer

QuickGUI includes an experimental Bun host split into two unstyled packages:

- `@quickgui/native` owns the N-API boundary, application loop, per-window retained trees, binary
  mutation batches, and window-routed event queue.
- `@quickgui/solid` owns Solid 2 JSX compilation, fine-grained reactive updates, and the host
  components `View`, `Text`, `Button`, `Input`, `TextArea`, `Markdown`, and `VirtualList`.

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
import { App, Button, Dialog, Text, View, Window, render, type NativeNode } from "@quickgui/solid";
import { createSignal } from "solid-js";

const app = new App();
const mainWindow = new Window({ title: "Counter", width: 480, height: 320 });

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

render(() => <Counter />, mainWindow);
await app.run();
```

`App` establishes the application context for its JavaScript Worker while the native host owns the
platform event loop. Window configuration and rendered state belong to `Window`. Constructing a `Window` works
both before and after `run` starts, so an event handler can open another independent native window
without creating another application runtime:

```tsx
function openSettings() {
  const settings = new Window({ title: "Settings", width: 520, height: 420 });
  render(() => <Settings close={() => settings.close()} />, settings);
}
```

Closing a `Window` automatically disposes every Solid root mounted into it. Closing the last window
ends the current JavaScript host run.

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

Pass a mounted node as `anchor` to create a parent-owned native popup surface. Placement is resolved
against the display work area, so the popup can extend beyond its parent window and flips or slides
back on screen when needed:

```tsx
let trigger: NativeNode | undefined;

<Button ref={(node) => { trigger = node; }} onClick={() => {
  if (!trigger) return;
  const popup = new Window({
    title: "Provider settings",
    anchor: trigger,
    width: 420,
    height: 280,
    placement: "bottom-end",
    gap: 8,
  });
  render(() => <ProviderSettings close={() => popup.close()} />, popup);
}}>
  Provider settings
</Button>
```

All Solid host components are intentionally unstyled. Their `style` prop uses web-shaped names
for the currently bridged QuickGUI layout, text, paint, overflow, cursor, positioning, and
`appRegion` properties. Native events are flushed at a Solid 2 event boundary before the retained
mutation batch is submitted.

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

The [Solid AI chat example](../examples/ai-chat-solid) adds Vercel AI SDK/DeepSeek streaming, an
anchored provider-settings popup with a revealable password field, controlled input, cancellation,
paced updates, and core Markdown rendering.

## Current boundary

This vertical slice supports dynamically created independent and anchored native windows,
native alert and file dialogs with optional window ownership, retained view/text/button/input/Markdown nodes,
variable-height virtual lists, password inputs, reactive properties and text,
click/hover/input/submit events, web-shaped Flexbox styling, hidden-inset titlebars, traffic-light
positioning, a stable real-`.app` development host, and self-contained production packaging on the
current macOS target. It is not yet the full Rust API surface:
declarative popup parts, menus, native child views, accessibility actions, every native binary
target, and dedicated JavaScript performance gates still need bindings and acceptance.

Return to the [documentation index](README.md).
