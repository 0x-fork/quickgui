# {{README_TITLE}}

A native QuickGUI application written in Go.

```console
bun install
go mod tidy
bun run dev
```

`bun run dev` compiles `main.go` with CGO disabled, packages and launches the app, and rebuilds on source changes. `bun run build` produces a distributable application with its prebuilt Rust host shared library. Components are passed directly as `Component: Counter`; accessors create fine-grained bindings.

Use Go 1.23+, Bun, and Xcode Command Line Tools on macOS. Application builds do not invoke Rust or a C compiler. The application uses purego to call Rust in process.
