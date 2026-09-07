# Go dependencies and native libraries

Applications use ordinary Go modules. Prefer pure Go dependencies for fast builds with `CGO_ENABLED=0`. There is no Zig module generator or TypeScript binding layer.

The framework itself loads its prebuilt Rust shared library through purego. Public native capabilities belong in the Rust core and should be exposed through the existing C ABI, then adopted by the Go SDK. Copy callback payloads before returning to Rust and keep all UI state on the application goroutine. Native main-thread operations must be asynchronous.

For an external native library, use its documented C ABI with purego and package the matching shared library as an application resource. This preserves CGO-free application builds, but that library is still a native build artifact with its own platform dependencies.

See the [Go guide](go.md), [CLI guide](cli.md), and [FFI implementation](../go/internal/ffi/ffi.go).
