package main

import (
	"github.com/egoist/quickgui/go/ui"
)

func main() { run("Routing", 860, 600, Routes) }

func Routes() {
	ui.Router(ui.RouterProps{
		InitialPath: "/",
		Routes: []*ui.RouteDeclaration{ui.Layout(
			Navigation,
			ui.Route(
				"/",
				func() { card(ui.Text("Home")) },
			),
			ui.Route(
				"/projects",
				func() { card(ui.Text("Projects")) },
			),
			ui.Route(
				"/projects/:id",
				func() {
					params := ui.UseParams()
					card(ui.Text(func() string { return "Project: " + params()["id"] }))
				},
			),
			ui.Route(
				"/settings",
				func() { card(ui.Text("Settings")) },
			),
		)},
	})
}

func Navigation() {
	router := ui.UseRouter()
	location := ui.UseLocation()
	var links []any
	for _, destination := range []string{"/", "/projects", "/projects/quickgui?tab=code", "/settings"} {
		links = append(links, func() {
			button(destination, func() { router.Push(destination) })
		})
	}
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(20),
		func() {
			ui.View(ui.Display("flex"), ui.Gap(8), links)
			ui.View(
				ui.Display("flex"),
				ui.Gap(8),
				func() {
					ui.Button(
						ui.WithStyle(buttonStyle),
						ui.Disabled(func() bool { return !router.State().CanGoBack }),
						ui.OnClick(func() { router.Back() }),
						"Back",
					)
					ui.Button(
						ui.WithStyle(buttonStyle),
						ui.Disabled(func() bool { return !router.State().CanGoForward }),
						ui.OnClick(func() { router.Forward() }),
						"Forward",
					)
				},
			)
			ui.Text(
				ui.FontSize(28),
				func() string { return location().Href },
			)
			ui.Outlet()
			ui.Text("Native route matching and history, with Go signals for location and parameters.")
		},
	)
}
