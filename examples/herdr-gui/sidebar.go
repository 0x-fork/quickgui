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
					ui.View(ui.Style{Flex: 1})
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
				ui.Style{
					Display:      "flex",
					Height:       40,
					FlexShrink:   0,
					AlignItems:   "center",
					PaddingLeft:  78,
					PaddingRight: 8,
					PaddingTop:   2,
					AppRegion:    "drag",
				},
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
					ui.View(ui.Style{
						Width:           "100%",
						Height:          1,
						BackgroundColor: m.color(func(t theme) string { return t.Border }),
					})
				},
				ui.AriaLabel("Resize sidebar sections"),
				ui.Style{
					Display:      "flex",
					Height:       7,
					FlexShrink:   0,
					AlignItems:   "center",
					PaddingLeft:  8,
					PaddingRight: 8,
					Cursor:       "ns-resize",
					AppRegion:    "no-drag",
				},
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
										ui.Style{
											Color:      m.color(func(t theme) string { return t.TextGhost }),
											FontSize:   12,
											LineHeight: 16,
										},
									)
								},
								ui.Style{
									Display:      "flex",
									PaddingLeft:  10,
									PaddingRight: 10,
									PaddingTop:   8,
								},
							)
						},
					)
				})
			})
		},
		ui.Style{
			Display:         "flex",
			FlexDirection:   "column",
			FlexShrink:      0,
			MinWidth:        0,
			MinHeight:       0,
			Width:           m.SidebarWidth.Read,
			BackgroundColor: m.color(func(t theme) string { return t.Sidebar }),
		},
	)
	ui.View(
		func() {
			ui.View(ui.Style{
				Width:           1,
				Height:          "100%",
				BackgroundColor: m.color(func(t theme) string { return t.Border }),
			})
		},
		ui.AriaLabel("Resize sidebar"),
		ui.Style{
			Position:   "relative",
			Display:    "flex",
			Width:      1,
			FlexShrink: 0,
		},
		ui.HitSlopLeft(5),
		ui.Style{
			Cursor:    "ew-resize",
			AppRegion: "no-drag",
		},
		ui.OnPointer(m.handleSidebarPointer),
	)
}
func sidebarSection(grow func() float64, children ui.Component) {
	ui.View(
		children,
		ui.Style{
			Display:       "flex",
			FlexBasis:     0,
			FlexGrow:      grow,
			MinHeight:     0,
			FlexDirection: "column",
			PaddingLeft:   8,
			PaddingRight:  8,
		},
	)
}
func sidebarList(children ui.Component) {
	ui.View(
		children,
		ui.Style{
			Display:       "flex",
			Flex:          1,
			MinHeight:     0,
			FlexDirection: "column",
			Gap:           2,
			PaddingBottom: 8,
			OverflowY:     "auto",
		},
	)
}
func sectionHeader(m *model, label, trailing string, action func()) {
	ui.View(
		func() {
			ui.Text(
				label,
				ui.Style{
					Color:      m.color(func(t theme) string { return t.TextTertiary }),
					FontSize:   12,
					FontWeight: 650,
				},
			)
			ui.View(ui.Style{Flex: 1})
			if trailing != "" {
				ui.Text(
					trailing,
					ui.Style{
						Color:    m.color(func(t theme) string { return t.TextGhost }),
						FontSize: 10.5,
					},
				)
			}
			if action != nil {
				iconButton(m, "Add space", "plus", 20, action, ui.Style{MarginLeft: 5})
			}
		},
		ui.Style{
			Display:      "flex",
			Height:       30,
			FlexShrink:   0,
			AlignItems:   "center",
			PaddingLeft:  10,
			PaddingRight: 6,
		},
	)
}
func twoLineRow(children ui.Component) {
	ui.View(
		children,
		ui.Style{
			Display:        "flex",
			Flex:           1,
			MinWidth:       0,
			FlexDirection:  "column",
			JustifyContent: "center",
			Gap:            1,
		},
	)
}
func rowLine(children ui.Component) {
	ui.View(children, ui.Style{Display: "flex", MinWidth: 0, AlignItems: "center", Gap: 5})
}
func rowTitle(m *model, text any) {
	ui.Text(
		text,
		ui.Style{
			Color:        m.color(func(t theme) string { return t.Text }),
			FontSize:     13.5,
			LineHeight:   16,
			FontWeight:   560,
			LineClamp:    1,
			TextOverflow: "ellipsis",
		},
	)
}
func rowMeta(m *model, text any) {
	ui.Text(
		text,
		ui.Style{
			Color:        m.color(func(t theme) string { return t.TextTertiary }),
			FontSize:     11.5,
			LineHeight:   15,
			LineClamp:    1,
			TextOverflow: "ellipsis",
		},
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
							ui.View(ui.Style{Flex: 1})
							ui.Show(
								func() bool { return len(agents()) > 0 },
								func() {
									ui.Text(
										func() string { return fmt.Sprint(len(agents())) },
										ui.Style{
											Color:      m.color(func(t theme) string { return t.TextGhost }),
											FontSize:   11.5,
											LineHeight: 16,
										},
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
				ui.Style{PaddingRight: func() int { return choose(selected(), 30, 8) }},
				ui.OnClick(func() { m.selectSpace(s) }),
			)
			ui.Show(
				func() bool { return selected() && len(m.Spaces.Read()) > 1 },
				func() {
					iconButton(m, "Remove "+s.Name+" space", "close", 20, func() { m.removeSpace(s) }, ui.Style{
						Position: "absolute",
						Top:      14,
						Right:    5,
					})
				},
			)
		},
		ui.Style{
			Position:     "relative",
			Display:      "flex",
			FlexShrink:   0,
			Height:       48,
			BorderRadius: 6,
		},
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
					ui.View(ui.Style{Flex: 1})
					ui.Text(
						func() string { return statusLabel(p.Status.Read().AgentStatus) },
						ui.Style{
							Color: func() string {
								return choose(p.Status.Read().AgentStatus == "blocked", m.theme().Danger, m.theme().TextGhost)
							},
							FontSize:   11.5,
							LineHeight: 15,
						},
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
