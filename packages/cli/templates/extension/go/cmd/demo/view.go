package main

import (
	extension "{{GO_MODULE}}"

	"github.com/egoist/quickgui/go/ui"
)

func App() {
	message, setMessage := ui.CreateSignal("Hello from a pure Go extension")
	ui.View(
		func() {
			ui.Text("{{NAME}}", ui.FontSize(24), ui.FontWeight(700))
			extension.Notice(message, ui.FontSize(16))
			ui.Button(
				"Update message",
				ui.OnClick(func() { setMessage("Only the message text changed") }),
				ui.Padding(12),
				ui.BorderRadius(8),
				ui.BackgroundColor("#2563eb"),
				ui.TextColor("white"),
				ui.Hover(ui.BackgroundColor("#3b82f6")),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.JustifyContent("center"),
		ui.Height("100%"),
		ui.Padding(24),
		ui.Gap(16),
	)
}
