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
						ui.Style{FontSize: 22, FontWeight: 700},
					)
					ui.Text(
						"This window has its own retained tree and native lifecycle.",
						ui.Style{Color: "#94a3b8", LineHeight: 21},
					)
					ui.Button(
						"Close window",
						ui.OnClick(func() { window.Close() }),
						ui.Style{
							Display:         "flex",
							Height:          40,
							AlignItems:      "center",
							JustifyContent:  "center",
							BackgroundColor: "#334155",
							BorderRadius:    9,
							Cursor:          "default",
						},
					)
				},
				ui.Style{
					Display:         "flex",
					FlexDirection:   "column",
					Width:           "100%",
					Height:          "100%",
					JustifyContent:  "center",
					Gap:             16,
					Padding:         28,
					BackgroundColor: "#111827",
					Color:           "#e2e8f0",
				},
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
					ui.Text("QuickGUI · Go", ui.Style{FontWeight: 600})
				},
				ui.Style{
					Display:        "flex",
					Height:         52,
					FlexShrink:     0,
					AlignItems:     "center",
					JustifyContent: "center",
					AppRegion:      "drag",
					BorderColor:    "#1e293b",
					BorderWidth:    1,
				},
			)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.Text(
								"Fine-grained native UI",
								ui.Style{FontSize: 28, LineHeight: 36, FontWeight: 700},
							)
							ui.Text(
								"Signals update only the changed text node. The application is ordinary Go, "+
									"and QuickGUI retains layout, sleeps while clean, and redraws once per mutation batch.",
								ui.Style{Color: "#94a3b8", FontSize: 14, LineHeight: 21},
							)
							ui.Text(
								"Count: ",
								count,
								ui.Style{Color: "#bfdbfe"},
								ui.When(
									func() bool { return count() >= 5 },
									ui.Style{Color: "#fbbf24"},
								),
								ui.Style{FontSize: 20, FontWeight: 600},
							)
							ui.Show(
								func() bool { return count() >= 5 },
								func() {
									ui.Text(
										"Five or more clicks: the row above was created on demand.",
										ui.Style{Color: "#fbbf24", FontSize: 14},
									)
								},
							)
							ui.Button(
								"Increment",
								ui.OnClick(func() { setCount(count() + 1) }),
								ui.Style{
									Display:         "flex",
									Height:          44,
									AlignItems:      "center",
									JustifyContent:  "center",
									BackgroundColor: "#2563eb",
									Color:           "white",
									BorderRadius:    9,
									Cursor:          "default",
									AppRegion:       "no-drag",
									UserSelect:      "none",
									Hover:           &ui.Style{BackgroundColor: "#3b82f6"},
								},
							)
							ui.Button(
								"Open window",
								ui.OnClick(func() { openDetailsWindow() }),
								ui.Style{
									Display:         "flex",
									Height:          44,
									AlignItems:      "center",
									JustifyContent:  "center",
									BackgroundColor: "#334155",
									Color:           "white",
									BorderRadius:    9,
									Cursor:          "default",
									AppRegion:       "no-drag",
									UserSelect:      "none",
								},
							)
						},
						ui.Style{
							Display:         "flex",
							FlexDirection:   "column",
							Width:           420,
							Gap:             18,
							Padding:         28,
							BackgroundColor: "#111827",
							BorderColor:     "#334155",
							BorderWidth:     1,
							BorderRadius:    16,
						},
					)
				},
				ui.Style{
					Display:        "flex",
					Flex:           1,
					MinHeight:      0,
					AlignItems:     "center",
					JustifyContent: "center",
					Padding:        32,
				},
			)
		},
		ui.Style{
			Display:         "flex",
			FlexDirection:   "column",
			Width:           "100%",
			Height:          "100%",
			BackgroundColor: "#090d16",
			Color:           "#e2e8f0",
		},
	)
}
