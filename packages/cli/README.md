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

The app source is not bundled into that development application. The host loads the TS/TSX entry
from disk with the built-in Solid compiler on every launch. Source edits therefore start a candidate
process without rebuilding or repackaging the `.app`; once its first native window is ready, the
CLI stops the prior process. A compile or startup failure leaves the prior app running. Changes to
bundle metadata, icons, entitlements, or copied resources repackage the host because those values
belong to the native bundle.

The bundle contains a small development manifest with the external project paths, so it can also
be launched directly from Finder or LaunchServices. Direct launch runs the current source once;
the CLI must remain in charge for watching and candidate-first restart.

QuickGUI uses process restart instead of in-isolate hot replacement because AppKit/Winit application
state is process-owned.

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
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "13.0",
  },
});
```

See the [CLI guide](../../docs/cli.md) for every command and option.
