# components

A gallery of 43 demos: all 42 from the original example, now written in Go, plus a native system context menu. It includes forms, pickers, menus, overlays, layout controls, a 5,000-row virtual table, and a lazy tree.

The sidebar preserves the original demos. Each demo includes interactive state readouts, keyboard behavior, and light/dark appearance. **System Context Menu**, next to Context Menu, uses `native.Window.PopupMenu` for operating-system menu rendering, checked items, a submenu, disabled actions, and close/error feedback. Right-click its target or activate **Open system menu** with the keyboard.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The prebuilt native library is loaded in process using purego; application builds do not use CGO.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/components
```

Check the application without CGO:

```console
bun packages/cli/src/cli.ts test --project examples/components
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
