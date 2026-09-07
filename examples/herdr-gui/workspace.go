package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func appView(m *model) {
	ui.View(
		ui.Position("relative"),
		ui.Display("flex"),
		ui.FlexDirection("row"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.BackgroundColor(m.color(func(t theme) string { return t.App })),
		ui.Color(m.color(func(t theme) string { return t.Text })),
		ui.FontWeight(500),
		func() {
			sidebar(m)
			workspace(m)
			agentSheet(m)
		},
	)
}
func workspace(m *model) {
	ui.View(
		ui.Display("flex"),
		ui.Flex(1),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.FlexDirection("column"),
		ui.BackgroundColor(m.color(func(t theme) string { return t.App })),
		func() {
			tabBar(m)
			ui.View(
				ui.Height(1),
				ui.FlexShrink(0),
				ui.BackgroundColor(m.color(func(t theme) string { return t.Border })),
			)
			ui.View(
				ui.Position("relative"),
				ui.Display("flex"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.MinHeight(0),
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
								ui.Display("flex"),
								ui.Flex(1),
								ui.MinWidth(0),
								ui.MinHeight(0),
								ui.FlexDirection("column"),
								ui.AlignItems("center"),
								ui.JustifyContent("center"),
								ui.Gap(9),
								ui.BackgroundColor(m.color(func(t theme) string { return t.Terminal })),
								func() {
									icon("terminal", 24, m.color(func(t theme) string { return t.TextGhost }))
									ui.Text(
										ui.Color(m.color(func(t theme) string { return t.TextTertiary })),
										ui.FontSize(13.5),
										"Open a tab to start working",
									)
									ui.Button(
										ui.WithStyle(m.buttonStyle(false)),
										ui.OnClick(func() { m.newTerminal(m.activeSpace()) }),
										"New Tab",
									)
								},
							)
						},
					)
				},
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
						ui.Display("flex"),
						ui.MinHeight(30),
						ui.FlexShrink(0),
						ui.AlignItems("center"),
						ui.Gap(8),
						ui.PaddingLeft(12),
						ui.PaddingRight(8),
						ui.BackgroundColor(m.color(func(t theme) string { return t.ErrorBackground })),
						func() {
							ui.Text(
								ui.Color(m.color(func(t theme) string { return t.Danger })),
								ui.FontSize(11.5),
								errorMessage,
							)
							ui.View(ui.Flex(1))
							ui.Show(
								func() bool {
									return m.Error.Read() == "" && m.activePane() != nil
								},
								func() {
									ui.Button(
										ui.WithStyle(m.buttonStyle(false)),
										ui.Height(22),
										ui.FontSize(10.5),
										ui.OnClick(func() {
											if p := m.activePane(); p != nil {
												m.restartPane(p)
											}
										}),
										"Restart",
									)
								},
								func() {
									iconButton(m, "Dismiss error", "close", 22, func() { m.Error.Write("") })
								},
							)
						},
					)
				},
			)
		},
	)
}
func tabBar(m *model) {
	ui.View(
		ui.Display("flex"),
		ui.Height(40),
		ui.FlexShrink(0),
		ui.AlignItems("center"),
		ui.Gap(4),
		ui.PaddingLeft(8),
		ui.PaddingRight(8),
		ui.AppRegion("drag"),
		func() {
			ui.View(
				ui.Display("flex"),
				ui.MinWidth(0),
				ui.AlignItems("center"),
				ui.Gap(3),
				ui.Overflow("hidden"),
				ui.AppRegion("no-drag"),
				func() {
					ui.KeyedFor(
						m.spaceTabs,
						func(t workspaceTab) any { return t.ID },
						func(tab func() workspaceTab, _ func() int) {
							active := func() bool { return tab().ID == m.ActiveTabID.Read() }
							ui.View(
								ui.Position("relative"),
								ui.Display("flex"),
								ui.MinWidth(74),
								ui.MaxWidth(170),
								ui.Height(28),
								ui.FlexShrink(1),
								ui.AlignItems("center"),
								ui.BackgroundColor(func() string {
									return choose(active(), m.theme().Selected, "transparent")
								}),
								ui.BorderRadius(6),
								ui.Group("tab"),
								func() {
									ui.Button(
										ui.AriaLabel("Select terminal tab"),
										ui.Display("flex"),
										ui.Width("100%"),
										ui.MinWidth(0),
										ui.Height(28),
										ui.AlignItems("center"),
										ui.PaddingLeft(10),
										ui.PaddingRight(func() int { return choose(active(), 30, 10) }),
										ui.BackgroundColor("transparent"),
										ui.Color(func() string {
											return choose(active(), m.theme().Text, m.theme().TextTertiary)
										}),
										ui.Hover(ui.BackgroundColor(func() string {
											return choose(active(), m.theme().SelectedStrong, m.theme().Hover)
										})),
										ui.Active(ui.BackgroundColor(m.color(func(t theme) string { return t.Active }))),
										ui.BorderRadius(6),
										ui.Cursor("default"),
										ui.AppRegion("no-drag"),
										ui.OnClick(func() { m.selectTab(tab()) }),
										func() {
											ui.Text(
												ui.FontSize(12.5),
												ui.FontWeight(560),
												ui.LineClamp(1),
												ui.TextOverflow("ellipsis"),
												func() string {
													return tabTitle(tab(), m.Panes.Read())
												},
											)
										},
									)
									ui.Show(
										active,
										func() {
											iconButton(m, "Close tab", "close", 20, func() { m.closeTab(tab().ID) }, ui.Position("absolute"), ui.Top(4), ui.Right(3))
										},
									)
								},
							)
						},
						nil,
					)
					iconButton(m, "New tab", "plus", 24, func() { m.newTerminal(m.activeSpace()) })
				},
			)
			ui.View(ui.Flex(1), ui.AppRegion("drag"))
			ui.Button(
				ui.AriaLabel("New agent"),
				ui.FocusOnPointer(ptr(false)),
				ui.Display("flex"),
				ui.Height(24),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.JustifyContent("center"),
				ui.Gap(5),
				ui.PaddingLeft(9),
				ui.PaddingRight(9),
				ui.Color(m.color(func(t theme) string { return t.TextSecondary })),
				ui.BackgroundColor("transparent"),
				ui.Hover(ui.BackgroundColor(m.color(func(t theme) string { return t.Hover }))),
				ui.Active(ui.BackgroundColor(m.color(func(t theme) string { return t.Active }))),
				ui.Transition(colorTransition),
				ui.BorderRadius(5),
				ui.FontSize(11.5),
				ui.FontWeight(600),
				ui.Cursor("default"),
				ui.AppRegion("no-drag"),
				ui.OnClick(m.openAgentSheet),
				func() {
					icon("plus", 13, m.color(func(t theme) string { return t.TextSecondary }))
					ui.Text("Agent")
				},
			)
			ui.View(
				ui.Width(1),
				ui.Height(16),
				ui.FlexShrink(0),
				ui.MarginLeft(2),
				ui.MarginRight(2),
				ui.BackgroundColor(m.color(func(t theme) string { return t.BorderStrong })),
			)
			iconButton(m, "Split right", "columns", 24, func() { m.splitTerminal("horizontal") })
			iconButton(m, "Split down", "rows", 24, func() { m.splitTerminal("vertical") })
		},
	)
}
func tabSurface(m *model, tab func() workspaceTab) {
	ui.View(
		ui.Position("absolute"),
		ui.Display("flex"),
		ui.Top(0),
		ui.Right(0),
		ui.Bottom(0),
		ui.Left(0),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.Gap(5),
		ui.FlexDirection("row"),
		ui.When(
			func() bool { return tab().Direction == "vertical" },
			ui.FlexDirection("column"),
		),
		ui.When(
			func() bool { return tab().ID != m.ActiveTabID.Read() },
			ui.Visibility("hidden"),
		),
		func() {
			// Restart replaces the pane object; other model updates preserve its identity.
			ui.For(
				func() []*pane { return m.tabPanes(tab().ID) },
				func(p *pane, _ func() int) { terminalPane(m, p) },
				func(p *pane) any { return p },
				nil,
			)
		},
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
		ui.Display("flex"),
		ui.FlexGrow(1),
		ui.FlexBasis(0),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.FlexDirection("column"),
		ui.BackgroundColor(m.color(func(t theme) string { return t.Terminal })),
		func() {
			ui.View(
				ui.Position("relative"),
				ui.Display("flex"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.MinHeight(0),
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
			)
		},
	)
}
