# @quickgui/cli

Project scaffolding, development, and production packaging for QuickGUI applications written with
Solid 2.

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

## Development

`quickgui dev` creates a genuine native development application. On macOS this is an ad-hoc-signed
`.app` under `.quickgui/dev/<target>/` and the CLI runs its `Contents/MacOS` executable.

Each development build compiles two Bun entrypoints into the bundle's executable: a minimal native
main-thread host and the current TS/TSX application Worker. Source edits create and sign a candidate
`.app`; after its first native window completes an event-loop turn, the CLI stops the prior process.
A compile or startup failure leaves the prior app running. The generated `.app` is self-contained
and can also be launched directly from Finder or LaunchServices.

AppKit/Winit stays permanently on the process main thread. Bun timers, fetch, streaming, and other
application work stay on the Worker's supported event loop. Bounded native queues and an explicit
Winit wake connect them without an idle polling interval.

QuickGUI uses candidate-first process restart instead of in-isolate hot replacement because
AppKit/Winit application state is process-owned.

## Production builds

`quickgui build` compiles the application, Solid renderer, Bun runtime, and target N-API addon into
a self-contained executable. A macOS target is wrapped in a signed `.app` under
`dist/<target>/`.

```console
bun run build --target darwin-arm64
bun run build --target windows-x64
```

A target build requires the installed `@quickgui/native` package to contain that target's addon.

## Configuration

```ts
import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "My App",
  identifier: "com.example.my-app",
  entry: "src/app.tsx",
  version: "0.1.0",
  resources: ["assets"],
  protocols: ["my-app"],
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "13.0",
  },
});
```

`protocols` is written into a signed macOS app's `CFBundleURLTypes`. On Windows and Linux, register
the same scheme at runtime with `DeepLink.register(...)`; macOS registration is intentionally
declarative because Launch Services reads it from the application bundle.

See the [CLI guide](../../docs/cli.md) for every command and option.
