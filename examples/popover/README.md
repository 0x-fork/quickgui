# popover

Compare a controlled in-window popover with an anchored native system popover. Both have independent counters, changing trigger labels, and open/close status. Escape, outside clicks, and Close dismiss the surfaces; reopening starts a fresh local counter. The in-window surface flips and shifts within its owner, while the system surface can cross the window edge.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/popover
```

Check the application without CGO:

```console
bun packages/cli/src/cli.ts test --project examples/popover
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
