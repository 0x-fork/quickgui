# Independent native extension

This example loads `acme-echo` version `1.0.0` through purego using the stock QuickGUI service ABI. The C extension includes only the standalone [SDK header](../../include/quickgui_extension.h). It does not link the Rust core or require a extension-specific core/CLI registry entry.

From the repository root, with Go, Bun, and Clang on PATH:

```console
bun packages/native/build.ts
bun examples/native-extension/build-extension.ts dev
```

The first command stages the reusable QuickGUI core. The second builds the extension and runs the application. Clicking **Call extension** sends a JSON string to the native C library and displays its reply. Normal Go edits reuse the native images. Use `bun examples/native-extension/build-extension.ts build` to package the application, or run the script without arguments to build only the extension. `CC` can select a different C compiler executable.

- `backend/echo.c` exports `quickgui_extension_v1` and a two-function service table. Every request replies and releases its sink exactly once.
- `echo/echo.go` declares the exact extension version and wraps `native.InvokeExtension` with a typed Go function. The Go application builds with CGO disabled; the C library is built separately.
- `echo/quickgui.extension.json` selects the artifact package `@acme/extension-echo`, independent of QuickGUI's version and npm scope.
- `backend/package.json` describes the artifact package. Its `lib/<target>/` directories contain `.dylib`, `.so`, or `.dll` images, built on their respective hosts. `@acme` is an example scope; the package is private and is not published.

To author your own extension, vendor the header, use your own Go module and npm package, and keep the manifest, descriptor, requirement, and artifact version in agreement. Install the matching artifact package in the consumer or use `QUICKGUI_EXTENSION_DIR` while developing. The CLI discovers the extension from the actual Go dependency graph and packages only imported extensions. End users receive the libraries with the application.

```console
bun scripts/check-extensions.ts
```

The headless check builds this extension, verifies real JSON replies/errors through purego, then copies its Go package and npm artifacts into an isolated consumer. After packaging, it removes those build inputs and runs the consumer from its bundled libraries. It also requires the staged built-in terminal and updater artifacts to check their existing combinations.

See the [extension authoring guide](../../website/src/content/docs/en/extensions.mdx) for session events, callback ownership, resource packaging, and ABI compatibility.
