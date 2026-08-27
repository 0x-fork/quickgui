# @quickgui/solid

Solid 2 renderer for QuickGUI. It exports unstyled `View`, `Text`, and `Button` host components,
plus `App`, `Window`, and `render`. Import reactive primitives from `solid-js` itself.

Run applications through the QuickGUI CLI:

```console
bun run dev
```

```tsx
import { App, Button, Window, render } from "@quickgui/solid";
import { createSignal } from "solid-js";

const app = new App();
const window = new Window({ title: "QuickGUI" });
const [count, setCount] = createSignal(0);
render(() => <Button onClick={() => setCount(count() + 1)}>Count: {count()}</Button>, window);
await app.run();
```

See the [Solid renderer guide](../../docs/solid.md) and the
[runnable example](../../examples/solid/app.tsx).
