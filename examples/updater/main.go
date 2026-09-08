package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"github.com/egoist/quickgui/go/updater"
)

func main() {
	if err := native.Run(func() {
		status, setStatus := ui.CreateSignal("Starting updater…")
		state, setState := ui.CreateSignal(updater.Event{})
		report := func(err error) {
			if err != nil {
				setStatus(err.Error())
			}
		}
		updates := updater.Start(updater.Options{}, func(event updater.Event) {
			setState(event)
			switch {
			case event.Error != "":
				setStatus(event.Error)
			case event.Status == updater.Disabled:
				setStatus("Updates are disabled in development builds.")
			case event.QuitRequired:
				native.App.Quit(false, nil)
			case event.Kind == "up-to-date":
				setStatus("You’re up to date.")
			case event.Status == updater.Available:
				setStatus("Version " + event.Version + " is available.")
			default:
				setStatus(string(event.Status))
			}
		}, report)
		native.NewWindow(native.WindowOptions{
			Title:  "Updater Example",
			Width:  520,
			Height: 340,
			Component: func() {
				ui.View(
					func() {
						ui.Text("Application updates", ui.FontSize(24), ui.FontWeight(700))
						ui.Text(status, ui.LineHeight(22))
						ui.Text(
							"macOS uses Sparkle. Windows and Linux share its signed appcast format.",
							ui.Color("#64748b"),
							ui.LineHeight(22),
						)
						ui.Button(
							"Check for Updates…",
							ui.OnClick(func() { updates.Check(report) }),
							ui.Padding(10),
							ui.BackgroundColor("#2563eb"),
							ui.Color("white"),
							ui.BorderRadius(8),
						)
						ui.Show(
							func() bool { return state().Status == updater.Available },
							func() {
								ui.Button(
									"Install update",
									ui.OnClick(func() { updates.Install(report) }),
									ui.Padding(10),
									ui.BackgroundColor("#16a34a"),
									ui.Color("white"),
									ui.BorderRadius(8),
								)
							},
						)
						ui.Button(
							func() string {
								if state().AutomaticChecks {
									return "Disable automatic checks"
								}
								return "Enable automatic checks"
							},
							ui.OnClick(func() {
								updates.SetAutomaticChecks(!state().AutomaticChecks, report)
							}),
							ui.Padding(10),
						)
					},
					ui.Display("flex"),
					ui.FlexDirection("column"),
					ui.Gap(16),
					ui.Padding(28),
					ui.Width("100%"),
					ui.Height("100%"),
				)
			},
		})
	}); err != nil {
		log.Fatal(err)
	}
}
