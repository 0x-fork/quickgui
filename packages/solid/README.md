# @quickgui/solid

Solid 2 renderer for QuickGUI. It exports unstyled `View`, `Text`, `Button`, `Input`, `TextArea`,
retained core `Markdown`, variable-height `VirtualList`, compound in-window `Popover` and
native-window `SystemPopover` parts, and `createRenderer`.
Import native application/window APIs from `@quickgui/native` and reactive primitives from
`solid-js` itself.

Real SwiftUI controls are available from the macOS-only subpath:

```tsx
import { Button, Host } from "@quickgui/solid/swift-ui";
import { buttonStyle } from "@quickgui/solid/swift-ui/modifiers";

<Host matchContents>
  <Button
    label="Save changes"
    modifiers={[buttonStyle("glass")]}
    onPress={save}
  />
</Host>;
```

`Host` is one Rust-owned native-layout leaf backed by `NSHostingView`. Use `matchContents` for
intrinsic controls such as `Button`, or size the host with ordinary QuickGUI styles when SwiftUI
content should fill available space.

For simple-label buttons, `Button` follows Expo UI's prop shape (`label`, `systemImage`, `role`,
`target`, `testID`, `onPress`, and `modifiers`). The modifiers subpath currently implements
`buttonStyle`, `buttonBorderShape`, `controlSize`, `labelStyle`, `tint`, and `disabled`.

Ordinary QuickGUI components can be mounted back inside SwiftUI with `QuickGUIHostView`. This is
the counterpart of Expo UI's `RNHostView`: it accepts one QuickGUI subtree, retains a separate
core renderer for it, and supports fixed dimensions or per-axis `matchContents` measurement.
That lets a native SwiftUI popover contain interactive QuickGUI controls while remaining above the
owner window's WGPU scene:

```tsx
import { View, Text } from "@quickgui/solid";
import {
  Button,
  Host,
  Popover,
  QuickGUIHostView,
} from "@quickgui/solid/swift-ui";

<Host matchContents>
  <Popover
    isPresented={open()}
    onIsPresentedChange={setOpen}
    attachmentAnchor="bottom"
    arrowEdge="top"
  >
    <Popover.Trigger render={<Button label="Open" />} />
    <Popover.Content>
      <QuickGUIHostView width={280} height={120}>
        <View style={{ width: 280, height: 120, padding: 16 }}>
          <Text>Ordinary QuickGUI content</Text>
        </View>
      </QuickGUIHostView>
    </Popover.Content>
  </Popover>
</Host>;
```

`Popover.Trigger` follows Base UI's composition contract: `render` supplies the real SwiftUI
component, while the trigger composes the controlled open behavior onto its press event. An
`onPress` on the rendered `Button` still runs first and can call `event.preventDefault()` to cancel
opening.

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
  return (
    <Button onClick={() => setCount(count() + 1)}>Count: {count()}</Button>
  );
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
The [sidebar vibrancy example](../../examples/sidebar-vibrancy-solid/app.tsx) switches every
Electron-compatible macOS vibrancy material behind a transparent Solid sidebar and opaque content
pane, leaving the native material unobstructed. It also demonstrates `borderTopWidth`,
`borderRightWidth`, `borderBottomWidth`, and CSS-like `boxShadow` styles backed by the Rust core.
The [system API example](../../examples/system-api-solid/app.tsx) demonstrates core-owned shell,
app environment, clipboard, display, notification, menu, tray, shortcut, single-instance,
deep-link, secure-storage, autostart, permission, preference, desktop, power, window, and updater
services from `@quickgui/native` alongside the Solid renderer.
The [SwiftUI example](../../examples/swift-ui-solid/app.tsx) is a centered native Liquid Glass
button whose native popover contains an ordinary interactive QuickGUI subtree.
