package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/ui"
)

func main() { run("Native styling", 960, 720, Styling) }

func Styling() {
	warm, setWarm := ui.CreateSignal(false)
	var cards []any
	for index, label := range []string{"Flex layout", "Retained hover styles", "Rounded surfaces", "Reactive properties"} {
		node := ui.View(
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.Height(140),
			ui.Gap(16),
			ui.Padding(24),
			ui.BorderRadius(12),
			ui.Hover(ui.BackgroundColor("#1e40af")),
			func() {
				ui.Text(ui.FontSize(22), label)
				ui.Text("Layout and paint remain in the Rust core.")
			},
		)
		node.Bind(func() {
			color := "#312e81"
			if warm() {
				color = "#713f12"
			} else if index%2 == 0 {
				color = "#1e3a5f"
			}
			native.SetColor(node, protocol.BackgroundColor, native.ParseColor(color))
		})
		cards = append(cards, node)
	}
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(24),
		func() {
			button("Switch palette", func() { setWarm(!warm()) })
			ui.View(ui.Display("grid"), ui.GridTemplateColumns("1fr 1fr"), ui.Gap(20), cards)
			ui.Text(
				ui.Color("#94a3b8"),
				"Numbers use logical pixels. Percentages and grid tracks use strings.",
			)
		},
	)
}
