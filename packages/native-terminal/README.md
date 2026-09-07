# @quickgui/native-terminal

Prebuilt PTY and Ghostty backend for `github.com/egoist/quickgui/go/terminal`.
The CLI discovers the Go import, caches the matching release artifact, and bundles
this library beside the core. Applications without the import do not need this package.

Build from a checkout with `bun packages/native/build.ts --extension terminal`.
The extension shares the existing renderer through a versioned C ABI and does not
include another QuickGUI/WGPU runtime. Go loads it in process through purego.
