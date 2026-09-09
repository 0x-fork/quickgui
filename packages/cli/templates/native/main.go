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

func Counter() *ui.Element {
	count, setCount := ui.CreateSignal(0)
	return ui.View(
		ui.Text("Fine-grained native UI").FontSize(28).FontWeight(700),
		ui.Text("Count: ", count()),
		ui.Button("Increment").
			OnClick(func() { setCount(count() + 1) }).
			Padding(12).
			RoundedLg().
			Bg("#2563eb").
			Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#3b82f6") }),
	).FlexCol().
		SizeFull().
		ItemsCenter().
		JustifyCenter().
		Gap(20).
		Bg("#090d16").
		TextColor("#e2e8f0")
}
