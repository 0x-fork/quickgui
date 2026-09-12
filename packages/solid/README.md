# @quickgui/solid

Solid 2's universal renderer for QuickGUI native desktop windows. It includes the complete component families: primitives, forms, selection, pickers, layout, virtual collections, menus, overlays, feedback, routing, and native SwiftUI controls.

The CLI compiles JSX for retained native nodes and selects Solid's client runtime inside Bun. The packages pin Solid `2.0.0-rc.8`.

Use `Window` and `app` from `@quickgui/native` with `createRenderer` from this package. Routing is also available from `@quickgui/solid/router`; native SwiftUI controls and modifiers are in `@quickgui/solid/swift-ui` and `@quickgui/solid/swift-ui/modifiers`.

See the [TypeScript guide](../../docs/typescript.md), [components gallery](../../examples/components-typescript), and [Quick Git](../../examples/quick-git-typescript).

## Intrinsic elements and text

`<View>some text</View>` accepts strings directly. `<div>` aliases `View` and `<span>` aliases `Text`, with no component imports:

```tsx
<div style={{ flexCol: true, gap2: true }}>
  some text
  <span style={{ textLg: true, fontSemibold: true }}>Styled text</span>
</div>
```

Use the usual `jsxImportSource: "@quickgui/solid"` configuration. All visual declarations use camelCase `style` fields. Component state, accessibility, and handlers remain ordinary props; direct style attributes, `class`, and `className` are unsupported.
