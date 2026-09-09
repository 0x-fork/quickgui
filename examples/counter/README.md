# counter

Node-returning components, plain value props, fluent styles, signals, and multiple windows.

The application is Go. Components return `*ui.Element` or `*native.Node`. Pass them directly through `native.WindowOptions.Component`; plain value props and signal reads update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/counter
```

Check the application without CGO:

```console
bun packages/cli/src/cli.ts test --project examples/counter
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
