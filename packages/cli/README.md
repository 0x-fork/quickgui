# @quickgui/cli

`@quickgui/cli` supports Go and TypeScript QuickGUI applications. It creates projects, watches source, compiles the selected frontend, and packages the executable with matching prebuilt native libraries. Go builds with `CGO_ENABLED=0`; TypeScript uses Bun and Solid 2. See the [TypeScript guide](../../docs/typescript.md).

## Create a project

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

The CLI asks you to choose Go or TypeScript. Pass `--frontend go` or `--frontend typescript` to skip the prompt; an explicit frontend is required in non-interactive environments.

The Go scaffold contains `main.go`, `go.mod`, `quickgui.config.ts`, and `package.json`. Pass your root component as `native.WindowOptions{Component: Counter}`. `native.Run` owns application startup. Initialization refuses to overwrite a non-empty directory; `--no-install` skips dependency installation and preparation.

## Create an extension

```console
quickgui init-extension my-components --type go
quickgui init-extension my-service --type zig
quickgui init-extension my-service --type rust --module github.com/acme/my-service --npm-package @acme/extension-my-service
```

`go` is the default. It creates a reusable component library and a reactive demo. `zig` and `rust` create independent service libraries, Go wrappers, manifests, and publishable native artifact packages. Each project includes a `cmd/demo` application and `quickgui.toml`; run `bun run dev` inside it. All build scripts are TypeScript executed with Bun.

Use `--name` to override the extension name, `--module` for the Go module path (default `example.com/<name>`), and `--npm-package` for a native artifact package (default `quickgui-extension-<name>`). `--no-install` skips both `bun install` and `go mod tidy`. Existing non-empty directories are preserved.

The Zig template targets Zig 0.16.x; the Rust template uses stable Cargo/Rust. Both produce native extensions usable by Go and TypeScript applications.

Native templates build for the current machine and stage their library in `artifacts/lib/<target>/`. The manifest controls the extension release and the generated native version constants. Application edits reuse the built library; restart `bun run dev` after native changes. Pure Go extensions need no native build toolchain or artifact package.

In this unpublished checkout, run `bun packages/cli/src/cli.ts init-extension <directory> --type <type> --no-install`. Before running the demo, point the generated Go SDK requirement to this checkout with a `replace` directive, set `@quickgui/cli` to a local `file:` dependency, and run the two installation commands. The integration check `bun scripts/check-extension-templates.ts` exercises all three templates from a packed CLI using the local SDK and staged core library.

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

## Optional native extensions

Import `github.com/egoist/quickgui/go/terminal` to use `terminal.View(terminal.Props{…})`. The core UI package does not import the terminal backend. The CLI examines the actual Go dependency graph, including transitive imports, target files, and build tags, then bundles the required extension libraries beside the core library.

Terminal uses the separate `@quickgui/extension-terminal` package. The CLI uses an installed package or downloads the exact SDK-matched version on first use, verifies its SHA-512 integrity, and caches it. Apps without that import neither download nor bundle it. Ordinary Go edits reuse these prebuilt artifacts; no native build or feature-combination matrix is needed.

For offline builds, install the matching `@quickgui/extension-terminal` version beforehand, or put the target libraries in `QUICKGUI_EXTENSION_DIR`. `QUICKGUI_CACHE_DIR` selects the download cache root (default: `~/.cache/quickgui`). Downloads and extracted libraries are limited to 128 MiB each, and the extension cache evicts old libraries above 512 MiB. At runtime, libraries are loaded locally through purego; there is no network access or IPC.

From a source checkout, build each native artifact once:

```console
bun packages/native/build.ts
bun packages/native/build.ts --extension terminal
bun packages/cli/src/cli.ts dev --project examples/herdr-gui
```

The built-in terminal requires the matching core release. Third-party services can use their own names, release versions, and npm scopes with the public service ABI. Put a `quickgui.extension.json` manifest beside the imported Go package and call `host.RequireExtension("your-extension", "1.0.0")`; the CLI bundles it and the core registers it automatically. See the [authoring guide](../../website/src/content/docs/go/en/extensions.mdx) and [standalone C/Go example](../../examples/native-extension/). Packaged apps include their selected libraries and work without Bun, Go, Rust, or npm installed.

## Configuration

Both `quickgui.toml` and `quickgui.config.ts` are supported. `dev` and `build` look for `quickgui.toml` first, then `quickgui.config.ts`. Pass `--config path/to/file.toml` (or a TypeScript file) to select one explicitly. Both formats use the same option names and validation; relative paths are resolved from the project directory. The application scaffold uses TypeScript configuration; extension demos use TOML.

A `quickgui.toml` can contain:

```toml
name = "My App"
identifier = "com.example.my-app"
entry = "."
version = "0.1.0"
fonts = ["assets/Custom.ttf"]
resources = ["assets"]
protocols = ["my-app"]

[native]
tags = ["production"]

[macos]
icon = "assets/AppIcon.icns"
minimumSystemVersion = "14.0"
signingIdentity = "Developer ID Application: Example (TEAMID)"

[macos.notarization]
keychainProfile = "quickgui-notary"
```

Use quoted strings for `version` and `buildVersion`. Nested options use TOML tables; document types use `[[documentTypes]]` array entries. Config changes are reloaded during `dev`, and malformed TOML reports a parse error instead of falling back to another file.

The equivalent TypeScript configuration is:

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
