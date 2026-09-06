# @quickgui/ui

Signals, typed JSX, and unstyled native components for applications compiled with QuickGUI.
The CLI lowers JSX with TypeScript 7 and compiles it with scriptc. The Rust core owns rendering,
input, accessibility, and native component behavior.

```tsx
import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer, createSignal } from "@quickgui/ui";

function Counter() {
  const [count, setCount] = createSignal(0);
  return <View><Text>{count()}</Text><Button onClick={() => setCount(count() + 1)}>Add</Button></View>;
}

await app.whenReady();
new Window({ title: "Counter", renderer: createRenderer(() => <Counter />) });
```

Compound families have dedicated imports, such as `@quickgui/ui/date-time`,
`@quickgui/ui/dialog`, `@quickgui/ui/router`, and `@quickgui/ui/swift-ui`.
Dynamic custom component props use accessors; setters accept values. `KeyedFor` retains rows
through immutable updates by exposing each item as an accessor.

See the [UI guide](../../docs/ui.md), [CLI guide](../../docs/cli.md), and
[components example](../../examples/components). Bun runs tooling; the application is native code.
