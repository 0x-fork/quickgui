# @quickgui/cli

`@quickgui/cli` is a TypeScript/Bun tool for Go QuickGUI applications. It creates projects, watches source, invokes `go build` with `CGO_ENABLED=0`, and packages the executable with a matching prebuilt Rust shared library.

## Create a project

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

The scaffold contains `main.go`, `go.mod`, `quickgui.config.ts`, and `package.json`. Pass your root component as `native.WindowOptions{Component: Counter}`. `native.Run` owns application startup. Initialization refuses to overwrite a non-empty directory; `--no-install` skips both dependency installations.

## Development

```console
quickgui dev
quickgui dev --project path/to/app
quickgui dev --once --no-launch
```

On macOS this creates an ad-hoc signed bundle under `.quickgui/dev/<target>/`. Source changes compile and launch a candidate app. The CLI replaces its previous process only after the candidate's first native window is ready; compile/startup failures leave the working app open. Quitting the active app stops that watcher. A small development readiness notification coordinates the CLI; application UI calls always stay inside the Go process through purego.

Components and events run on a dedicated Go goroutine pinned to an OS thread. AppKit/Winit owns the process main thread. Background work dispatches its UI result with `native.Dispatch` or `ui.Async`.

`quickgui fmt` formats Go files recursively using the project's SDK formatter. It wraps long and multiline QuickGUI calls, places callback arguments on separate lines, and then runs standard Go formatting. Use `quickgui fmt --check` to check without writing, or `--project path/to/app` to select a project. Generated files, hidden directories, `vendor`, and `node_modules` are skipped. The SDK must be present in the project's `go.mod`.

From this repository, run `bun run build:native` once to stage the Rust library. Ordinary app edits only rebuild Go. A released `@quickgui/native` package supplies the library; consumers do not need Rust or a C compiler. Go 1.23+, Bun, and macOS Xcode Command Line Tools are required for development/packaging.

## Configuration

```ts
import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "My App",
  identifier: "com.example.my-app",
  entry: ".", // A Go main package, e.g. "cmd/app".
  version: "0.1.0",
  fonts: ["assets/Custom.ttf"],
  resources: ["assets"],
  protocols: ["my-app"],
  native: { tags: ["production"] },
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "14.0",
    signingIdentity: "Developer ID Application: Example (TEAMID)",
    notarization: { keychainProfile: "quickgui-notary" },
  },
});
```

`native.libraryPath` or `QUICKGUI_LIBRARY` selects a custom host library. Otherwise the CLI finds the matching asset in `@quickgui/native` or the repository build output. Rust and Go protocol versions must match. `native.tags` passes Go build tags. Application metadata and packaged font paths are injected at link time. There is no runtime TypeScript compiler, JSX lowering, or native-module code generator.

## Production

```console
quickgui build --target darwin-arm64
quickgui build --target darwin-x64
quickgui build --sign "Developer ID Application: Example (TEAMID)" --notarize quickgui-notary
quickgui build --update-manifest --update-base-url https://dl.example.com/demo
quickgui build --mas
```

Production Go builds use `-trimpath -ldflags='-s -w …'`. macOS packages put the shared library in `Contents/Frameworks` and resources in `Contents/Resources`. The signed `.app` is packaged in a versioned DMG with an Applications link. Notarization uses an existing `notarytool` Keychain profile; development builds do not create DMGs. MAS builds use the configured app/installer identities and entitlements. Signed update manifests require the configured update signing key.

The Go compiler maps `darwin-x64`, `linux-x64`, and `windows-x64` to `GOARCH=amd64`; arm64 targets use `GOARCH=arm64`. A matching native library and target packaging tools are required. Linux AppDir/Debian and Windows installer payloads include the shared library beside the executable. Published native assets currently cover macOS arm64/x64; Linux/Windows runtime and installer acceptance remain platform-specific work.

Use `quickgui dev --help` and `quickgui build --help` for all command flags, and [config.ts](src/config.ts) for typed resource, signing, entitlements, file associations, update, and platform packaging options.
