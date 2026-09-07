# system-api

Native application, display, preference, permission, power, clipboard, notification,
desktop, credential, autostart, deep-link, shortcut, and tray APIs through typed Go
bindings. The example retains the original 20 integration actions and adds the
keyboard-layout query. Native menus and second-instance, power, preference,
notification, and deep-link events update the same window.

The sleep assertion, shortcut, tray icon, and Dock badge are released when the
window closes. Persistent actions such as autostart registration and credential
storage run only when their buttons are pressed.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/system-api
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/system-api test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
