# TypeScript with Bun and Solid 2

The TypeScript frontend uses Bun, Solid 2, and the same Rust renderer as Go. Bun calls the shared library through `bun:ffi` in the same process. JSX produces retained native nodes without a DOM or WebView.

## Start a project

```sh
quickgui init my-app --frontend typescript
cd my-app
bun run dev
```

In this source checkout, run `bun install` and `bun run build:native` once, then:

```sh
bun packages/cli/src/cli.ts dev --project examples/counter-typescript
```

Use Bun 1.4 or later. Solid, `@solidjs/compiler`, and `@solidjs/universal` are pinned to `2.0.0-rc.7`. Solid 2 is a release candidate, and Bun labels its FFI API experimental. The frontend targets the repository's macOS workflow; Windows/Linux native runtime acceptance remains outstanding.

## Components and windows

```tsx
import { createSignal } from "solid-js";
import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";

function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <View display="flex" flexDirection="column" padding={24} gap={12}>
      <Text fontSize={24}>Count: {count()}</Text>
      <Button onClick={() => setCount(count() + 1)}>Increment</Button>
    </View>
  );
}

function openWindow() {
  new Window({ title: "Counter", width: 640, height: 460, renderer: createRenderer(Counter) });
}
app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openWindow();
});
await app.whenReady();
openWindow();
```

Components construct once. Solid tracks JSX expressions and updates affected native properties or child edges. Use Solid 2's `Show`, `For`, signals, effects, and cleanup APIs. Each window has an independent Solid root, disposed when the native window closes. The Dock handler creates a fresh window after the last window closes on macOS.

The frontend exposes the complete native component families:

- Primitives: `View`, `Text`, `Button`, `Input`/`TextInput`, `TextArea`, `Markdown`, `Image`, `Svg`, `Shader`, `VirtualList`, and optional `Terminal`.
- Forms: `Field`, `Fieldset`, `Checkbox`, `CheckboxGroup`, `Radio`, `RadioGroup`, `Switch`, `NumberField`, `OtpField`, `Select`, `Combobox`, and `Autocomplete`.
- Layout and data: `Tabs`, `Accordion`, `Collapsible`, `Separator`, `Splitter`, `ScrollArea`, `Table`, `Tree`, `Calendar`, `DateField`, and `TimeField`.
- Menus and overlays: `Menu`, `Menubar`, `ContextMenu`, `NavigationMenu`, `PopoverMenu`, `Popover`, `SystemPopover`, `Dialog`, `AlertDialog`, `Tooltip`, `PreviewCard`, and `Drawer`.
- Feedback: `Avatar`, `Progress`, `Meter`, `Toggle`, `ToggleGroup`, `Toolbar`, and `Toast`.

Compound components expose their named parts and typed state hooks. Rust owns selection, keyboard navigation, focus, layout, placement, and interaction; Solid owns construction and reactive bindings. The [components gallery](../examples/components-typescript/app.tsx) demonstrates the complete control set.

`@quickgui/solid/router` supplies routes, nested outlets, links, and navigation hooks backed by Rust route matching and memory history. `@quickgui/solid/swift-ui` exposes native controls, popovers, and reverse QuickGUI hosting; typed modifier factories are in `@quickgui/solid/swift-ui/modifiers`.

Use native camelCase properties directly or reusable `NativeStyle` objects:

```tsx
const panel = { padding: 16, borderRadius: 12, backgroundColor: "#18181b" };
<View style={panel} textColor="#fafafa"><Text>Hello</Text></View>
```

Direct properties override `style`. Removing a style field clears its native value; removing a direct property reveals its style value. Colors accept CSS hex forms or an integer packed as `0xAABBGGRR`, matching Rust. Property IDs and bounds are generated from Rust with `bun scripts/generate-typescript.ts`.

Input handlers receive native events, with text in `event.value`:

```tsx
<TextInput value={name()} onInput={event => setName(event.value ?? "")} />
```

`Window.close()`, `Window.setTitle()`, `Window.getState()`, `app.quit()`, and `app.exit()` use the native host. Operations returning values are asynchronous. `Window.whenReady()` resolves when Rust has mounted the window, including hidden windows; snapshot getters await that event; setters queue their native work until creation. `app.command()` exposes existing native JSON commands for advanced use.
Native window snapshots use `bounds`, `viewportSize`, and `scaleFactor`. Observe close completion with `window.onClose()` or `window.on("closed", ...)`. Menus, clipboard, file and alert dialogs, display information, appearance, notifications, global shortcuts, tray icons, permissions, power assertions, secure storage, and metrics are available from `@quickgui/native`.

Declare optional providers with `native.extensions: ["terminal"]` or `["updater"]`, and install their corresponding native packages. `ExtensionSession` and `invokeExtension` expose other provider services through the shared C ABI. `Updater` uses the current Rust updater extension.

The [Quick Git example](../examples/quick-git-typescript/README.md) includes changes and history, partial staging, branches, stashes, worktrees, native menus and dialogs, independent per-window stores, and cancellable Bun subprocesses. Its interface follows the current Go example, including native splitters, the QuickGUI toolbar and composer, virtualized commit files, and history selection that survives focus refresh.

## Build and check

Set `frontend: "typescript"` in `quickgui.config.ts`, or `frontend = "typescript"` in TOML. The default entry is `app.tsx`. Use `jsx: "preserve"` and `jsxImportSource: "@quickgui/solid"` in TypeScript configuration, as in the scaffold.

```sh
bun run check
bun run test
bun run build
```

`quickgui check` runs the project's TypeScript compiler. `quickgui test` preloads the same Solid compiler and reactive client runtime as builds. `quickgui fmt` uses the project's `oxfmt`. Production builds embed Bun and both host/worker entrypoints in one executable and bundle the native library alongside it. App edits rebuild only the executable.

macOS signing includes Bun's JIT and unsigned-executable-memory entitlements by default. If you supply `macos.entitlements`, include those keys in your file. Developer ID signing/notarization and Mac App Store distribution require their own platform acceptance.

## Runtime ownership

The host waits for the worker startup handshake before entering the native loop on the process main thread. The worker owns Solid, handlers, promises, timers, and application I/O. Both isolates load the same Rust library; UI commands enter the existing bounded host queue directly through FFI.

Rust copies native spans into a queue bounded to 8,192 events and 32 MiB. A coalesced, zero-argument thread-safe callback wakes the worker, which drains events with synchronous callbacks that copy data before returning. No borrowed pointer survives a callback, and there is no polling timer. Overflow fails the application explicitly. Shutdown disposes windows and pending requests before terminating the worker.

Framework checks: `bun run typecheck:js`, `bun run test:js`, `bun run test:typescript`, `bun scripts/generate-typescript.ts --check`, and `scripts/with-macos-ghostty-zig.sh cargo test -p quickgui-host --lib`.

After staging the native library, `bun scripts/check-typescript.ts` checks compiled development and production workers against real native windows, asynchronous replies, and shutdown. Its windows remain hidden.

## Complete examples and reference

- [Components gallery](../examples/components-typescript/README.md): 44 demos, including all Go gallery entries and Drawer.
- [Quick Git](../examples/quick-git-typescript/README.md): changes, partial staging, history, branches, stashes, worktrees, and multiple windows.
- The website exposes `/docs/typescript` and 69 component API pages, with source-derived signatures and TypeScript examples.

Run `bun scripts/check-typescript-examples.ts` for native lifecycle/content validation of both applications.
