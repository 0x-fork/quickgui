# {{NAME}}

A pure Go QuickGUI component library. The `cmd/demo` application exercises the
reactive `Notice` component.

## Develop

Install Go 1.23 or newer and Bun 1.3 or newer, then:

```sh
bun install
go mod tidy
bun run dev
```

`bun run build` checks the library and demo with `CGO_ENABLED=0`. `bun run fmt`
formats the Go UI declarations. The QuickGUI CLI builds and reloads the demo on
Go edits; only the application's Go code needs recompilation.

## Use in another application

Publish this repository as the Go module `{{GO_MODULE}}`, then:

```sh
go get {{GO_MODULE}}@v0.1.0
```

```go
import (
	"github.com/egoist/quickgui/go/ui"
	extension "{{GO_MODULE}}"
)

func App() *ui.Element {
	message, _ := ui.CreateSignal("Hello")
	return extension.Notice(message, ui.Style().FontSize(18))
}
```

The package uses normal QuickGUI signals and components. It needs no extension
manifest, separate shared library, or npm artifact package. The root npm package
is private development tooling. Replace the default `example.com` module path in
`go.mod` and `cmd/demo/view.go` with your repository before publishing.

See [extension authoring](https://github.com/egoist/quickgui/blob/main/website/src/content/docs/en/extensions.mdx) for distribution
and native service alternatives.
