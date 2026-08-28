# Solid 2 renderer

QuickGUI includes an experimental Bun host split into two unstyled packages:

- `@quickgui/native` owns the N-API boundary, application loop, per-window retained trees, binary
  mutation batches, and window-routed event queue.
- `@quickgui/solid` owns Solid 2 JSX compilation, fine-grained reactive updates, and the host
  components `View`, `Text`, `Button`, `Input`, `TextArea`, and `Markdown`.

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
import { App, Button, Text, View, Window, render, type NativeNode } from "@quickgui/solid";
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

The runnable source and CLI configuration are in
[`examples/solid`](../examples/solid).

The [Solid AI chat example](../examples/ai-chat-solid) adds Vercel AI SDK/DeepSeek streaming, an
anchored provider-settings popup with a revealable password field, controlled input, cancellation,
paced updates, and core Markdown rendering.

## Current boundary

This vertical slice supports dynamically created independent and anchored native windows, retained
view/text/button/input/Markdown nodes, password inputs, reactive properties and text,
click/hover/input/submit events, web-shaped Flexbox styling, hidden-inset titlebars, traffic-light
positioning, a stable real-`.app` development host, and self-contained production packaging on the
current macOS target. It is not yet the full Rust API surface: declarative popup parts, lists, menus,
native child views, accessibility actions, every native binary target, and dedicated JavaScript
performance gates still need bindings and acceptance.

Return to the [documentation index](README.md).
