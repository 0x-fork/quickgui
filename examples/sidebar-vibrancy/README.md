# sidebar-vibrancy

Compare all 15 macOS vibrancy materials in a translucent sidebar alongside an
opaque content pane. Selected rows, SVG checkmarks, and live readouts track the
material and Auto/Active/Inactive visual effect state without rebuilding the window.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/sidebar-vibrancy
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/sidebar-vibrancy test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
