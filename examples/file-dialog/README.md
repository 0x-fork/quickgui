# file-dialog

Native multi-file open with text/all-file filters, an application-modal folder
picker, and a save panel with a suggested filename and text filter. Results and
cancellation update the retained status text; choosing a destination writes no file.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/file-dialog
```

Check the application without CGO:

```console
bun packages/cli/src/cli.ts test --project examples/file-dialog
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
