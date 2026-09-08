package main

import (
	extension "{{GO_MODULE}}"

	"github.com/egoist/quickgui/go/ui"
)

func App() {
	message, setMessage := ui.CreateSignal("Ready to call the {{TYPE}} extension")
	ui.View(
		func() {
			ui.Text("{{NAME}}", ui.FontSize(24), ui.FontWeight(700))
			ui.Text(message, ui.FontSize(16))
			ui.Button(
				"Call extension",
				ui.OnClick(func() {
					extension.Echo("Hello from {{TYPE}} through purego", func(reply string, err error) {
						if err != nil {
							setMessage(err.Error())
							return
						}
						setMessage(reply)
					})
				}),
				ui.Padding(12),
				ui.BorderRadius(8),
				ui.BackgroundColor("#2563eb"),
				ui.Color("white"),
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
