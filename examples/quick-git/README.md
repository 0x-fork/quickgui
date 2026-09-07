# quick-git

A native Git client with changes/diffs, staging, history, branches, worktrees, stashes, and commit-message generation. Git commands run in bounded background workers; filesystem changes arrive from the Rust OS watcher.

The application is Go and its interface uses QuickGUI components throughout, including the toolbar buttons, SVG icons, and progress indicator. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/quick-git
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/quick-git test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
