package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func run(title string, width, height float64, component func()) {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:         title,
				Width:         width,
				Height:        height,
				MinimumWidth:  500,
				MinimumHeight: 380,
				Background:    "#0b1020",
				TitleBarStyle: "hiddenInset",
				Component: func() {
					ui.View(
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Width("100%"),
						ui.Height("100%"),
						ui.Gap(22),
						ui.Padding(36),
						ui.PaddingTop(52),
						ui.BackgroundColor("#0b1020"),
						ui.Color("#e2e8f0"),
						func() {
							ui.Text(ui.FontSize(22), ui.FontWeight(700), title)
							component()
						},
					)
				},
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

var buttonStyle = ui.Style{
	Display:         "flex",
	Padding:         12,
	BorderRadius:    8,
	BackgroundColor: "#2563eb",
	Color:           "white",
	Hover:           &ui.Style{BackgroundColor: "#3b82f6"},
	UserSelect:      "none",
	AppRegion:       "no-drag",
}

func button(label string, click func(), options ...any) {
	args := []any{ui.WithStyle(buttonStyle), ui.OnClick(func() { click() }), label}
	args = append(args, options...)

	ui.Button(args...)

}

func card(children ...any) {
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(16),
		ui.Padding(24),
		ui.BorderRadius(12),
		ui.BackgroundColor("#151e30"),
		children,
	)
}
