# {{NAME}}

A standalone {{TYPE}} native service with a Go wrapper and a QuickGUI demo.
The provider exports the public service ABI and loads through purego, without
CGO, IPC, or changes to the QuickGUI core.

## Develop

Install Go 1.23 or newer, Bun 1.3 or newer, and {{COMPILER_REQUIREMENT}}.

```sh
bun install
go mod tidy
bun run dev
```

`bun run dev` builds the native library once and launches the demo. Go edits
reuse that library. Restart the command after changing native code.
`bun run build` builds just the extension; `bun run fmt` formats the Go code.
All tooling scripts are TypeScript run with Bun.

The layout is:

- `extension.go`: typed Go API and import-time declaration; native loading is deferred.
- `quickgui.extension.json`: provider identity, ABI, npm package, and exact release version.
- `native/`: independent service implementation and vendored ABI definitions.
- `scripts/`: build and development commands.
- `artifacts/`: publishable npm package containing `lib/<platform>-<arch>/`.
- `cmd/demo/`: runnable Go application; `quickgui.toml` points to it.

The build script generates native identity/version constants from the manifest
and synchronizes `artifacts/package.json`. Always build through `bun run build`.
The Go wrapper embeds the same manifest, so it requires that exact provider version.

## Extend the service

Replace the sample `echo` method with your API. It returns the original JSON
payload and handles errors through the reply sink. The ABI is C-compatible;
the provider does not depend on QuickGUI's renderer or internal Rust types.

Inputs are borrowed for the call. Copy them before starting asynchronous work.
Emit JSON success replies or UTF-8 errors, then release each sink exactly once,
including error paths. Enqueue slow work and cancel it during shutdown without
blocking the UI thread. Keep request and reply payloads at or below 64 KiB.

## Distribute

1. Set your real Go module path in `go.mod` and `cmd/demo/view.go`.
2. Set the npm package and release version in `quickgui.extension.json`.
3. Run `bun run build` on each supported target machine. Builds target the host;
   collect their `artifacts/lib/<platform>-<arch>/` directories into one package.
4. Publish `artifacts/` to npm and tag the Go module with the matching release.

For example, after collecting the supported binaries:

```sh
cd artifacts
npm publish --access public
```

Consumers import the Go wrapper:

```sh
go get {{GO_MODULE}}@v0.1.0
bun add {{NPM_PACKAGE}}@0.1.0
```

The CLI discovers the manifest through Go imports and bundles the exact native
artifact alongside the application. Installing the artifact package explicitly
is optional: the CLI can fetch its exact version from npm during packaging.
For local builds, `scripts/dev.ts` sets `QUICKGUI_EXTENSION_DIR` to the built
target directory. The root npm package remains private development tooling.

See [extension authoring](https://github.com/egoist/quickgui/blob/main/website/src/content/docs/go/en/extensions.mdx) for sessions,
resource bundles, platform support, and ABI lifecycle rules.
