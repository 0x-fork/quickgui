# @quickgui/native

Prebuilt Rust shared libraries for the Go QuickGUI frontend. The TypeScript CLI selects the target asset and bundles it with an application. Go loads it in process through purego, with `CGO_ENABLED=0`.

This package exports asset metadata only; application APIs live in `github.com/egoist/quickgui/go/native` and `github.com/egoist/quickgui/go/ui`.

Build from a source checkout with `bun run build:native`. macOS arm64 and x64 libraries are staged in `lib/darwin-arm64` and `lib/darwin-x64`. Linux uses `libquickgui_host.so`; Windows uses `quickgui_host.dll`. App code reuses these artifacts without rebuilding Rust. The SDK rejects incompatible protocol versions at startup.
