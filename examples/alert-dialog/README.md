# alert-dialog

Native alert sheets with asynchronous results.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/alert-dialog
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/alert-dialog test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
