package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/ui"
)

func agentSheet(m *model) {
	ui.Dialog.Root(
		ui.DialogRootProps{
			Open: m.SheetOpen.Read,
			OnOpenChange: func(open bool, _ ui.DialogOpenChangeDetails) {
				if !open {
					m.closeAgentSheet()
				}
			},
		},
		func() {
			ui.Dialog.Portal(
				ui.PartProps{Style: ui.Style{
					Position:       "absolute",
					Top:            0,
					Right:          0,
					Bottom:         0,
					Left:           0,
					Display:        "flex",
					AlignItems:     "center",
					JustifyContent: "center",
					Padding:        24,
				}},
				func() {
					ui.Dialog.Backdrop(ui.PartProps{Style: ui.Style{
						Position:        "absolute",
						Top:             0,
						Right:           0,
						Bottom:          0,
						Left:            0,
						BackgroundColor: m.color(func(t theme) string { return t.Scrim }),
					}})
					ui.Dialog.Popup(
						ui.DialogPopupProps{PartProps: ui.PartProps{
							AriaLabel: "New agent",
							Style: ui.Style{
								Display:         "flex",
								Width:           472,
								MaxWidth:        "100%",
								FlexDirection:   "column",
								Gap:             17,
								Padding:         20,
								BackgroundColor: m.color(func(t theme) string { return t.Surface }),
								BorderColor:     m.color(func(t theme) string { return t.BorderStrong }),
								BorderWidth:     1,
								BorderRadius:    9,
							},
						}},
						func() {
							ui.View(
								ui.Display("flex"),
								ui.AlignItems("flex-start"),
								func() {
									ui.View(
										ui.Display("flex"),
										ui.FlexDirection("column"),
										ui.Gap(4),
										func() {
											ui.Dialog.Title(
												ui.PartProps{Style: ui.Style{
													Color:      m.color(func(t theme) string { return t.Text }),
													FontSize:   17,
													FontWeight: 720,
												}},
												"New agent",
											)
											ui.Dialog.Description(
												ui.PartProps{Style: ui.Style{
													Color:    m.color(func(t theme) string { return t.TextTertiary }),
													FontSize: 12,
												}},
												func() {
													ui.Text(func() string {
														return "Start a real CLI in " + m.activeSpace().Name
													})
												},
											)
										},
									)
									ui.View(ui.Flex(1))
									iconButton(m, "Close new agent", "close", 26, m.closeAgentSheet)
								},
							)
							formGroup(func() {
								formLabel(m, "Agent")
								ui.View(
									ui.Display("flex"),
									ui.Gap(8),
									func() {
										ui.KeyedFor(
											m.Launchers.Read,
											func(l launcher) any { return l.ID },
											func(read func() launcher, _ func() int) {
												selected := func() bool { return read().ID == m.SelectedLauncherID.Read() }
												ui.Button(
													ui.AriaLabel(read().Label),
													ui.Disabled(func() bool { return !read().installed() }),
													ui.OnClick(func() {
														m.SelectedLauncherID.Write(read().ID)
													}),
													ui.Display("flex"),
													ui.Flex(1),
													ui.MinWidth(0),
													ui.Height(54),
													ui.AlignItems("center"),
													ui.Gap(8),
													ui.PaddingLeft(8),
													ui.PaddingRight(8),
													ui.BackgroundColor(func() string {
														return choose(selected(), m.theme().Selected, "transparent")
													}),
													ui.Color(func() string {
														return choose(read().installed(), m.theme().Text, m.theme().TextGhost)
													}),
													ui.BorderColor(func() string {
														return choose(selected(), m.theme().Accent, m.theme().Border)
													}),
													ui.BorderWidth(1),
													ui.BorderRadius(6),
													ui.Hover(ui.BackgroundColor(func() string {
														return choose(read().installed(), m.theme().Hover, "transparent")
													})),
													ui.Opacity(func() float64 {
														return choose(read().installed(), 1.0, .5)
													}),
													ui.Cursor("default"),
													func() {
														ui.View(
															ui.Display("flex"),
															ui.Width(24),
															ui.Height(24),
															ui.FlexShrink(0),
															ui.AlignItems("center"),
															ui.JustifyContent("center"),
															ui.BackgroundColor(m.color(func(t theme) string { return t.AccentWash })),
															ui.BorderRadius(5),
															func() {
																ui.Text(
																	ui.FontSize(13),
																	ui.FontWeight(750),
																	read().Mark,
																)
															},
														)
														ui.View(
															ui.Display("flex"),
															ui.FlexDirection("column"),
															ui.Gap(2),
															func() {
																ui.Text(
																	ui.FontSize(12),
																	ui.FontWeight(620),
																	read().Label,
																)
																ui.Text(
																	ui.Color(m.color(func(t theme) string { return t.TextGhost })),
																	ui.FontSize(10),
																	func() string {
																		return choose(read().installed(), "Available", "Not found")
																	},
																)
															},
														)
													},
												)
											},
											nil,
										)
									},
								)
								ui.Text(
									ui.Color(m.color(func(t theme) string { return t.TextGhost })),
									ui.FontSize(11),
									func() string {
										if m.CatalogLoading.Read() {
											return "Reading your interactive login-shell PATH…"
										}
										return m.selectedLauncher().Description
									},
								)
							})
							formGroup(func() {
								formLabel(m, "Initial instruction · optional")
								ui.TextArea(
									ui.AriaLabel("Initial instruction"),
									ui.Value(m.Prompt.Read),
									ui.Placeholder("What should this agent work on?"),
									ui.OnInput(func(e *native.Event) { m.Prompt.Write(ui.InputValue(e)) }),
									ui.Ref(func(node *native.Node) {
										native.SetBoolean(
											node,
											protocol.AutoFocus,
											true,
										)
									}),
									ui.Display("flex"),
									ui.Height(88),
									ui.Width("100%"),
									ui.PaddingLeft(10),
									ui.PaddingRight(10),
									ui.PaddingTop(9),
									ui.PaddingBottom(9),
									ui.BackgroundColor(m.color(func(t theme) string { return t.Terminal })),
									ui.Color(m.color(func(t theme) string { return t.Text })),
									ui.BorderColor(m.color(func(t theme) string { return t.BorderStrong })),
									ui.BorderWidth(1),
									ui.BorderRadius(6),
									ui.FontSize(12.5),
									ui.Focus(
										ui.OutlineWidth(1),
										ui.OutlineColor(m.color(func(t theme) string { return t.Accent })),
									),
								)
							})
							ui.Show(
								func() bool {
									if m.CatalogLoading.Read() {
										return false
									}
									for _, l := range m.Launchers.Read() {
										if l.installed() {
											return false
										}
									}
									return true
								},
								func() {
									ui.View(
										ui.Display("flex"),
										ui.MinHeight(36),
										ui.AlignItems("center"),
										ui.PaddingLeft(10),
										ui.PaddingRight(10),
										ui.PaddingTop(7),
										ui.PaddingBottom(7),
										ui.BackgroundColor(m.color(func(t theme) string { return t.AccentWash })),
										ui.BorderRadius(5),
										func() {
											ui.Text(
												ui.Color(m.color(func(t theme) string { return t.Warning })),
												ui.FontSize(11.5),
												ui.LineHeight(16),
												"No supported agent CLI was found. You can still open a terminal and run any installed agent; the sidebar detects it automatically.",
											)
										},
									)
								},
							)
							ui.View(
								ui.Display("flex"),
								ui.JustifyContent("flex-end"),
								ui.Gap(8),
								func() {
									ui.Button(
										ui.WithStyle(m.buttonStyle(false)),
										ui.OnClick(m.closeAgentSheet),
										"Cancel",
									)
									disabled := func() bool { return m.CatalogLoading.Read() || !m.selectedLauncher().installed() }
									ui.Button(
										ui.WithStyle(m.buttonStyle(true)),
										ui.Disabled(disabled),
										ui.Opacity(func() float64 { return choose(disabled(), .45, 1.0) }),
										ui.OnClick(m.launchAgent),
										func() string {
											return "Start " + m.selectedLauncher().Label
										},
									)
								},
							)
						},
					)
				},
			)
		},
	)
}
func formGroup(children ui.Component) {
	ui.View(ui.Display("flex"), ui.FlexDirection("column"), ui.Gap(7), children)
}
func formLabel(m *model, label string) {
	ui.Text(
		ui.Color(m.color(func(t theme) string { return t.TextSecondary })),
		ui.FontSize(11.5),
		ui.FontWeight(620),
		label,
	)
}
