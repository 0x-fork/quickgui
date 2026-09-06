# QuickGUI sidebar vibrancy

This example exercises every Electron-compatible macOS vibrancy material plus the `followWindow`,
`active`, and `inactive` visual-effect states. QuickGUI keeps one core-owned `NSVisualEffectView`
behind the stable Winit/Metal rendering view and switches its semantic material without recreating
the retained tree or native window.

The root view and sidebar remain transparent, while the content pane stays opaque. That leaves the
native material unobstructed in the sidebar and confines it visually to that region. AppKit
deprecates the legacy `appearance-based` material in favor of semantic types, but QuickGUI retains
it for Electron API compatibility.

```console
cd examples/sidebar-vibrancy
bun run dev
```
