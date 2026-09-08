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
				Title:                "QuickGUI Routing",
				Width:                820,
				Height:               560,
				MinimumWidth:         680,
				MinimumHeight:        440,
				Background:           background,
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 18},
				Component:            Routes,
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

func Routes() {
	ui.Router(ui.RouterProps{
		InitialPath: "/",
		Routes: []*ui.RouteDeclaration{ui.Route(
			"/",
			Navigation,
			ui.Route("/", Home),
			ui.Route("/projects", Projects),
			ui.Route("/projects/:projectId", Project),
			ui.Route(
				"/settings",
				SettingsLayout,
				ui.Route("", GeneralSettings),
				ui.Route("appearance", AppearanceSettings),
			),
			ui.Route("*", NotFound),
		)},
	})
}

func Navigation() {
	router, location := ui.UseRouter(), ui.UseLocation()
	ui.View(
		func() {
			ui.View(
				func() {
					ui.View(
						func() { ui.Text("Router", ui.FontWeight(700)) },
						ui.Display("flex"),
						ui.Height("100%"),
						ui.AlignItems("center"),
						ui.PaddingRight(10),
						ui.AppRegion("drag"),
					)
					link("Home", "/", true)
					link("Projects", "/projects", false)
					link("Settings", "/settings", false)
					ui.View(ui.Flex(1), ui.Height("100%"), ui.AppRegion("drag"))
					historyButton("Go back", "M19 12H5m6-6-6 6 6 6", router.Back, func() bool { return !router.State().CanGoBack })
					historyButton("Go forward", "M5 12h14m-6-6 6 6-6 6", router.Forward, func() bool { return !router.State().CanGoForward })
				},
				ui.Display("flex"),
				ui.Height(54),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.PaddingLeft(78),
				ui.PaddingRight(16),
				ui.Gap(8),
				ui.BorderBottomWidth(1),
				ui.BorderColor(border),
			)
			ui.View(
				func() {
					ui.Text(
						func() string { return location().Href },
						ui.FontFamily("monospace"),
						ui.FontSize(12),
						ui.Color(muted),
					)
				},
				ui.Display("flex"),
				ui.Height(34),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.PaddingLeft(20),
				ui.PaddingRight(20),
				ui.BackgroundColor("#0e1526"),
				ui.BorderBottomWidth(1),
				ui.BorderColor(border),
			)
			ui.View(
				func() { ui.Outlet() },
				ui.Flex(1),
				ui.MinHeight(0),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.BackgroundColor(background),
		ui.Color(textColor),
	)
}

func Home() {
	page("Core-owned routing", "The native core owns matching, decoded parameters, query parsing, active paths, and bounded memory history. The application renders the returned route chain.", func() {
		ui.View(
			func() {
				card("Dynamic parameters", "Open /projects/quickgui and read :projectId from the matched core route.", "/projects/quickgui?tab=overview")
				card("Nested layouts", "Settings keeps its local navigation mounted while its Outlet changes.", "/settings")
				card("Fallback routes", "A final wildcard catches destinations that no specific pattern matched.", "/this-route-does-not-exist")
			},
			ui.Display("flex"),
			ui.FlexWrap("wrap"),
			ui.Gap(12),
		)
	})
}

func Projects() {
	page("Projects", "These links push memory-history entries. Use the title-bar arrows to traverse them.", func() {
		ui.View(
			func() {
				card("QuickGUI", "A native retained UI framework.", "/projects/quickgui?tab=overview")
				card("Screenflare", "A polished native screen recorder.", "/projects/screenflare?tab=activity")
			},
			ui.Display("flex"),
			ui.FlexWrap("wrap"),
			ui.Gap(12),
		)
	})
}

func Project() {
	projectID, search, navigate := ui.UseParam("projectId"), ui.UseSearchParams(), ui.UseNavigate()
	page([]any{"Project: ", projectID}, "The page stays mounted when only the query changes; its reactive core snapshot updates in place.", func() {
		ui.View(
			func() {
				ui.Text("Decoded :projectId", ui.Color(muted))
				ui.Text(projectID, ui.FontFamily("monospace"), ui.Color(blue))
				ui.Text("Decoded ?tab", ui.MarginTop(8), ui.Color(muted))
				ui.Text(
					func() string {
						if value, ok := search()["tab"]; ok {
							return value
						}
						return "overview"
					},
					ui.FontFamily("monospace"),
					ui.Color(blue),
				)
				ui.View(
					func() {
						button("Overview", func() { navigate("?tab=overview") })
						button("Activity", func() { navigate("?tab=activity") })
						button("Replace", func() { navigate("?tab=activity", ui.NavigateOptions{Replace: true}) }, ui.Styles(ui.BackgroundColor("#322847")))
					},
					ui.Display("flex"),
					ui.Gap(8),
					ui.MarginTop(8),
				)
			},
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.Width(440),
			ui.MaxWidth("100%"),
			ui.Padding(18),
			ui.Gap(12),
			ui.BackgroundColor(panel),
			ui.BorderWidth(1),
			ui.BorderColor(border),
			ui.BorderRadius(12),
		)
	})
}

func SettingsLayout() {
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text("Settings", ui.MarginBottom(6), ui.FontWeight(700))
					link("General", "/settings", true)
					link("Appearance", "/settings/appearance", false)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width(190),
				ui.FlexShrink(0),
				ui.Padding(16),
				ui.Gap(8),
				ui.BackgroundColor(panel),
				ui.BorderRightWidth(1),
				ui.BorderColor(border),
			)
			ui.View(
				func() { ui.Outlet() },
				ui.Flex(1),
				ui.MinWidth(0),
			)
		},
		ui.Display("flex"),
		ui.Width("100%"),
		ui.Height("100%"),
	)
}

func GeneralSettings() {
	page("General", "This is the index child at the same /settings path.")
}

func AppearanceSettings() {
	page("Appearance", "The settings layout is shared; only this nested Outlet branch changes.")
}

func NotFound() {
	wildcard, navigate := ui.UseParam("*"), ui.UseNavigate()
	page("Route not found", []any{"The core wildcard captured: ", wildcard}, func() {
		button("Back home", func() { navigate("/", ui.NavigateOptions{Replace: true}) }, ui.Styles(
			ui.Width(140),
			ui.Height(38),
			ui.BackgroundColor(blueSurface),
		))
	})
}
