# {{README_TITLE}}

A native QuickGUI application written in Go.

```console
bun install
go mod tidy
bun run dev
```

`bun run dev` compiles `main.go` with CGO disabled, packages and launches the app, and rebuilds on source changes. `bun run build` produces a distributable application with its prebuilt Rust host shared library. Components use `func()` and declare their children implicitly. Pass a root directly as `Component: Counter`. Plain value props and expressions such as `ui.Text(count())` stay reactive through the compiler. Native constructors return builders for fluent styling; node-returning component factories are unsupported.

Use Go 1.23+, Bun, and Xcode Command Line Tools on macOS. Application builds do not invoke Rust or a C compiler. The application uses purego to call Rust in process.

Configuration can use `quickgui.toml` or `quickgui.config.ts`. The CLI looks for TOML first; `--config path/to/file` selects a file explicitly. Both formats use the same settings.
