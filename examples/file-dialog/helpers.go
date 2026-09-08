package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func run(options native.WindowOptions, component func()) {
	options.Background = "#0b0e14"
	options.TitleBarStyle = "hiddenInset"
	options.TrafficLightPosition = &native.Point{X: 16, Y: 14}
	options.Component = func() {
		ui.View(
			func() {
				ui.View(
					func() { ui.Text(options.Title, ui.FontSize(14), ui.FontWeight(600)) },
					ui.Display("flex"),
					ui.Height(52),
					ui.FlexShrink(0),
					ui.AlignItems("center"),
					ui.JustifyContent("center"),
					ui.AppRegion("drag"),
					ui.BorderColor("#1f2530"),
					ui.BorderBottomWidth(1),
				)
				ui.View(
					component,
					ui.Display("flex"),
					ui.Flex(1),
					ui.MinHeight(0),
					ui.Padding(36),
					ui.AlignItems("center"),
					ui.JustifyContent("center"),
					ui.OverflowY("auto"),
				)
			},
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.Width("100%"),
			ui.Height("100%"),
			ui.BackgroundColor("#0b0e14"),
			ui.Color("#f4f7fb"),
		)
	}
	if err := native.Run(func() {
		open := func() { native.NewWindow(options) }
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

var panelStyle = ui.Styles(
	ui.Display("flex"),
	ui.FlexDirection("column"),
	ui.Width("100%"),
	ui.MaxWidth(480),
	ui.FlexShrink(0),
	ui.Gap(20),
	ui.Padding(28),
	ui.BackgroundColor("#151922"),
	ui.BorderColor("#2c3442"),
	ui.BorderWidth(1),
	ui.BorderRadius(16),
)

var buttonStyle = ui.Styles(
	ui.Display("flex"),
	ui.Height(42),
	ui.AlignItems("center"),
	ui.JustifyContent("center"),
	ui.PaddingLeft(16),
	ui.PaddingRight(16),
	ui.BackgroundColor("#262c38"),
	ui.Color("#f4f7fb"),
	ui.BorderColor("#3a4353"),
	ui.BorderWidth(1),
	ui.BorderRadius(9),
	ui.Cursor("default"),
	ui.AppRegion("no-drag"),
	ui.UserSelect("none"),
	ui.Hover(ui.BackgroundColor("#30394a")),
)

func button(label string, disabled func() bool, click func()) {
	ui.Button(label, buttonStyle, ui.Disabled(disabled), ui.OnClick(click))
}

func dialogStatus(status func() string, pending func() bool) {
	ui.View(
		func() {
			ui.Text(
				status,
				ui.FontSize(13),
				ui.LineHeight(19),
				ui.TextAlign("center"),
				ui.UserSelect("text"),
				ui.Color(func() string {
					if pending() {
						return "#c7d2fe"
					}
					return "#aeb9c9"
				}),
			)
		},
		ui.Display("flex"),
		ui.MinHeight(64),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Padding(14),
		ui.BackgroundColor("#0f131a"),
		ui.BorderColor("#252c38"),
		ui.BorderWidth(1),
		ui.BorderRadius(9),
	)
}
