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
								func() {
									ui.View(
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
										ui.Style{Display: "flex", FlexDirection: "column", Gap: 4},
									)
									ui.View(ui.Style{Flex: 1})
									iconButton(m, "Close new agent", "close", 26, m.closeAgentSheet)
								},
								ui.Style{Display: "flex", AlignItems: "flex-start"},
							)
							formGroup(func() {
								formLabel(m, "Agent")
								ui.View(
									func() {
										ui.KeyedFor(
											m.Launchers.Read,
											func(l launcher) any { return l.ID },
											func(read func() launcher, _ func() int) {
												selected := func() bool { return read().ID == m.SelectedLauncherID.Read() }
												ui.Button(
													func() {
														ui.View(
															func() {
																ui.Text(
																	read().Mark,
																	ui.Style{
																		FontSize:   13,
																		FontWeight: 750,
																	},
																)
															},
															ui.Style{
																Display:         "flex",
																Width:           24,
																Height:          24,
																FlexShrink:      0,
																AlignItems:      "center",
																JustifyContent:  "center",
																BackgroundColor: m.color(func(t theme) string { return t.AccentWash }),
																BorderRadius:    5,
															},
														)
														ui.View(
															func() {
																ui.Text(
																	read().Label,
																	ui.Style{
																		FontSize:   12,
																		FontWeight: 620,
																	},
																)
																ui.Text(
																	func() string {
																		return choose(read().installed(), "Available", "Not found")
																	},
																	ui.Style{
																		Color:    m.color(func(t theme) string { return t.TextGhost }),
																		FontSize: 10,
																	},
																)
															},
															ui.Style{
																Display:       "flex",
																FlexDirection: "column",
																Gap:           2,
															},
														)
													},
													ui.AriaLabel(read().Label),
													ui.Disabled(func() bool { return !read().installed() }),
													ui.OnClick(func() {
														m.SelectedLauncherID.Write(read().ID)
													}),
													ui.Style{
														Display:      "flex",
														Flex:         1,
														MinWidth:     0,
														Height:       54,
														AlignItems:   "center",
														Gap:          8,
														PaddingLeft:  8,
														PaddingRight: 8,
														BackgroundColor: func() string {
															return choose(selected(), m.theme().Selected, "transparent")
														},
														Color: func() string {
															return choose(read().installed(), m.theme().Text, m.theme().TextGhost)
														},
														BorderColor: func() string {
															return choose(selected(), m.theme().Accent, m.theme().Border)
														},
														BorderWidth:  1,
														BorderRadius: 6,
														Hover: &ui.Style{BackgroundColor: func() string {
															return choose(read().installed(), m.theme().Hover, "transparent")
														}},
														Opacity: func() float64 {
															return choose(read().installed(), 1.0, .5)
														},
														Cursor: "default",
													},
												)
											},
											nil,
										)
									},
									ui.Style{Display: "flex", Gap: 8},
								)
								ui.Text(
									func() string {
										if m.CatalogLoading.Read() {
											return "Reading your interactive login-shell PATH…"
										}
										return m.selectedLauncher().Description
									},
									ui.Style{
										Color:    m.color(func(t theme) string { return t.TextGhost }),
										FontSize: 11,
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
									ui.Style{
										Display:         "flex",
										Height:          88,
										Width:           "100%",
										PaddingLeft:     10,
										PaddingRight:    10,
										PaddingTop:      9,
										PaddingBottom:   9,
										BackgroundColor: m.color(func(t theme) string { return t.Terminal }),
										Color:           m.color(func(t theme) string { return t.Text }),
										BorderColor:     m.color(func(t theme) string { return t.BorderStrong }),
										BorderWidth:     1,
										BorderRadius:    6,
										FontSize:        12.5,
										Focus: &ui.Style{
											OutlineWidth: 1,
											OutlineColor: m.color(func(t theme) string { return t.Accent }),
										},
									},
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
										func() {
											ui.Text(
												"No supported agent CLI was found. You can still open a terminal and run any installed agent; the sidebar detects it automatically.",
												ui.Style{
													Color:      m.color(func(t theme) string { return t.Warning }),
													FontSize:   11.5,
													LineHeight: 16,
												},
											)
										},
										ui.Style{
											Display:         "flex",
											MinHeight:       36,
											AlignItems:      "center",
											PaddingLeft:     10,
											PaddingRight:    10,
											PaddingTop:      7,
											PaddingBottom:   7,
											BackgroundColor: m.color(func(t theme) string { return t.AccentWash }),
											BorderRadius:    5,
										},
									)
								},
							)
							ui.View(
								func() {
									ui.Button(
										"Cancel",
										m.buttonStyle(false),
										ui.OnClick(m.closeAgentSheet),
									)
									disabled := func() bool { return m.CatalogLoading.Read() || !m.selectedLauncher().installed() }
									ui.Button(
										func() string {
											return "Start " + m.selectedLauncher().Label
										},
										m.buttonStyle(true),
										ui.Disabled(disabled),
										ui.Style{Opacity: func() float64 { return choose(disabled(), .45, 1.0) }},
										ui.OnClick(m.launchAgent),
									)
								},
								ui.Style{Display: "flex", JustifyContent: "flex-end", Gap: 8},
							)
						},
					)
				},
			)
		},
	)
}
func formGroup(children ui.Component) {
	ui.View(children, ui.Style{Display: "flex", FlexDirection: "column", Gap: 7})
}
func formLabel(m *model, label string) {
	ui.Text(
		label,
		ui.Style{
			Color:      m.color(func(t theme) string { return t.TextSecondary }),
			FontSize:   11.5,
			FontWeight: 620,
		},
	)
}
