# styling

Ten demos cover text alignment and decoration, RTL layout, gradients, corners and
outlines, raster and subtree filters, transforms and blending, interaction states,
sticky headers, and mandatory scroll snapping. Hover a row to reveal its actions;
named group hints follow the whole list. The palette switch changes retained panel
styles without remounting the gallery.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/styling
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/styling test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
