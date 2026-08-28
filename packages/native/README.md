# @quickgui/native

Low-level Bun/N-API host for QuickGUI. AppKit/Winit permanently owns the process main thread while
the application runs on Bun's ordinary Worker event loop. Bounded native command and event queues
connect the two; a Winit proxy wakes the main thread only when Worker work arrives. Settled
applications therefore block without a polling interval, timer shim, or descriptor scan. The host
retains a JavaScript node tree, sends bounded binary mutation batches to Rust, and routes bounded
native events to independent `Window` trees. Native input nodes route
controlled value payloads, including masked password fields, and native Markdown nodes retain
QuickGUI core parser/render state across mutations. `new Window({ anchor: node, ... })` creates a display-aware,
parent-owned native popup. Its public lifecycle remains `new App()`, `new Window(options)`, and
`window.close()`.

Most applications should use [`@quickgui/solid`](../solid/README.md). The native package is the
renderer-neutral layer for additional JavaScript reconcilers.

From the repository root:

```console
bun run build:native
bun test packages/native
```

The current binary target is macOS. The source protocol is versioned and malformed batches are
rejected transactionally before the committed retained tree changes.

On macOS, the build script detects a selected beta Xcode and prefers `/Applications/Xcode.app`
when it is available. Set `QUICKGUI_ALLOW_BETA_XCODE=1` to opt out of that fallback.
