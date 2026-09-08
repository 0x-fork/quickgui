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
			ui.Text("Fine-grained native UI", ui.FontSize(28), ui.FontWeight(700))

			ui.Text("Count: ", count)

			ui.Button(
				"Increment",
				ui.Padding(12),
				ui.BorderRadius(8),
				ui.BackgroundColor("#2563eb"),
				ui.Hover(ui.BackgroundColor("#3b82f6")),
				ui.OnClick(func() { setCount(count() + 1) }),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Gap(20),
		ui.BackgroundColor("#090d16"),
		ui.Color("#e2e8f0"),
	)
}
