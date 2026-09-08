package main

import (
	"log"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

const identifier = "dev.quickgui.system-api-example"
const appName = "QuickGUI System APIs"

func main() {
	if err := native.Run(func() {
		native.App.RequestSingleInstanceLock(
			identifier,
			func(primary bool, err error) {
				if err != nil {
					log.Print(err)
					native.App.Exit(1, nil)
					return
				}
				if !primary {
					native.App.Exit(0, nil)
					return
				}
				open := func() {
					native.NewWindow(native.WindowOptions{
						Title:                appName,
						Width:                760,
						Height:               640,
						MinimumWidth:         620,
						MinimumHeight:        520,
						Background:           "#0b0e14",
						TitleBarStyle:        "hiddenInset",
						TrafficLightPosition: &native.Point{X: 16, Y: 14},
						Component:            SystemAPIs,
					})
				}
				native.App.OnReopen(func(event native.ReopenEvent) {
					if !event.HasVisibleWindows {
						open()
					}
				})
				open()
			},
		)
	}); err != nil {
		log.Fatal(err)
	}
}

func SystemAPIs() {
	window := native.CurrentWindow()
	status, setStatus := ui.CreateSignal("Primary instance")
	busy, setBusy := ui.CreateSignal(false)
	state := &systemState{window: window, status: setStatus, busy: busy, setBusy: setBusy}
	ui.OnCleanup(state.dispose)
	native.App.OnSecondInstance(func(event native.SecondInstanceEvent) {
		state.showWindow()
		setStatus("A second launch forwarded " + strconv.Itoa(len(event.Argv)) + " argument(s).")
	})
	native.App.OnOpenURLs(func(event native.OpenURLsEvent) {
		setStatus("Deep link: " + strings.Join(event.URLs, ", "))
	})
	native.PowerMonitor.OnEvent(func(event native.PowerEvent) { setStatus("Power event: " + event.Type) })
	native.SystemPreferences.OnChange(func(value native.SystemPreferencesSnapshot) {
		setStatus("System preferences changed to " + value.ColorScheme + " appearance.")
	})
	native.App.OnNotificationResponse(func(event native.NotificationResponseEvent) {
		message := "Notification " + event.Tag + " activated"
		if event.Reply != nil {
			message += ": " + *event.Reply
		} else if event.ActionID != nil {
			message += " via " + *event.ActionID
		}
		setStatus(message)
	})
	native.SetApplicationMenu([]native.MenuDefinition{
		{Label: "App", Items: []native.MenuItem{
			{Label: "Show window", Click: state.showWindow},
			{Type: "separator"},
			{Label: "Hide", Role: "hide-application"},
			{Label: "Quit", Role: "quit"},
		}},
		{Label: "Edit", Items: []native.MenuItem{
			{Label: "Copy", Role: "copy"}, {Label: "Paste", Role: "paste"}, {Label: "Select All", Role: "select-all"},
		}},
		{Label: "Window", Items: []native.MenuItem{
			{Label: "Minimize", Role: "minimize-window"}, {Label: "Close", Role: "close-window"},
		}},
	})
	ui.View(
		func() {
			ui.View(
				"Native system APIs",
				ui.Display("flex"),
				ui.Height(52),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.JustifyContent("center"),
				ui.FontSize(14),
				ui.FontWeight(600),
				ui.AppRegion("drag"),
				ui.BorderColor("#1f2530"),
				ui.BorderBottomWidth(1),
			)
			ui.View(
				func() {
					ui.Text(
						"Native integrations",
						ui.FontSize(26),
						ui.LineHeight(32),
						ui.FontWeight(700),
					)
					ui.Text(
						"Typed Go APIs for application state, desktop services, notifications, and native resources.",
						ui.Color("#9aa6b7"),
						ui.FontSize(14),
						ui.LineHeight(21),
					)
					ui.View(
						func() {
							for _, item := range state.actions() {
								ui.Button(
									item.label,
									buttonStyle,
									ui.Disabled(busy),
									ui.OnClick(func() { state.run(item) }),
								)
							}
						},
						ui.Display("flex"),
						ui.FlexWrap("wrap"),
						ui.Gap(10),
					)
					ui.View(
						func() {
							ui.Text(
								status,
								ui.FontSize(13),
								ui.LineHeight(19),
								ui.UserSelect("text"),
								ui.FontFamily("monospace"),
								ui.Color(func() string {
									if busy() {
										return "#c7d2fe"
									}
									return "#aeb9c9"
								}),
							)
						},
						ui.MinHeight(68),
						ui.Padding(16),
						ui.BackgroundColor("#111620"),
						ui.BorderColor("#293242"),
						ui.BorderWidth(1),
						ui.BorderRadius(10),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.MinHeight(0),
				ui.Gap(18),
				ui.Padding(28),
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
