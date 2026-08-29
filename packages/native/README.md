# @quickgui/native

Low-level Bun/N-API host for QuickGUI. AppKit/Winit permanently owns the process main thread while
the application runs on Bun's ordinary Worker event loop. Bounded native command and event queues
connect the two; a Winit proxy wakes the main thread only when Worker work arrives. Settled
applications therefore block without a polling interval, timer shim, or descriptor scan. The host
retains a JavaScript node tree, sends bounded binary mutation batches to Rust, and routes bounded
native events to independent `Window` trees. Native input nodes route
controlled value payloads, including masked password fields, and native Markdown nodes retain
QuickGUI core parser/render state across mutations. `new Window({ anchor: node, ... })` creates a display-aware,
parent-owned native popover. Every window receives a renderer adapter through
`new Window({ renderer, ... })`, owns the returned cleanup function, and disposes it when the
window closes. Its initial renderer batch is committed before the native window opens, preserving
one content-complete first-frame boundary. The `Dialog` namespace exposes `showAlertDialog`, `showOpenDialog`, and
`showSaveDialog`; each accepts an optional leading `Window`, matching Electron's parented and
application-modal call shapes. Alert dialogs resolve with a button index, while file dialogs use
Electron-shaped result objects. The file-dialog backend uses QuickGUI's AppKit panels on macOS and
`rfd` on Windows and Linux. The portable alert backend supports up to three uniquely labelled
buttons. Linux file dialogs prefer XDG Desktop Portals (with RFD's Zenity fallback), while Linux
alert dialogs require Zenity because the portal API has no standardized message-dialog surface.

System services are core-first and exported from this package: singleton `app` lifecycle and
single-instance locking, `Appearance`, `AutoStart`, `Clipboard`, `DeepLink`, `GlobalShortcut`,
`Keyboard`, `Menu`, `Notifications`, `PowerMonitor`, `Screen`, `SecureStorage`, `Shell`, `Tray`,
`Updater`, and imperative `Window` controls. The updater discovers and stages signed artifacts;
package-format-specific installation remains the application's responsibility.

Most applications should depend on this package and [`@quickgui/solid`](../solid/README.md)
directly. Import `app`, `Window`, dialogs, and platform APIs here, await `app.whenReady()`, then pass
Solid's `createRenderer(() => <App />)` to `Window`. The QuickGUI CLI owns the application loop for
packaged and development applications, so application code never calls `app.run()`. The native
package remains the renderer-neutral layer for additional JavaScript reconcilers.

From the repository root:

```console
bun run build:native
bun test packages/native
```

The native package declares macOS, Windows, and Linux targets. A source checkout only needs the
host `.node` build for development; release packaging must supply the target-specific addons used
by the CLI. The source protocol is versioned and malformed batches are rejected transactionally
before the committed retained tree changes.

On macOS, the build script detects a selected beta Xcode and prefers `/Applications/Xcode.app`
when it is available. Set `QUICKGUI_ALLOW_BETA_XCODE=1` to opt out of that fallback.
