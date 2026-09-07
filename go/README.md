# QuickGUI for Go

QuickGUI applications are ordinary Go programs. `purego` loads the prebuilt Rust shared library in the same process, with `CGO_ENABLED=0`. The Rust core owns layout, rendering, accessibility, native controls, windows, and platform services. Go owns application state and the fine-grained reactive graph. The TypeScript CLI handles development and packaging.

## Components

Every component is a `func()` declaration, including window roots, routes, conditional branches, and children blocks. Pass it directly as `WindowOptions.Component`; QuickGUI collects its nodes and owns their lifetime. Components can declare zero, one, or several roots without a return or fragment wrapper.

```go
package main

import (
	"fmt"
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:     "Counter",
				Width:     760,
				Height:    520,
				Component: Counter,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func Counter() {
	count, setCount := ui.CreateSignal(0)
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Gap(20),
		ui.BackgroundColor("#090d16"),
		ui.Color("#e2e8f0"),
		func() {
			ui.Text(ui.FontSize(28), ui.FontWeight(700), "Fine-grained native UI")

			ui.Text(func() string {
				return fmt.Sprintf("Count: %d", count())
			})

			ui.Button(
				ui.Padding(12),
				ui.BorderRadius(8),
				ui.BackgroundColor("#2563eb"),
				ui.Hover(ui.BackgroundColor("#3b82f6")),
				ui.OnClick(func() { setCount(count() + 1) }),
				"Increment",
			)
		},
	)
}
```

Components and `func() { ... }` children blocks run once when mounted. UI calls inside a block declare children in order; nested blocks keep their own parent. Ordinary `if` and `for` statements are useful for static construction. Reading a signal inside an accessor subscribes that binding; setting it changes the affected native properties or text nodes. Event handlers batch writes automatically. Use `ui.Batch` to group writes outside an event. A getter called while constructing a static string captures its current value: pass `func() string { … }` as the text child for changing text.

Primitives accept options and children directly: `ui.View(ui.Padding(20), ui.BackgroundColor("#ccc"), "Hello")`. Use `ui.OnClick(func() { ... })` for ordinary clicks, or `ui.OnClickEvent` when you need the native event. Reuse a style record with `ui.WithStyle(style)`; compound controls also accept `ui.Style` records. Numeric lengths are logical pixels; strings support percentages and CSS declarations such as grid tracks. `nil` means an omitted numeric field, so `Width: 0` remains meaningful. `ui.Hover`, `ui.Active`, and `ui.Focus` accept nested style options that the Rust core applies during interaction. `ui.DisabledStyle` and `ui.SelectedStyle` distinguish interaction styles from the node state options.

## Conditional options

`ui.When` tracks a condition and applies one or more options while it is true. Later options override earlier ones. Turning a condition off restores an earlier value, or clears the property when there is no base value. Children stay mounted. Conditional event handlers and value bindings are released when their condition becomes false.

```go
selected, setSelected := ui.CreateSignal(false)
ui.Button(
	ui.BackgroundColor("#ccc"),
	ui.When(selected,
		ui.BackgroundColor("#2563eb"),
		ui.Color("white"),
	),
	ui.OnClick(func() { setSelected(!selected()) }),
	"Toggle selection",
)
```

For a computed condition, use `ui.When(func() bool { return count() >= 5 }, ...)`. Keep ref callbacks outside conditional options; refs run after mounting.

Run `quickgui fmt` to apply QuickGUI's formatting rule. Long UI calls (over 100 columns), multiline calls, and calls with callback arguments use one argument per line and a trailing comma. Multiline props and style records use one field per line. Short calls stay compact; `gofmt` then handles ordinary Go spacing, indentation, and alignment. Comments and string contents are preserved, and generated files are left to their generator. Running `gofmt` afterward preserves the layout.

Use `quickgui fmt --check` in CI or `quickgui fmt --project path/to/app` for another project. The formatter uses the SDK version in the project's `go.mod`. In this repository, `bun run fmt:go` formats the SDK and examples; `bun run test:go` checks the same rule. The Go command can also read source on stdin for editor integration: `go run github.com/egoist/quickgui/go/cmd/quickguifmt`.

## Conditional content and lists

```go
ui.Show(func() bool { return count() >= 5 }, func() {
	ui.Text("Five or more clicks")
})
```

`Show` accepts component functions for its content and optional fallback. It creates children lazily and disposes them when hidden. `For` reuses unchanged rows by a comparable key; `KeyedFor` gives each retained row an item accessor so changing its data preserves local state. Keys must be unique. Removed rows release their effects and native listeners. Window closure disposes the entire component tree and outstanding component background work.

`ui.Dynamic(func() ui.Component { ... })` selects a component reactively. Only the selector reruns; bindings inside the selected component keep updating its retained nodes. Return `nil` from the selector to render nothing. List callbacks also declare their children directly:

```go
ui.For(items, func(item Item, index func() int) {
	ui.Text(item.Name)
	ui.Text(func() string { return fmt.Sprint(index()) })
}, func(item Item) any { return item.ID }, nil)
```

Use `ui.Ref(func(node *native.Node) { ... })` when a component needs a node handle, such as a popover anchor. Framework constructors still expose node handles for low-level tree work; component callbacks always use `func()`.

## Background work

Components, signals, event callbacks, and CPU-only native router calls run on one Go goroutine pinned to an OS thread. `native.Run` must be called once from `main`; it reserves the process main thread for AppKit/Winit. Do not read or write signals from worker goroutines.

`native.Dispatch(func() { … })` queues a background result onto the UI goroutine. `ui.Async(work, done)` runs a context-aware worker and dispatches its completion, canceling it when its component is disposed. `ui.OnCleanup` releases component resources. Native dialogs and services use asynchronous callbacks: none synchronously waits for the native main thread.

Go and Rust exchange bounded binary mutation batches and copied event data through an in-process C ABI. No application host process or frontend IPC is involved. Clean windows sleep; property deduplication and batched effects avoid redundant native mutations. App-specific background jobs can still consume CPU.

## Packages and controls

- `native`: application/window lifecycle, nodes, events, asynchronous dialogs and services, menus, OS filesystem watchers, and the Rust router.
- `ui`: functional options, primitives, `Show`, `For`, `KeyedFor`, compound controls, routing, and macOS SwiftUI hosting.
- `reactive`: signals, memos, batching, effects, owners, and contexts.
- `host`: the purego C ABI adapter and a replaceable host interface for tests.

Compound controls accept the same children blocks after their typed props, so descendants inherit their root context: `ui.Tabs.Root(props, func() { ... })`. Pass strings or string accessors directly as children to text and buttons. Primitives accept style and event options directly, without a props wrapper. `ui.Child(node)` inserts a previously constructed detached node in a block. The `Props.Children` form remains available for programmatic composition. Families include `Checkbox`, `Switch`, `Tabs`, `Dialog`, `Popover`, `SystemPopover`, `Slider`, `Select`, `Combobox`, `Menu`, `Table`, `Tree`, and `Toast`. Their native behavior remains in Rust. `ui.SwiftUI` provides the macOS SwiftUI control gallery and reverse-hosted QuickGUI views.

See [counter](../examples/counter/main.go), [components](../examples/components/main.go), [routing](../examples/routing/main.go), [SwiftUI](../examples/swift-ui/main.go), and the full [Quick Git](../examples/quick-git/main.go) application.

## Build

Go 1.23 or newer and Bun are required for development. On macOS, packaging also uses Xcode Command Line Tools. Published native assets currently target macOS arm64 and x64; Linux and Windows need a matching host shared library and native runtime validation.

From a source checkout, build the Rust library once:

```console
bun install
bun run build:native
bun packages/cli/src/cli.ts dev --project examples/counter
```

Application edits only rebuild Go. To build directly without the CLI:

```console
CGO_ENABLED=0 go -C examples/counter build -o /tmp/quickgui-counter .
QUICKGUI_LIBRARY="$PWD/target/release/libquickgui_host.dylib" /tmp/quickgui-counter
```

A Go `replace` directive points each repository example at `../../go`. External applications depend on `github.com/egoist/quickgui/go`; releases use the submodule tag `go/v<version>`. The CLI bundles the matching library under macOS `Contents/Frameworks`, or beside the executable on Linux/Windows. `QUICKGUI_LIBRARY` can select an explicit library for development. Rust and Go protocol versions are checked at load time.

Run `bun run test:go` to check formatting, generated Rust protocol constants, the SDK, and every Go example with CGO disabled. Run `go -C go generate ./protocol` after changing Rust wire constants.
