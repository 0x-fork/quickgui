package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		openMainWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openMainWindow()
			}
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func openMainWindow() {
	native.NewWindow(native.WindowOptions{
		Title:                "QuickGUI Counter",
		Width:                760,
		Height:               520,
		MinimumWidth:         520,
		MinimumHeight:        360,
		Background:           "#090d16",
		TitleBarStyle:        "hiddenInset",
		TrafficLightPosition: &native.Point{X: 16, Y: 13},
		Component:            Counter,
	})
}

func openDetailsWindow() {
	native.NewWindow(native.WindowOptions{
		Title:         "Dynamic QuickGUI window",
		Width:         420,
		Height:        260,
		MinimumWidth:  320,
		MinimumHeight: 200,
		Background:    "#111827",
		Component: func() {
			window := native.CurrentWindow()
			ui.View(
				func() {
					ui.Text(
						"Created while the app is running",
						ui.FontSize(22),
						ui.FontWeight(700),
					)
					ui.Text(
						"This window has its own retained tree and native lifecycle.",
						ui.Color("#94a3b8"),
						ui.LineHeight(21),
					)
					ui.Button(
						"Close window",
						ui.OnClick(func() { window.Close() }),
						ui.Display("flex"),
						ui.Height(40),
						ui.AlignItems("center"),
						ui.JustifyContent("center"),
						ui.BackgroundColor("#334155"),
						ui.BorderRadius(9),
						ui.Cursor("default"),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width("100%"),
				ui.Height("100%"),
				ui.JustifyContent("center"),
				ui.Gap(16),
				ui.Padding(28),
				ui.BackgroundColor("#111827"),
				ui.Color("#e2e8f0"),
			)
		},
	})
}

func Counter() {
	count, setCount := ui.CreateSignal(100)
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text("QuickGUI · Go", ui.FontWeight(600))
				},
				ui.Display("flex"),
				ui.Height(52),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.JustifyContent("center"),
				ui.AppRegion("drag"),
				ui.BorderColor("#1e293b"),
				ui.BorderWidth(1),
			)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.Text(
								"Fine-grained native UI",
								ui.FontSize(28),
								ui.LineHeight(36),
								ui.FontWeight(700),
							)
							ui.Text(
								"Signals update only the changed text node. The application is ordinary Go, "+
									"and QuickGUI retains layout, sleeps while clean, and redraws once per mutation batch.",
								ui.Color("#94a3b8"),
								ui.FontSize(14),
								ui.LineHeight(21),
							)
							ui.Text(
								"Count: ",
								count,
								ui.Color("#bfdbfe"),
								ui.When(
									func() bool { return count() >= 5 },
									ui.Color("#fbbf24"),
								),
								ui.FontSize(20),
								ui.FontWeight(600),
							)
							ui.Show(
								func() bool { return count() >= 5 },
								func() {
									ui.Text(
										"Five or more clicks: the row above was created on demand.",
										ui.Color("#fbbf24"),
										ui.FontSize(14),
									)
								},
							)
							ui.Button(
								"Increment",
								ui.OnClick(func() { setCount(count() + 1) }),
								ui.Display("flex"),
								ui.Height(44),
								ui.AlignItems("center"),
								ui.JustifyContent("center"),
								ui.BackgroundColor("#2563eb"),
								ui.Color("white"),
								ui.BorderRadius(9),
								ui.Cursor("default"),
								ui.AppRegion("no-drag"),
								ui.UserSelect("none"),
								ui.Hover(ui.BackgroundColor("#3b82f6")),
							)
							ui.Button(
								"Open window",
								ui.OnClick(func() { openDetailsWindow() }),
								ui.Display("flex"),
								ui.Height(44),
								ui.AlignItems("center"),
								ui.JustifyContent("center"),
								ui.BackgroundColor("#334155"),
								ui.Color("white"),
								ui.BorderRadius(9),
								ui.Cursor("default"),
								ui.AppRegion("no-drag"),
								ui.UserSelect("none"),
							)
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Width(420),
						ui.Gap(18),
						ui.Padding(28),
						ui.BackgroundColor("#111827"),
						ui.BorderColor("#334155"),
						ui.BorderWidth(1),
						ui.BorderRadius(16),
					)
				},
				ui.Display("flex"),
				ui.Flex(1),
				ui.MinHeight(0),
				ui.AlignItems("center"),
				ui.JustifyContent("center"),
				ui.Padding(32),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.BackgroundColor("#090d16"),
		ui.Color("#e2e8f0"),
	)
}
