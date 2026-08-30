# @quickgui/solid

Solid 2 renderer for QuickGUI. It exports unstyled `View`, `Text`, `Button`, `Input`, `TextArea`,
retained core `Markdown`, variable-height `VirtualList`, compound in-window `Popover` and
native-window `SystemPopover` parts, and `createRenderer`.
Import native application/window APIs from `@quickgui/native` and reactive primitives from
`solid-js` itself.

Run applications through the QuickGUI CLI:

```console
bun run dev
```

```tsx
import { app, Window } from "@quickgui/native";
import { Button, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

function Counter() {
  const [count, setCount] = createSignal(0);
  return <Button onClick={() => setCount(count() + 1)}>Count: {count()}</Button>;
}

await app.whenReady();
new Window({
  title: "QuickGUI",
  renderer: createRenderer(() => <Counter />),
});
```

The CLI owns the native application loop; application source does not call `app.run()`.

See the [Solid renderer guide](../../docs/solid.md) and the
[runnable example](../../examples/solid/app.tsx). The
[AI chat example](../../examples/ai-chat-solid/app.tsx) demonstrates controlled input and live
streaming Markdown over a tail-following virtual transcript with the Vercel AI SDK and DeepSeek.
The [alert-dialog example](../../examples/alert-dialog-solid/app.tsx) demonstrates information,
warning, and critical native alerts with optional window ownership and semantic button roles.
The [file-dialog example](../../examples/file-dialog-solid/app.tsx) demonstrates native open-file,
open-folder, and save-destination panels with explicit cancellation results.
The [popover example](../../examples/popover-solid/app.tsx) compares the shared
`Root`/`Trigger`/`Content` JSX API of `SystemPopover` and the retained in-window `Popover`.
The [system API example](../../examples/system-api-solid/app.tsx) demonstrates core-owned shell,
app environment, clipboard, display, notification, menu, tray, shortcut, single-instance,
deep-link, secure-storage, autostart, permission, preference, desktop, power, window, and updater
services from `@quickgui/native` alongside the Solid renderer.
