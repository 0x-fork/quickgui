# alert-dialog

Native information and three-choice Save/Don't Save/Cancel sheets, plus an
application-modal critical alert. Results and cancellation update the retained
status text; pending dialogs disable repeat requests. The critical example does
not delete anything.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/alert-dialog
```

Check the application without CGO:

```console
bun packages/cli/src/cli.ts test --project examples/alert-dialog
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
