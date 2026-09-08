# {{NAME}}

A standalone MoonBit service, usable from Go and MoonBit QuickGUI applications.
It includes a Go wrapper and demo, a service ABI adapter, and Bun build scripts.
The service runs in the application's process without core integration.

## Build the provider

Install Bun and MoonBit (tested with v0.10.11), plus a native C compiler. On
Windows, use an MSVC developer shell with `cl` or `clang-cl` and the Windows SDK.
`MOON`, `MOON_HOME`, and `CC` can select your toolchain.

```sh
bun run build
moon -C native test --target native --release
```

Edit `native/extension.mbt` to implement your operations. `handle` receives the
request ID, operation name, and raw UTF-8 JSON bytes. Return `Ok(bytes)` containing
JSON, or `Err(message)` for a recoverable failure. The sample echoes the original
JSON without decoding and re-encoding it, preserving nulls and large numbers.

The build uses MoonBit's C backend and a private copy of its runtime. Only
`quickgui_extension_v1` is exported from the shared library, so multiple providers
and a MoonBit application can coexist without sharing allocation/layout tables.
`native/bridge.mbt` and `native/bridge.c` handle the ABI; no Go or QuickGUI SDK is
needed to compile the provider. The nested `native/moon.work` keeps provider
compilation independent of a containing application's Moon workspace.

The generated adapter supports bounded synchronous operations. QuickGUI invokes
MoonBit on its UI worker; keep `handle` and `shutdown` short. The host can request
shutdown on its main thread. A nonblocking gate serializes access to the private
runtime and defers shutdown until an active handler returns; concurrent or
reentrant requests receive a busy error instead of waiting. The request context
is borrowed for that call only. The adapter copies input into MoonBit-owned bytes,
emits a reply, and releases the native sink exactly once, including errors. Both
input and output are limited to 64 KiB.

For slow native work or persistent sessions, extend the C adapter to copy inputs,
own and retain the sink until completion, then emit/release through the native
ABI. Background workers must not call MoonBit functions or touch MoonBit objects.
Do not wait for workers or the native main thread during invoke or shutdown.
Return `Err` for expected errors; MoonBit `abort` terminates the process.

## Run the Go demo

Install Go 1.23 or newer in addition to the provider tools:

```sh
bun install
go mod tidy
bun run dev
```

The demo calls the MoonBit provider through purego. `bun run dev` builds the
provider once; Go edits reuse it. Restart after native changes. `bun run fmt`
formats the Go wrapper and demo; `moon -C native fmt` formats the MoonBit code.
`--module` configures the Go wrapper's module; edit `native/moon.mod` to change
the provider's MoonBit module name.

## Use from MoonBit

Add this extension's directory to the application's `native.extensions` array
in `quickgui.toml`, for example:

```toml
[native]
extensions = ["vendor/{{NAME}}"]
```

Paths are relative to the application project. The CLI reads
`quickgui.extension.json` inside each directory. The same manifest/library
works with both frontends; the Go wrapper is optional.

```moonbit
fn echo(message : String, done : (Result[Json, String]) -> Unit) -> Unit {
  @native.invoke("extension/{{NAME}}/echo", Json::string(message), done)
}
```

Import `egoist/quickgui/native` as `@native`, and call this inside a live
component, event, or application owner. The CLI packages and registers the
selected provider before startup. For local development, set
`QUICKGUI_EXTENSION_DIR` to the absolute `artifacts/lib/<platform>-<arch>` path.

## Distribute

`quickgui.extension.json` controls the provider's name, exact release, and npm
artifact package. The build generates matching C constants and updates
`artifacts/package.json`; the Go wrapper embeds the manifest. Build through
`bun run build` on each supported target machine, then collect
`artifacts/lib/<platform>-<arch>/` into one package. This template builds for the
current host, not a cross-compilation target.

Publish `artifacts/` to npm with its generated MoonBit license and notice. Go
consumers import `{{GO_MODULE}}`; MoonBit consumers select the directory and use a
MoonBit wrapper. Both resolve `{{NPM_PACKAGE}}` at the exact manifest version.
The CLI bundles the library beside the app; end users need no compiler.

The packages are not published yet. In this checkout, `--no-install` lets you
build the provider immediately. To run the Go demo, point its SDK to the local
`go` directory using `replace` in `go.mod`, set `@quickgui/cli` to a local `file:`
dependency, then install. A MoonBit consumer can use `moon.work` with the local
`moonbit` SDK directory.

See [extension authoring](https://github.com/egoist/quickgui/blob/main/website/src/content/docs/moonbit/en/extensions.mdx)
for manifest selection, session events, resource bundles, and distribution.
