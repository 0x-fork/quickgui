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
	).Position("relative").Display("flex").FlexDirection("row").Width("100%").Height("100%").MinWidth(0).MinHeight(0).BackgroundColor(m.color(func(t theme) string { return t.App })).TextColor(m.color(func(t theme) string { return t.Text })).FontWeight(500)
}
func workspace(m *model) {
	ui.View(
		func() {
			tabBar(m)
			ui.View().Height(1).FlexShrink(0).BackgroundColor(m.color(func(t theme) string { return t.Border }))
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
									).TextColor(m.color(func(t theme) string { return t.TextTertiary })).FontSize(13.5)
									ui.Button(
										"New Tab",
										m.buttonStyle(false),
										ui.OnClick(func() { m.newTerminal(m.activeSpace()) }),
									)
								},
							).Display("flex").Flex(1).MinWidth(0).MinHeight(0).FlexDirection("column").AlignItems("center").JustifyContent("center").Gap(9).BackgroundColor(m.color(func(t theme) string { return t.Terminal }))
						},
					)
				},
			).Position("relative").Display("flex").Flex(1).MinWidth(0).MinHeight(0)
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
							).TextColor(m.color(func(t theme) string { return t.Danger })).FontSize(11.5)
							ui.View().Flex(1)
							ui.Show(
								func() bool {
									return m.Error.Read() == "" && m.activePane() != nil
								},
								func() {
									ui.Button(
										"Restart",
										m.buttonStyle(false),
										ui.Style().Height(22),
										ui.Style().FontSize(10.5),
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
					).Display("flex").MinHeight(30).FlexShrink(0).AlignItems("center").Gap(8).PaddingLeft(12).PaddingRight(8).BackgroundColor(m.color(func(t theme) string { return t.ErrorBackground }))
				},
			)
		},
	).Display("flex").Flex(1).MinWidth(0).MinHeight(0).FlexDirection("column").BackgroundColor(m.color(func(t theme) string { return t.App }))
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
											).FontSize(12.5).FontWeight(560).LineClamp(1).TextOverflow("ellipsis")
										},
										ui.AriaLabel("Select terminal tab"),
										ui.Style().Display("flex"),
										ui.Style().Width("100%"),
										ui.Style().MinWidth(0),
										ui.Style().Height(28),
										ui.Style().AlignItems("center"),
										ui.Style().PaddingLeft(10),
										ui.Style().PaddingRight(func() int { return choose(active(), 30, 10) }),
										ui.Style().BackgroundColor("transparent"),
										ui.Style().
											TextColor(func() string {
												return choose(active(), m.theme().Text, m.theme().TextTertiary)
											}),
										ui.Style().
											Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
												return s.
													BackgroundColor(func() string {
														return choose(active(), m.theme().SelectedStrong, m.theme().Hover)
													})
											}),
										ui.Style().
											Active(func(s ui.StyleBuilder) ui.StyleBuilder {
												return s.BackgroundColor(m.color(func(t theme) string { return t.Active }))
											}),
										ui.Style().BorderRadius(6),
										ui.Style().Cursor("default"),
										ui.Style().AppRegion("no-drag"),
										ui.OnClick(func() { m.selectTab(tab()) }),
									)
									ui.Show(
										active,
										func() {
											iconButton(m, "Close tab", "close", 20, func() { m.closeTab(tab().ID) }, ui.Style().
												Position("absolute").
												Top(4).
												Right(3))
										},
									)
								},
								ui.Style().Position("relative"),
								ui.Style().Display("flex"),
								ui.Style().MinWidth(74),
								ui.Style().MaxWidth(170),
								ui.Style().Height(28),
								ui.Style().FlexShrink(1),
								ui.Style().AlignItems("center"),
								ui.Style().
									BackgroundColor(func() string {
										return choose(active(), m.theme().Selected, "transparent")
									}),
								ui.Style().BorderRadius(6),
								ui.Group("tab"),
							)
						},
						nil,
					)
					iconButton(m, "New tab", "plus", 24, func() { m.newTerminal(m.activeSpace()) })
				},
			).Display("flex").MinWidth(0).AlignItems("center").Gap(3).Overflow("hidden").AppRegion("no-drag")
			ui.View().Flex(1).AppRegion("drag")
			ui.Button(
				func() {
					icon("plus", 13, m.color(func(t theme) string { return t.TextSecondary }))
					ui.Text("Agent")
				},
				ui.AriaLabel("New agent"),
				ui.FocusOnPointer(ptr(false)),
				ui.Style().Display("flex"),
				ui.Style().Height(24),
				ui.Style().FlexShrink(0),
				ui.Style().AlignItems("center"),
				ui.Style().JustifyContent("center"),
				ui.Style().Gap(5),
				ui.Style().PaddingLeft(9),
				ui.Style().PaddingRight(9),
				ui.Style().TextColor(m.color(func(t theme) string { return t.TextSecondary })),
				ui.Style().BackgroundColor("transparent"),
				ui.Style().
					Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
						return s.BackgroundColor(m.color(func(t theme) string { return t.Hover }))
					}),
				ui.Style().
					Active(func(s ui.StyleBuilder) ui.StyleBuilder {
						return s.BackgroundColor(m.color(func(t theme) string { return t.Active }))
					}),
				ui.Style().Transition(colorTransition),
				ui.Style().BorderRadius(5),
				ui.Style().FontSize(11.5),
				ui.Style().FontWeight(600),
				ui.Style().Cursor("default"),
				ui.Style().AppRegion("no-drag"),
				ui.OnClick(m.openAgentSheet),
			)
			ui.View().Width(1).Height(16).FlexShrink(0).MarginLeft(2).MarginRight(2).BackgroundColor(m.color(func(t theme) string { return t.BorderStrong }))
			iconButton(m, "Split right", "columns", 24, func() { m.splitTerminal("horizontal") })
			iconButton(m, "Split down", "rows", 24, func() { m.splitTerminal("vertical") })
		},
	).Display("flex").Height(40).FlexShrink(0).AlignItems("center").Gap(4).PaddingLeft(8).PaddingRight(8).AppRegion("drag")
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
		ui.Style().Position("absolute"),
		ui.Style().Display("flex"),
		ui.Style().Top(0),
		ui.Style().Right(0),
		ui.Style().Bottom(0),
		ui.Style().Left(0),
		ui.Style().MinWidth(0),
		ui.Style().MinHeight(0),
		ui.Style().Gap(5),
		ui.Style().FlexDirection("row"),
		ui.When(
			func() bool { return tab().Direction == "vertical" },
			ui.Style().FlexDirection("column"),
		),
		ui.When(
			func() bool { return tab().ID != m.ActiveTabID.Read() },
			ui.Style().Visibility("hidden"),
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
							Style: ui.Style().
								Position("absolute").
								Top(0).
								Right(0).
								Bottom(0).
								Left(0).
								Padding(8).
								BorderWidth(1).
								BorderColor(m.color(func(t theme) string { return t.Terminal })).
								BackgroundColor(m.color(func(t theme) string { return t.Terminal })).
								TextColor(m.color(func(t theme) string { return t.TerminalText })).
								FontFamily("JetBrainsMono Nerd Font Mono").
								FontSize(14).
								FontWeight(400).
								LineHeight(20.5),
						},
					})
				},
			).Position("relative").Display("flex").Flex(1).MinWidth(0).MinHeight(0)
		},
	).Display("flex").FlexGrow(1).FlexBasis(0).MinWidth(0).MinHeight(0).FlexDirection("column").BackgroundColor(m.color(func(t theme) string { return t.Terminal }))
}
