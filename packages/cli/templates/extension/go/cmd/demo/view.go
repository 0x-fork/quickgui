package main

import (
	extension "{{GO_MODULE}}"

	"github.com/egoist/quickgui/go/ui"
)

func App() {
	message, setMessage := ui.CreateSignal("Hello from a pure Go extension")
	ui.View(
		func() {
			ui.Text("{{NAME}}").FontSize(24).FontWeight(700)
			extension.Notice(message, ui.Style().FontSize(16))
			ui.Button("Update message").
				OnClick(func() { setMessage("Only the message text changed") }).
				Padding(12).BorderRadius(8).BackgroundColor("#2563eb").TextColor("white").
				Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#3b82f6") })
		},
	).Display("flex").FlexDirection("column").JustifyContent("center").Height("100%").Padding(24).Gap(16)
}
