# {{README_TITLE}}

A native QuickGUI application written in Go.

```console
bun install
go mod tidy
bun run dev
```

`bun run dev` compiles `main.go` with CGO disabled, packages and launches the app, and rebuilds on source changes. `bun run build` produces a distributable application with its prebuilt Rust host shared library. Components return `*ui.Element` or `*native.Node` and are passed directly as `Component: Counter`. Compose children with `ui.View(ui.Text(count()), ...)`; the compiler preserves reactive reads without a children callback.

Use Go 1.23+, Bun, and Xcode Command Line Tools on macOS. Application builds do not invoke Rust or a C compiler. The application uses purego to call Rust in process.

Configuration can use `quickgui.toml` or `quickgui.config.ts`. The CLI looks for TOML first; `--config path/to/file` selects a file explicitly. Both formats use the same settings.
