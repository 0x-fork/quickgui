# Project CLI and application packaging

`@quickgui/cli` creates Solid projects, runs a reloadable native development application, and
packages production applications for a target platform.

## Create a project

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

The scaffold imports `App`, `Window`, `View`, `Text`, `Button`, and `render` from
`@quickgui/solid`. Solid reactivity such as `createSignal` comes directly from `solid-js`.

Initialization refuses to overwrite a non-empty directory. Use `--no-install` when dependency
installation belongs to another workflow.

## Development application

```console
quickgui dev
```

On macOS, development first creates and ad-hoc signs a real bundle at
`.quickgui/dev/darwin-arm64/<executable>.app`. The CLI directly owns the process created from that
bundle's `Contents/MacOS` executable, so the running GUI has normal bundle and `Info.plist` context.

The development executable contains two compiled Bun entrypoints: a minimal native host and the
configured TS/TSX application Worker. Ordinary source edits therefore follow this sequence:

1. compile the edited source and package a candidate `.app`;
2. start the candidate process with AppKit/Winit on its main thread and application code in a Bun
   Worker;
3. wait until QuickGUI completes the first native window event-loop turn;
4. stop only the previous process owned by this CLI session.

There is no soft in-isolate reload. If the new source cannot compile or exits before a window is
ready, the candidate is discarded and the last working app stays open. The generated `.app` is
self-contained and also works when launched directly; use `quickgui dev` when the CLI should own
watching and candidate-first process replacement. Quitting the active app also stops the watcher;
an older app terminated as part of a successful reload does not.

AppKit/Winit permanently owns the process main thread. Bun timers, fetch, streaming, and other
application work run on the Worker's normal event loop. Bounded command/event queues and a Winit
proxy wake join the two without periodic native pumping.

Available development options:

```console
quickgui dev --project path/to/app
quickgui dev --config quickgui.config.ts
quickgui dev --sign "Apple Development: Example"
quickgui dev --once
quickgui dev --once --no-launch
```

Development runs only the host architecture. Use `build --target` to cross-compile artifacts.

## Production package

```console
quickgui build
quickgui build --target darwin-arm64
quickgui build --target windows-x64 --out-dir artifacts
```

Production compilation embeds the application, Solid runtime, Bun runtime, and selected N-API
addon. macOS output is a signed `.app`; Linux and Windows output a native executable. Ad-hoc signing
is the macOS default. Pass `--sign <identity>` or set `macos.signingIdentity` for a distributable
identity. Notarization and installer generation remain release-pipeline responsibilities.

Recognized targets are `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`,
`windows-arm64`, and `windows-x64`. A build is available only when the installed
`@quickgui/native` package contains the corresponding addon; target recognition is not a claim of
native runtime acceptance.

## Configuration

Create `quickgui.config.ts` at the project root:

```ts
import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "My App",
  identifier: "com.example.my-app",
  entry: "src/app.tsx",
  outDir: "dist",
  version: "0.1.0",
  buildVersion: "1",
  resources: ["assets"],
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "13.0",
    category: "public.app-category.developer-tools",
    signingIdentity: "-",
    entitlements: "Entitlements.plist",
  },
  windows: {
    icon: "assets/app.ico",
    hideConsole: true,
  },
});
```

Resource files and directories are copied by basename into the application resources directory.
Two configured resources may not use the same destination name.

Return to the [documentation index](README.md).
