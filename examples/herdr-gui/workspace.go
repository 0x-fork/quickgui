package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/terminal"
	"github.com/egoist/quickgui/go/ui"
)

func appView(m *model) {
	ui.View(
		func() {
			sidebar(m)
			workspace(m)
			agentSheet(m)
		},
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
	)
}
func workspace(m *model) {
	ui.View(
		func() {
			tabBar(m)
			ui.View(
				ui.Height(1),
				ui.FlexShrink(0),
				ui.BackgroundColor(m.color(func(t theme) string { return t.Border })),
			)
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
										ui.Color(m.color(func(t theme) string { return t.TextTertiary })),
										ui.FontSize(13.5),
									)
									ui.Button(
										"New Tab",
										m.buttonStyle(false),
										ui.OnClick(func() { m.newTerminal(m.activeSpace()) }),
									)
								},
								ui.Display("flex"),
								ui.Flex(1),
								ui.MinWidth(0),
								ui.MinHeight(0),
								ui.FlexDirection("column"),
								ui.AlignItems("center"),
								ui.JustifyContent("center"),
								ui.Gap(9),
								ui.BackgroundColor(m.color(func(t theme) string { return t.Terminal })),
							)
						},
					)
				},
				ui.Position("relative"),
				ui.Display("flex"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.MinHeight(0),
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
								ui.Color(m.color(func(t theme) string { return t.Danger })),
								ui.FontSize(11.5),
							)
							ui.View(ui.Flex(1))
							ui.Show(
								func() bool {
									return m.Error.Read() == "" && m.activePane() != nil
								},
								func() {
									ui.Button(
										"Restart",
										m.buttonStyle(false),
										ui.Height(22),
										ui.FontSize(10.5),
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
						ui.Display("flex"),
						ui.MinHeight(30),
						ui.FlexShrink(0),
						ui.AlignItems("center"),
						ui.Gap(8),
						ui.PaddingLeft(12),
						ui.PaddingRight(8),
						ui.BackgroundColor(m.color(func(t theme) string { return t.ErrorBackground })),
					)
				},
			)
		},
		ui.Display("flex"),
		ui.Flex(1),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.FlexDirection("column"),
		ui.BackgroundColor(m.color(func(t theme) string { return t.App })),
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
												ui.FontSize(12.5),
												ui.FontWeight(560),
												ui.LineClamp(1),
												ui.TextOverflow("ellipsis"),
											)
										},
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
									)
									ui.Show(
										active,
										func() {
											iconButton(m, "Close tab", "close", 20, func() { m.closeTab(tab().ID) }, ui.Styles(
												ui.Position("absolute"),
												ui.Top(4),
												ui.Right(3),
											))
										},
									)
								},
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
							)
						},
						nil,
					)
					iconButton(m, "New tab", "plus", 24, func() { m.newTerminal(m.activeSpace()) })
				},
				ui.Display("flex"),
				ui.MinWidth(0),
				ui.AlignItems("center"),
				ui.Gap(3),
				ui.Overflow("hidden"),
				ui.AppRegion("no-drag"),
			)
			ui.View(ui.Flex(1), ui.AppRegion("drag"))
			ui.Button(
				func() {
					icon("plus", 13, m.color(func(t theme) string { return t.TextSecondary }))
					ui.Text("Agent")
				},
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
		ui.Display("flex"),
		ui.Height(40),
		ui.FlexShrink(0),
		ui.AlignItems("center"),
		ui.Gap(4),
		ui.PaddingLeft(8),
		ui.PaddingRight(8),
		ui.AppRegion("drag"),
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
					terminal.View(terminal.Props{
						Program:          p.Program,
						Args:             p.Arguments,
						WorkingDirectory: directory,
						Environment:      p.Environment,
						Scrollback:       50000,
						Palette:          func() terminal.Palette { return m.theme().TerminalPalette },
						CursorColor:      m.color(func(t theme) string { return t.TerminalCursor }),
						PaddingColor:     "extend",
						FontThicken:      true,
						OnStatus: func(event *native.Event) {
							if status := terminal.StatusFromEvent(event); status != nil {
								p.Status.Write(*status)
							}
						},
						Props: ui.Props{
							Ref:       func(node *native.Node) { m.registerTerminal(p, node) },
							AriaLabel: "Terminal pane",
							OnClick:   func(*native.Event) { m.selectPane(p) },
							Style: ui.Styles(
								ui.Position("absolute"),
								ui.Top(0),
								ui.Right(0),
								ui.Bottom(0),
								ui.Left(0),
								ui.Padding(8),
								ui.BorderWidth(1),
								ui.BorderColor(m.color(func(t theme) string { return t.Terminal })),
								ui.BackgroundColor(m.color(func(t theme) string { return t.Terminal })),
								ui.Color(m.color(func(t theme) string { return t.TerminalText })),
								ui.FontFamily("JetBrainsMono Nerd Font Mono"),
								ui.FontSize(14),
								ui.FontWeight(400),
								ui.LineHeight(20.5),
							),
						},
					})
				},
				ui.Position("relative"),
				ui.Display("flex"),
				ui.Flex(1),
				ui.MinWidth(0),
				ui.MinHeight(0),
			)
		},
		ui.Display("flex"),
		ui.FlexGrow(1),
		ui.FlexBasis(0),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.FlexDirection("column"),
		ui.BackgroundColor(m.color(func(t theme) string { return t.Terminal })),
	)
}
