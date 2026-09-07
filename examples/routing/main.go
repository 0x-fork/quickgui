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
						func() { ui.Text("Router", ui.Style{FontWeight: 700}) },
						ui.Style{
							Display:      "flex",
							Height:       "100%",
							AlignItems:   "center",
							PaddingRight: 10,
							AppRegion:    "drag",
						},
					)
					link("Home", "/", true)
					link("Projects", "/projects", false)
					link("Settings", "/settings", false)
					ui.View(ui.Style{Flex: 1, Height: "100%", AppRegion: "drag"})
					historyButton("Go back", "M19 12H5m6-6-6 6 6 6", router.Back, func() bool { return !router.State().CanGoBack })
					historyButton("Go forward", "M5 12h14m-6-6 6 6-6 6", router.Forward, func() bool { return !router.State().CanGoForward })
				},
				ui.Style{
					Display:           "flex",
					Height:            54,
					FlexShrink:        0,
					AlignItems:        "center",
					PaddingLeft:       78,
					PaddingRight:      16,
					Gap:               8,
					BorderBottomWidth: 1,
					BorderColor:       border,
				},
			)
			ui.View(
				func() {
					ui.Text(
						func() string { return location().Href },
						ui.Style{
							FontFamily: "monospace",
							FontSize:   12,
							Color:      muted,
						},
					)
				},
				ui.Style{
					Display:           "flex",
					Height:            34,
					FlexShrink:        0,
					AlignItems:        "center",
					PaddingLeft:       20,
					PaddingRight:      20,
					BackgroundColor:   "#0e1526",
					BorderBottomWidth: 1,
					BorderColor:       border,
				},
			)
			ui.View(
				func() { ui.Outlet() },
				ui.Style{Flex: 1, MinHeight: 0},
			)
		},
		ui.Style{
			Display:         "flex",
			FlexDirection:   "column",
			Width:           "100%",
			Height:          "100%",
			BackgroundColor: background,
			Color:           textColor,
		},
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
			ui.Style{Display: "flex", FlexWrap: "wrap", Gap: 12},
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
			ui.Style{Display: "flex", FlexWrap: "wrap", Gap: 12},
		)
	})
}

func Project() {
	projectID, search, navigate := ui.UseParam("projectId"), ui.UseSearchParams(), ui.UseNavigate()
	page([]any{"Project: ", projectID}, "The page stays mounted when only the query changes; its reactive core snapshot updates in place.", func() {
		ui.View(
			func() {
				ui.Text("Decoded :projectId", ui.Style{Color: muted})
				ui.Text(projectID, ui.Style{FontFamily: "monospace", Color: blue})
				ui.Text("Decoded ?tab", ui.Style{MarginTop: 8, Color: muted})
				ui.Text(
					func() string {
						if value, ok := search()["tab"]; ok {
							return value
						}
						return "overview"
					},
					ui.Style{FontFamily: "monospace", Color: blue},
				)
				ui.View(
					func() {
						button("Overview", func() { navigate("?tab=overview") })
						button("Activity", func() { navigate("?tab=activity") })
						button("Replace", func() { navigate("?tab=activity", ui.NavigateOptions{Replace: true}) }, ui.Style{BackgroundColor: "#322847"})
					},
					ui.Style{Display: "flex", Gap: 8, MarginTop: 8},
				)
			},
			ui.Style{
				Display:         "flex",
				FlexDirection:   "column",
				Width:           440,
				MaxWidth:        "100%",
				Padding:         18,
				Gap:             12,
				BackgroundColor: panel,
				BorderWidth:     1,
				BorderColor:     border,
				BorderRadius:    12,
			},
		)
	})
}

func SettingsLayout() {
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text("Settings", ui.Style{MarginBottom: 6, FontWeight: 700})
					link("General", "/settings", true)
					link("Appearance", "/settings/appearance", false)
				},
				ui.Style{
					Display:          "flex",
					FlexDirection:    "column",
					Width:            190,
					FlexShrink:       0,
					Padding:          16,
					Gap:              8,
					BackgroundColor:  panel,
					BorderRightWidth: 1,
					BorderColor:      border,
				},
			)
			ui.View(
				func() { ui.Outlet() },
				ui.Style{Flex: 1, MinWidth: 0},
			)
		},
		ui.Style{Display: "flex", Width: "100%", Height: "100%"},
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
		button("Back home", func() { navigate("/", ui.NavigateOptions{Replace: true}) }, ui.Style{
			Width:           140,
			Height:          38,
			BackgroundColor: blueSurface,
		})
	})
}
