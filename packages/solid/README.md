# @quickgui/solid

Solid 2 renderer for QuickGUI. It exports unstyled `View`, `Text`, and `Button` host components,
plus `App`, `Window`, and `render`. Import reactive primitives from `solid-js` itself.

Run TSX through the package's Bun preload and select Solid's interactive client condition:

```console
bun --conditions=browser --preload @quickgui/solid/register app.tsx
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
