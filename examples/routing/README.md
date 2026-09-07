# routing

Route matching and history with active navigation links, back/forward controls,
dynamic parameters, query updates, replacement navigation, nested settings routes,
and a wildcard fallback. Shared layouts and their local state remain mounted while
the outlet changes; parameter and query updates keep the current page mounted.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/routing
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/routing test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
