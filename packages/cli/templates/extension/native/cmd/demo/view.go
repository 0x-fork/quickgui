package main

import (
	extension "{{GO_MODULE}}"

	"github.com/egoist/quickgui/go/ui"
)

func App() {
	message, setMessage := ui.CreateSignal("Ready to call the {{TYPE}} extension")
	ui.View(
		func() {
			ui.Text("{{NAME}}", ui.Style{FontSize: 24, FontWeight: 700})
			ui.Text(message, ui.Style{FontSize: 16})
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
				ui.Style{
					Padding:         12,
					BorderRadius:    8,
					BackgroundColor: "#2563eb",
					Color:           "white",
					Hover:           &ui.Style{BackgroundColor: "#3b82f6"},
				},
			)
		},
		ui.Style{
			Display:        "flex",
			FlexDirection:  "column",
			JustifyContent: "center",
			Height:         "100%",
			Padding:        24,
			Gap:            16,
		},
	)
}
