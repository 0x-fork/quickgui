# QuickGUI SwiftUI

This example embeds a real SwiftUI Liquid Glass `Button` in QuickGUI, presents a native popover,
then reverse-hosts ordinary QuickGUI `View`, `Text`, `Input`, and `Button` components inside it.
The embedded subtree keeps its Rust renderer and native input/accessibility surface instead of
being translated into SwiftUI controls.

```console
cd examples/swift-ui-solid
bun run dev
```

The `glass` style uses Liquid Glass on macOS 26 and falls back to a bordered native button on older
systems. Dismiss the popover by clicking outside it, or use the QuickGUI Save button inside it.
