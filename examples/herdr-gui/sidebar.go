package main

import (
	"fmt"

	"github.com/egoist/quickgui/go/ui"
)

func sidebar(m *model) {
	ui.View(
		func() {
			ui.View(
				func() {
					ui.View(ui.Flex(1))
					ui.Button(
						func() {
							dynamicIcon(func() string { return choose(m.Appearance.Read() == "dark", "sun", "moon") }, 14, m.color(func(t theme) string { return t.TextTertiary }))
						},
						ui.AriaLabel("Toggle light or dark appearance"),
						ui.FocusOnPointer(ptr(false)),
						m.iconStyle(24),
						ui.OnClick(m.toggleTheme),
					)
				},
				ui.Display("flex"),
				ui.Height(40),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.PaddingLeft(78),
				ui.PaddingRight(8),
				ui.PaddingTop(2),
				ui.AppRegion("drag"),
			)
			sidebarSection(m.SectionRatio.Read, func() {
				sectionHeader(m, "Spaces", "", func() { m.addSpace() })
				sidebarList(func() {
					ui.For(
						m.Spaces.Read,
						func(s space, _ func() int) { spaceRow(m, s) },
						func(s space) any { return s.ID },
						nil,
					)
				})
			})
			ui.View(
				func() {
					ui.View(
						ui.Width("100%"),
						ui.Height(1),
						ui.BackgroundColor(m.color(func(t theme) string { return t.Border })),
					)
				},
				ui.AriaLabel("Resize sidebar sections"),
				ui.Display("flex"),
				ui.Height(7),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.PaddingLeft(8),
				ui.PaddingRight(8),
				ui.Cursor("ns-resize"),
				ui.AppRegion("no-drag"),
				ui.OnPointer(m.handleSectionPointer),
			)
			sidebarSection(func() float64 { return 1 - m.SectionRatio.Read() }, func() {
				sectionHeader(m, "Agents", "grouped", nil)
				sidebarList(func() {
					ui.For(
						m.visibleAgents,
						func(p *pane, _ func() int) { agentRow(m, p) },
						func(p *pane) any { return p },
						func() {
							ui.View(
								func() {
									ui.Text(
										"Agents appear here when detected in a pane.",
										ui.TextColor(m.color(func(t theme) string { return t.TextGhost })),
										ui.FontSize(12),
										ui.LineHeight(16),
									)
								},
								ui.Display("flex"),
								ui.PaddingLeft(10),
								ui.PaddingRight(10),
								ui.PaddingTop(8),
							)
						},
					)
				})
			})
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.FlexShrink(0),
		ui.MinWidth(0),
		ui.MinHeight(0),
		ui.Width(m.SidebarWidth.Read),
		ui.BackgroundColor(m.color(func(t theme) string { return t.Sidebar })),
	)
	ui.View(
		func() {
			ui.View(
				ui.Width(1),
				ui.Height("100%"),
				ui.BackgroundColor(m.color(func(t theme) string { return t.Border })),
			)
		},
		ui.AriaLabel("Resize sidebar"),
		ui.Position("relative"),
		ui.Display("flex"),
		ui.Width(1),
		ui.FlexShrink(0),
		ui.HitSlopLeft(5),
		ui.Cursor("ew-resize"),
		ui.AppRegion("no-drag"),
		ui.OnPointer(m.handleSidebarPointer),
	)
}
func sidebarSection(grow func() float64, children ui.Component) {
	ui.View(
		children,
		ui.Display("flex"),
		ui.FlexBasis(0),
		ui.FlexGrow(grow),
		ui.MinHeight(0),
		ui.FlexDirection("column"),
		ui.PaddingLeft(8),
		ui.PaddingRight(8),
	)
}
func sidebarList(children ui.Component) {
	ui.View(
		children,
		ui.Display("flex"),
		ui.Flex(1),
		ui.MinHeight(0),
		ui.FlexDirection("column"),
		ui.Gap(2),
		ui.PaddingBottom(8),
		ui.OverflowY("auto"),
	)
}
func sectionHeader(m *model, label, trailing string, action func()) {
	ui.View(
		func() {
			ui.Text(
				label,
				ui.TextColor(m.color(func(t theme) string { return t.TextTertiary })),
				ui.FontSize(12),
				ui.FontWeight(650),
			)
			ui.View(ui.Flex(1))
			if trailing != "" {
				ui.Text(
					trailing,
					ui.TextColor(m.color(func(t theme) string { return t.TextGhost })),
					ui.FontSize(10.5),
				)
			}
			if action != nil {
				iconButton(m, "Add space", "plus", 20, action, ui.Styles(ui.MarginLeft(5)))
			}
		},
		ui.Display("flex"),
		ui.Height(30),
		ui.FlexShrink(0),
		ui.AlignItems("center"),
		ui.PaddingLeft(10),
		ui.PaddingRight(6),
	)
}
func twoLineRow(children ui.Component) {
	ui.View(
		children,
		ui.Display("flex"),
		ui.Flex(1),
		ui.MinWidth(0),
		ui.FlexDirection("column"),
		ui.JustifyContent("center"),
		ui.Gap(1),
	)
}
func rowLine(children ui.Component) {
	ui.View(children, ui.Display("flex"), ui.MinWidth(0), ui.AlignItems("center"), ui.Gap(5))
}
func rowTitle(m *model, text any) {
	ui.Text(
		text,
		ui.TextColor(m.color(func(t theme) string { return t.Text })),
		ui.FontSize(13.5),
		ui.LineHeight(16),
		ui.FontWeight(560),
		ui.LineClamp(1),
		ui.TextOverflow("ellipsis"),
	)
}
func rowMeta(m *model, text any) {
	ui.Text(
		text,
		ui.TextColor(m.color(func(t theme) string { return t.TextTertiary })),
		ui.FontSize(11.5),
		ui.LineHeight(15),
		ui.LineClamp(1),
		ui.TextOverflow("ellipsis"),
	)
}
func spaceRow(m *model, s space) {
	selected := func() bool { return m.ActiveSpaceID.Read() == s.ID }
	agents := func() []*pane {
		var out []*pane
		for _, p := range m.Panes.Read() {
			if p.SpaceID == s.ID && p.Status.Read().Agent != "" {
				out = append(out, p)
			}
		}
		return out
	}
	ui.View(
		func() {
			ui.Button(
				func() {
					statusGlyph(m, func() string { return aggregateStatus(agents()) }, true)
					twoLineRow(func() {
						rowLine(func() {
							rowTitle(m, s.Name)
							ui.View(ui.Flex(1))
							ui.Show(
								func() bool { return len(agents()) > 0 },
								func() {
									ui.Text(
										func() string { return fmt.Sprint(len(agents())) },
										ui.TextColor(m.color(func(t theme) string { return t.TextGhost })),
										ui.FontSize(11.5),
										ui.LineHeight(16),
									)
								},
							)
						})
						rowMeta(m, shortPath(s.Path, m.Home))
					})
				},
				ui.AriaLabel("Open "+s.Name+" space"),
				ui.FocusOnPointer(ptr(false)),
				m.rowStyle(selected),
				ui.PaddingRight(func() int { return choose(selected(), 30, 8) }),
				ui.OnClick(func() { m.selectSpace(s) }),
			)
			ui.Show(
				func() bool { return selected() && len(m.Spaces.Read()) > 1 },
				func() {
					iconButton(m, "Remove "+s.Name+" space", "close", 20, func() { m.removeSpace(s) }, ui.Styles(
						ui.Position("absolute"),
						ui.Top(14),
						ui.Right(5),
					))
				},
			)
		},
		ui.Position("relative"),
		ui.Display("flex"),
		ui.FlexShrink(0),
		ui.Height(48),
		ui.BorderRadius(6),
	)
}
func agentRow(m *model, p *pane) {
	ui.Button(
		func() {
			statusGlyph(m, func() string { return p.Status.Read().AgentStatus }, false)
			twoLineRow(func() {
				rowLine(func() {
					rowTitle(m, func() string {
						for _, s := range m.Spaces.Read() {
							if s.ID == p.SpaceID {
								return s.Name
							}
						}
						return ""
					})
					rowMeta(m, "·")
					rowMeta(m, func() string {
						if t := m.tab(p.TabID); t != nil {
							return tabTitle(*t, m.Panes.Read())
						}
						return "Terminal"
					})
				})
				rowLine(func() {
					rowMeta(m, func() string { return agentLabel(p.Status.Read().Agent) })
					ui.View(ui.Flex(1))
					ui.Text(
						func() string { return statusLabel(p.Status.Read().AgentStatus) },
						ui.TextColor(func() string {
							return choose(p.Status.Read().AgentStatus == "blocked", m.theme().Danger, m.theme().TextGhost)
						}),
						ui.FontSize(11.5),
						ui.LineHeight(15),
					)
				})
			})
		},
		ui.AriaLabel("Open detected agent"),
		ui.FocusOnPointer(ptr(false)),
		m.rowStyle(func() bool { return m.ActivePaneID.Read() == p.ID }),
		ui.OnClick(func() { m.selectPane(p) }),
	)
}
