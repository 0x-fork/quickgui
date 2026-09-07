# Native extensions

Go applications load one QuickGUI core shared library through purego. Optional backends ship as separate libraries with the same release version. The terminal extension separates rendering from its backend: the core keeps its retained terminal view while `quickgui-terminal` owns Ghostty, the PTY, and terminal workers. The updater extension uses Sparkle on macOS and a compatible signed-appcast backend on Windows/Linux. Its network and installer code is absent from the default core.

## Application usage

```go
import (
    "github.com/egoist/quickgui/go/terminal"
    "github.com/egoist/quickgui/go/ui"
)

func Console() {
    terminal.View(terminal.Props{
        Program: "/bin/zsh",
        Args:    []string{"-l"},
        Props: ui.Props{
            Style: ui.Style{Width: "100%", Height: 320},
        },
    })
}
```

`terminal.Palette`, `terminal.StatusDetails`, and `terminal.StatusFromEvent` replace their previous `ui.Terminal…` equivalents. Normal styles, children, reactive properties, event ownership, and PTY lifecycle remain supported. Do not import the terminal package from `ui`: an ordinary app must stay independent of it.

## Resolution and packaging

Each optional Go package contains a `quickgui.extension.json` manifest and calls `host.RequireExtension` during package initialization. Initialization only records the requirement. Before running the application, the Go loader opens the selected extension image, obtains `quickgui_extension_v1`, and registers its descriptor with the core.

The CLI runs `go list -deps` with the same entry, target, environment, and build tags as `go build`. A transitive import opts in too; excluded target/tag files do not. The resolver deduplicates requirements, rejects conflicting versions, and supports at most 32 extensions. No feature list is duplicated in application config.

Artifacts are resolved from an explicit `QUICKGUI_EXTENSION_DIR`, an installed matching npm package, source-checkout staging, or an exact-version npm download. Downloaded tarballs require SHA-512 integrity; only declared target libraries and resources are extracted, with bounded input and decompressed sizes. Cached libraries have checked digests and a 512 MiB eviction budget. The extension loader only loads local files. An updater session starts update networking only after the application explicitly initializes it.

Manifests may declare bounded per-platform `resources`: installer helpers or `.qgr` resource bundles. Framework bundles validate every path and byte budget before writing files, then create confined symlinks. macOS preserves Sparkle framework links and signs its nested code with the app identity.

macOS bundles place core and extension images in `Contents/Frameworks`. Linux and Windows payloads place them beside the executable, including AppDir, Debian, and NSIS payloads. Removing a Go import removes the extension from the next fresh bundle. End users need only the packaged application.

## Updater service

Import `github.com/egoist/quickgui/go/updater` and call `updater.Start` once after application readiness. The core routes service replies and events through the existing Go event queue. `ServiceApi` copies bounded inputs and queues native work; a start sink persists for its session and releases exactly once after its last worker. Command replies do not tear down event subscriptions. Sparkle objects are confined to the macOS main queue. Portable service commands serialize, downloads run off-thread, and closing a session cancels outstanding network work without joining a worker on the UI thread.

The signed installer handoff uses a helper only after payload verification, allowing normal quit hooks to run before loaded application files are replaced. See [automatic updates](../updater.md) for configuration, publication, and platform limits.

## Native contract

`src/extension_api.rs` defines ABI version 1 using C layouts, fixed-width fields, borrowed spans, function pointers, and opaque session handles. The core checks the descriptor header before reading its layout, then validates the requested extension, function table size, ABI version, and exact release version. Rust-owned allocations, traits, objects, and allocator ownership never cross libraries.

A terminal creation call owns its wake callback context even on failure. The final worker releases that context exactly once. Destroying a session triggers the existing PTY shutdown path. Images remain loaded for process lifetime because worker callbacks retain function pointers; session resources do not remain alive just because the image does.

Commands are bounded byte spans or JSON metadata. A changed frame crosses in one typed callback, carrying text, style runs, graphics, cursor, selection, scroll, edge colors, and process metadata. The core copies a complete frame once per revision into an immutable snapshot. Unchanged revisions return without invoking the callback or copying screen data. Invalid frames become a terminal failure instead of silently keeping stale content.

The extension compiles the existing backend sources without depending on the core crate, WGPU, Taffy, or the application runtime. The core `terminal-extension` feature compiles the retained adapter without Ghostty or PTY dependencies. Existing Rust users can still opt into the `terminal` feature to link the backend statically. Keep both routes on the shared terminal algorithms.

## Development and releases

```console
bun packages/native/build.ts
bun packages/native/build.ts --extension terminal
bun packages/native/build.ts --extension updater
bun scripts/check-extensions.ts
```

The native build commands rebuild framework artifacts after native changes. Go application edits only run Go compilation and reuse them. Cross-architecture release builds pass `--target aarch64-apple-darwin` or `--target x86_64-apple-darwin` for each artifact.

Version synchronization covers the backend crate, npm package, and Go manifest. The release workflow builds and validates the two core images, two terminal images, and two updater images with Sparkle resources, packages four independent npm archives, then publishes them in dependency order. `@quickgui/cli` depends on `@quickgui/native`; `@quickgui/native-terminal` and `@quickgui/native-updater` remain optional.

For future extensions, add a Go manifest/registration, a separately built backend, and a versioned typed provider contract consumed by a core adapter. Extend the core registry and release build list; do not generate bundles for combinations of extensions or link another renderer into a backend.
