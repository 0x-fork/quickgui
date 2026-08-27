# Solid 2 renderer

QuickGUI includes an experimental Bun host split into two unstyled packages:

- `@quickgui/native` owns the N-API boundary, application loop, per-window retained trees, binary
  mutation batches, and window-routed event queue.
- `@quickgui/solid` owns Solid 2 JSX compilation, fine-grained reactive updates, and the host
  components `View`, `Text`, and `Button`.

The renderer does not use a webview or virtual DOM. Solid updates the affected retained native
nodes, and one binary batch crosses N-API before QuickGUI invalidates the WGPU window.

## Run an application

For application development, the preferred path is the [QuickGUI CLI](cli.md):

```console
quickgui dev
```

On macOS this runs a real signed `.app` whose stable host loads the project TSX directly from disk.
Source changes restart that host without rebundling or repackaging the application. The CLI owns
the Solid compiler setup, so applications do not need a Bun preload or a special start command.

```tsx
import { App, Button, Text, View, Window, render } from "@quickgui/solid";
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

`App` owns the native event loop and establishes the application context for its JavaScript
isolate. Window configuration and rendered state belong to `Window`. Constructing a `Window` works
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

`View`, `Text`, and `Button` are intentionally unstyled. Their `style` prop uses web-shaped names
for the currently bridged QuickGUI layout, text, paint, overflow, cursor, positioning, and
`appRegion` properties. Native events are flushed at a Solid 2 event boundary before the retained
mutation batch is submitted.

The runnable source and CLI configuration are in
[`examples/solid`](../examples/solid).

## Current boundary

This first vertical slice supports dynamically created independent native windows, retained
view/text/button nodes, reactive properties and text, click and hover events, web-shaped Flexbox
styling, hidden-inset titlebars, and traffic-light positioning. It is not yet the full Rust API
surface: inputs, lists, menus, popovers, native child views, accessibility actions, packaging of
every native binary target, and production performance gates still need dedicated bindings.

Return to the [documentation index](README.md).
