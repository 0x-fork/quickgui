package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:     {{APP_NAME}},
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
		func() {
			ui.Text("Fine-grained native UI", ui.Style{FontSize: 28, FontWeight: 700})

			ui.Text("Count: ", count)

			ui.Button(
				"Increment",
				ui.Style{
					Padding:         12,
					BorderRadius:    8,
					BackgroundColor: "#2563eb",
					Hover:           &ui.Style{BackgroundColor: "#3b82f6"},
				},
				ui.OnClick(func() { setCount(count() + 1) }),
			)
		},
		ui.Style{
			Display:         "flex",
			FlexDirection:   "column",
			Width:           "100%",
			Height:          "100%",
			AlignItems:      "center",
			JustifyContent:  "center",
			Gap:             20,
			BackgroundColor: "#090d16",
			Color:           "#e2e8f0",
		},
	)
}
