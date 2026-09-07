# components

A gallery of all 42 component demos from the original example, now written in Go: forms, pickers, menus, overlays, layout controls, a 5,000-row virtual table, and a lazy tree.

The sidebar preserves the original demo list. Each demo includes interactive state readouts, keyboard behavior, and light/dark appearance.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The prebuilt native library is loaded in process using purego; application builds do not use CGO.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/components
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/components test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
