package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func appView(m *model) {
	ui.View(
		func() {
			sidebar(m)
			workspace(m)
			agentSheet(m)
		},
		ui.Style{
			Position:        "relative",
			Display:         "flex",
			FlexDirection:   "row",
			Width:           "100%",
			Height:          "100%",
			MinWidth:        0,
			MinHeight:       0,
			BackgroundColor: m.color(func(t theme) string { return t.App }),
			Color:           m.color(func(t theme) string { return t.Text }),
			FontWeight:      500,
		},
	)
}
func workspace(m *model) {
	ui.View(
		func() {
			tabBar(m)
			ui.View(ui.Style{
				Height:          1,
				FlexShrink:      0,
				BackgroundColor: m.color(func(t theme) string { return t.Border }),
			})
			ui.View(
				func() {
					// Hide inactive surfaces without unmounting them. Each native PTY, scrollback,
					// selection and working directory survives tab and space navigation.
					ui.KeyedFor(
						m.Tabs.Read,
						func(t workspaceTab) any { return t.ID },
						func(tab func() workspaceTab, _ func() int) { tabSurface(m, tab) },
						nil,
					)
					ui.Show(
						func() bool { return m.activeTab() == nil },
						func() {
							ui.View(
								func() {
									icon("terminal", 24, m.color(func(t theme) string { return t.TextGhost }))
									ui.Text(
										"Open a tab to start working",
										ui.Style{
											Color:    m.color(func(t theme) string { return t.TextTertiary }),
											FontSize: 13.5,
										},
									)
									ui.Button(
										"New Tab",
										m.buttonStyle(false),
										ui.OnClick(func() { m.newTerminal(m.activeSpace()) }),
									)
								},
								ui.Style{
									Display:         "flex",
									Flex:            1,
									MinWidth:        0,
									MinHeight:       0,
									FlexDirection:   "column",
									AlignItems:      "center",
									JustifyContent:  "center",
									Gap:             9,
									BackgroundColor: m.color(func(t theme) string { return t.Terminal }),
								},
							)
						},
					)
				},
				ui.Style{Position: "relative", Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0},
			)
			errorMessage := func() string {
				if err := m.Error.Read(); err != "" {
					return err
				}
				if p := m.activePane(); p != nil && p.Status.Read().Status == "failed" {
					return p.Status.Read().Message
				}
				return ""
			}
			ui.Show(
				func() bool { return errorMessage() != "" },
				func() {
					ui.View(
						func() {
							ui.Text(
								errorMessage,
								ui.Style{
									Color:    m.color(func(t theme) string { return t.Danger }),
									FontSize: 11.5,
								},
							)
							ui.View(ui.Style{Flex: 1})
							ui.Show(
								func() bool {
									return m.Error.Read() == "" && m.activePane() != nil
								},
								func() {
									ui.Button(
										"Restart",
										m.buttonStyle(false),
										ui.Style{Height: 22, FontSize: 10.5},
										ui.OnClick(func() {
											if p := m.activePane(); p != nil {
												m.restartPane(p)
											}
										}),
									)
								},
								func() {
									iconButton(m, "Dismiss error", "close", 22, func() { m.Error.Write("") })
								},
							)
						},
						ui.Style{
							Display:         "flex",
							MinHeight:       30,
							FlexShrink:      0,
							AlignItems:      "center",
							Gap:             8,
							PaddingLeft:     12,
							PaddingRight:    8,
							BackgroundColor: m.color(func(t theme) string { return t.ErrorBackground }),
						},
					)
				},
			)
		},
		ui.Style{
			Display:         "flex",
			Flex:            1,
			MinWidth:        0,
			MinHeight:       0,
			FlexDirection:   "column",
			BackgroundColor: m.color(func(t theme) string { return t.App }),
		},
	)
}
func tabBar(m *model) {
	ui.View(
		func() {
			ui.View(
				func() {
					ui.KeyedFor(
						m.spaceTabs,
						func(t workspaceTab) any { return t.ID },
						func(tab func() workspaceTab, _ func() int) {
							active := func() bool { return tab().ID == m.ActiveTabID.Read() }
							ui.View(
								func() {
									ui.Button(
										func() {
											ui.Text(
												func() string {
													return tabTitle(tab(), m.Panes.Read())
												},
												ui.Style{
													FontSize:     12.5,
													FontWeight:   560,
													LineClamp:    1,
													TextOverflow: "ellipsis",
												},
											)
										},
										ui.AriaLabel("Select terminal tab"),
										ui.Style{
											Display:         "flex",
											Width:           "100%",
											MinWidth:        0,
											Height:          28,
											AlignItems:      "center",
											PaddingLeft:     10,
											PaddingRight:    func() int { return choose(active(), 30, 10) },
											BackgroundColor: "transparent",
											Color: func() string {
												return choose(active(), m.theme().Text, m.theme().TextTertiary)
											},
											Hover: &ui.Style{BackgroundColor: func() string {
												return choose(active(), m.theme().SelectedStrong, m.theme().Hover)
											}},
											Active:       &ui.Style{BackgroundColor: m.color(func(t theme) string { return t.Active })},
											BorderRadius: 6,
											Cursor:       "default",
											AppRegion:    "no-drag",
										},
										ui.OnClick(func() { m.selectTab(tab()) }),
									)
									ui.Show(
										active,
										func() {
											iconButton(m, "Close tab", "close", 20, func() { m.closeTab(tab().ID) }, ui.Style{
												Position: "absolute",
												Top:      4,
												Right:    3,
											})
										},
									)
								},
								ui.Style{
									Position:   "relative",
									Display:    "flex",
									MinWidth:   74,
									MaxWidth:   170,
									Height:     28,
									FlexShrink: 1,
									AlignItems: "center",
									BackgroundColor: func() string {
										return choose(active(), m.theme().Selected, "transparent")
									},
									BorderRadius: 6,
								},
								ui.Group("tab"),
							)
						},
						nil,
					)
					iconButton(m, "New tab", "plus", 24, func() { m.newTerminal(m.activeSpace()) })
				},
				ui.Style{
					Display:    "flex",
					MinWidth:   0,
					AlignItems: "center",
					Gap:        3,
					Overflow:   "hidden",
					AppRegion:  "no-drag",
				},
			)
			ui.View(ui.Style{Flex: 1, AppRegion: "drag"})
			ui.Button(
				func() {
					icon("plus", 13, m.color(func(t theme) string { return t.TextSecondary }))
					ui.Text("Agent")
				},
				ui.AriaLabel("New agent"),
				ui.FocusOnPointer(ptr(false)),
				ui.Style{
					Display:         "flex",
					Height:          24,
					FlexShrink:      0,
					AlignItems:      "center",
					JustifyContent:  "center",
					Gap:             5,
					PaddingLeft:     9,
					PaddingRight:    9,
					Color:           m.color(func(t theme) string { return t.TextSecondary }),
					BackgroundColor: "transparent",
					Hover:           &ui.Style{BackgroundColor: m.color(func(t theme) string { return t.Hover })},
					Active:          &ui.Style{BackgroundColor: m.color(func(t theme) string { return t.Active })},
					Transition:      colorTransition,
					BorderRadius:    5,
					FontSize:        11.5,
					FontWeight:      600,
					Cursor:          "default",
					AppRegion:       "no-drag",
				},
				ui.OnClick(m.openAgentSheet),
			)
			ui.View(ui.Style{
				Width:           1,
				Height:          16,
				FlexShrink:      0,
				MarginLeft:      2,
				MarginRight:     2,
				BackgroundColor: m.color(func(t theme) string { return t.BorderStrong }),
			})
			iconButton(m, "Split right", "columns", 24, func() { m.splitTerminal("horizontal") })
			iconButton(m, "Split down", "rows", 24, func() { m.splitTerminal("vertical") })
		},
		ui.Style{
			Display:      "flex",
			Height:       40,
			FlexShrink:   0,
			AlignItems:   "center",
			Gap:          4,
			PaddingLeft:  8,
			PaddingRight: 8,
			AppRegion:    "drag",
		},
	)
}
func tabSurface(m *model, tab func() workspaceTab) {
	ui.View(
		func() {
			// Restart replaces the pane object; other model updates preserve its identity.
			ui.For(
				func() []*pane { return m.tabPanes(tab().ID) },
				func(p *pane, _ func() int) { terminalPane(m, p) },
				func(p *pane) any { return p },
				nil,
			)
		},
		ui.Style{
			Position:      "absolute",
			Display:       "flex",
			Top:           0,
			Right:         0,
			Bottom:        0,
			Left:          0,
			MinWidth:      0,
			MinHeight:     0,
			Gap:           5,
			FlexDirection: "row",
		},
		ui.When(
			func() bool { return tab().Direction == "vertical" },
			ui.Style{FlexDirection: "column"},
		),
		ui.When(
			func() bool { return tab().ID != m.ActiveTabID.Read() },
			ui.Style{Visibility: "hidden"},
		),
	)
}
func terminalPane(m *model, p *pane) {
	directory := ""
	for _, s := range m.Spaces.Peek() {
		if s.ID == p.SpaceID {
			directory = s.Path
		}
	}
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Terminal(ui.TerminalProps{
						Program:          p.Program,
						Args:             p.Arguments,
						WorkingDirectory: directory,
						Environment:      p.Environment,
						Scrollback:       50000,
						Palette:          func() ui.TerminalPalette { return m.theme().TerminalPalette },
						CursorColor:      m.color(func(t theme) string { return t.TerminalCursor }),
						PaddingColor:     "extend",
						FontThicken:      true,
						OnStatus: func(event *native.Event) {
							if status := ui.TerminalStatusFromEvent(event); status != nil {
								p.Status.Write(*status)
							}
						},
						Props: ui.Props{
							Ref:       func(node *native.Node) { m.registerTerminal(p, node) },
							AriaLabel: "Terminal pane",
							OnClick:   func(*native.Event) { m.selectPane(p) },
							Style: ui.Style{
								Position:        "absolute",
								Top:             0,
								Right:           0,
								Bottom:          0,
								Left:            0,
								Padding:         8,
								BorderWidth:     1,
								BorderColor:     m.color(func(t theme) string { return t.Terminal }),
								BackgroundColor: m.color(func(t theme) string { return t.Terminal }),
								Color:           m.color(func(t theme) string { return t.TerminalText }),
								FontFamily:      "JetBrainsMono Nerd Font Mono",
								FontSize:        14,
								FontWeight:      400,
								LineHeight:      20.5,
							},
						},
					})
				},
				ui.Style{Position: "relative", Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0},
			)
		},
		ui.Style{
			Display:         "flex",
			FlexGrow:        1,
			FlexBasis:       0,
			MinWidth:        0,
			MinHeight:       0,
			FlexDirection:   "column",
			BackgroundColor: m.color(func(t theme) string { return t.Terminal }),
		},
	)
}
