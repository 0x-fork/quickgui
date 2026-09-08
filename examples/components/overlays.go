package main

import (
	"strconv"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func menuAppearance() *ui.MenuAppearance {
	return &ui.MenuAppearance{
		Width:               ptr(220.0),
		ItemHeight:          ptr(28.0),
		FontSize:            ptr(12.0),
		Radius:              ptr(10.0),
		Background:          p().Popup,
		Color:               p().Ink,
		HighlightBackground: p().Accent,
		HighlightColor:      p().OnAccent,
		MutedColor:          p().Muted,
	}
}
func menuRow() ui.PartProps {
	return ui.PartProps{Style: ui.Style().
		Merge(controlStyle()).
		JustifyContent("space-between").
		BackgroundColor("transparent").
		BorderWidth(0).
		Height(28).
		BorderRadius(5).
		PaddingLeft(10).
		PaddingRight(10).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(color(func(p palette) string { return p.Selection }))
		})}
}
func menuLabel(caption string) {
	state := ui.UseMenuItemState()
	ui.Text(
		caption,
	).TextColor(func() string { return choose(state().Highlighted, p().Accent, p().Ink) }).Flex(1)
}
func ContextMenuDemo() {
	command, setCommand := ui.CreateSignal("nothing yet")
	parts, setParts := ui.CreateSignal("nothing yet")
	target := func() ui.ContextMenuTriggerProps {
		return ui.ContextMenuTriggerProps{PartProps: ui.PartProps{Style: ui.Style().
			Display("flex").
			Height(64).
			BorderRadius(10).
			BorderWidth(1).
			BorderColor(color(func(p palette) string { return p.Border })).
			BorderStyle("dashed").
			AlignItems("center").
			JustifyContent("center").
			BackgroundColor(color(func(p palette) string { return p.PanelAlt }))}}
	}
	panel("Context menu", "Secondary-click menus from a declared row model or from Menu.Item parts.", func() {
		ui.ContextMenu.Root(
			ui.ContextMenuRootProps{
				Items: func() []ui.MenuItemDeclaration {
					return []ui.MenuItemDeclaration{{ID: "cut", Label: "Cut"}, {ID: "copy", Label: "Copy"}, {Type: "separator"}, {ID: "paste", Label: "Paste"}}
				},
				Appearance: menuAppearance(),
				OnSelect:   func(d ui.MenuSelectDetails, _ *native.Event) { setCommand(d.ID) },
			},
			func() {
				ui.ContextMenu.Trigger(
					target(),
					"Right-click: declared rows",
				)
			},
		)
		ui.ContextMenu.Root(
			ui.ContextMenuRootProps{
				Appearance: menuAppearance(),
				OnSelect:   func(d ui.MenuSelectDetails, _ *native.Event) { setParts(d.ID) },
			},
			func() {
				ui.ContextMenu.Trigger(
					target(),
					func() {
						ui.Menu.Item(ui.MenuItemProps{Value: "rename", Label: "Rename"})
						ui.Menu.Item(ui.MenuItemProps{Value: "duplicate", Label: "Duplicate"})
						ui.Menu.Separator(ui.PartProps{})
						ui.Menu.Item(ui.MenuItemProps{Value: "delete", Label: "Delete"})
						muted("Right-click: Menu.Item parts")
					},
				)
			},
		)
		note(func() string { return "rows → " + command() + " · parts → " + parts() })
	})
}
func SystemContextMenuDemo() {
	action, setAction := ui.CreateSignal("nothing yet")
	status, setStatus := ui.CreateSignal("closed")
	showHidden, setShowHidden := ui.CreateSignal(false)
	sortBy, setSortBy := ui.CreateSignal("Name")
	open := func() {
		window := native.CurrentWindow()
		sortItems := make([]native.MenuItem, 0, 3)
		for _, name := range []string{"Name", "Date modified", "Size"} {
			sortItems = append(sortItems, native.MenuItem{
				Label:   name,
				Checked: sortBy() == name,
				Click: func() {
					setSortBy(name)
					setAction("Sort by " + name)
				},
			})
		}
		setStatus("open")
		window.PopupMenu(
			[]native.MenuItem{
				{Label: "Open", Click: func() { setAction("Open") }},
				{Label: "Rename", Click: func() { setAction("Rename") }},
				{Type: "separator"},
				{
					Label:   "Show hidden files",
					Checked: showHidden(),
					Click: func() {
						setShowHidden(!showHidden())
						setAction("Show hidden files " + choose(showHidden(), "on", "off"))
					},
				},
				{Label: "Sort by", Items: sortItems},
				{Type: "separator"},
				{Label: "Unavailable action", Enabled: ptr(false)},
			},
			nil,
			func(err error) {
				if err != nil {
					setStatus("error: " + err.Error())
					return
				}
				setStatus("closed")
			},
		)
	}
	panel("System context menu", "The operating system draws this menu. Actions update the readout below; reopen it to see the current checkmarks.", func() {
		ui.View(
			"Right-click here for the system menu",
			ui.Style().Display("flex"),
			ui.Style().Height(96),
			ui.Style().AlignItems("center"),
			ui.Style().JustifyContent("center"),
			ui.Style().BorderRadius(10),
			ui.Style().BorderWidth(1),
			ui.Style().BorderStyle("dashed"),
			ui.Style().BorderColor(color(func(p palette) string { return p.Border })),
			ui.Style().BackgroundColor(color(func(p palette) string { return p.PanelAlt })),
			ui.Style().TextColor(color(func(p palette) string { return p.Muted })),
			ui.Style().AppRegion("no-drag"),
			ui.Style().UserSelect("none"),
			ui.OnContextMenu(func(event *native.Event) {
				event.PreventDefault()
				open()
			}),
		)
		row(func() { button("Open system menu", open) })
		note(func() string { return "menu " + status() + " · last action " + action() })
		note(func() string { return "hidden files " + strconv.FormatBool(showHidden()) + " · sort by " + sortBy() })
	})
}

func MenuDemo() {
	open, setOpen := ui.CreateSignal(false)
	wrap, setWrap := ui.CreateSignal(false)
	density, setDensity := ui.CreateSignal("cozy")
	activated, setActivated := ui.CreateSignal("nothing yet")
	item := func(value, caption string) {
		props := menuRow()
		props.OnClick = func(*native.Event) { setActivated(value) }
		ui.Menu.Item(
			ui.MenuItemProps{Value: value, Label: caption, PartProps: props},
			func() { menuLabel(caption) },
		)
	}
	panel("Menu", "Styled rows with keyboard navigation, checkbox state, radio exclusivity, and a nested submenu.", func() {
		ui.Menu.Root(
			ui.MenuRootProps{
				Open:         open,
				OnOpenChange: change(setOpen),
				Side:         "bottom",
				Align:        "start",
				SideOffset:   ptr(6.0),
			},
			func() {
				ui.Menu.Trigger(ui.MenuTriggerProps{PartProps: control()}, "Edit ▾")
				ui.Menu.Positioner(
					ui.MenuPositionerProps{},
					func() {
						ui.Menu.Popup(
							popup(250),
							func() {
								ui.Menu.GroupLabel(
									ui.MenuGroupLabelProps{
										Value: "clipboard",
										Label: "Clipboard",
									},
									"Clipboard",
								)
								item("copy", "Copy")
								item("cut", "Cut")
								ui.Menu.LinkItem(
									ui.MenuLinkItemProps{
										MenuItemProps: ui.MenuItemProps{
											Value:     "docs",
											Label:     "Documentation",
											PartProps: menuRow(),
										},
										Href:       "https://quickgui.dev",
										OnNavigate: func(href string, _ *native.Event) { setActivated("link " + href) },
									},
									func() { menuLabel("Documentation") },
								)
								ui.Menu.Separator(ui.PartProps{Style: ui.Style().
									Height(1).
									BackgroundColor(color(func(p palette) string { return p.Border }))})
								ui.Menu.CheckboxItem(
									ui.MenuCheckboxItemProps{
										MenuItemProps: ui.MenuItemProps{
											Value:     "wrap",
											Label:     "Wrap lines",
											PartProps: menuRow(),
										},
										Checked:         wrap,
										OnCheckedChange: change(setWrap),
									},
									func() {
										menuLabel("Wrap lines")
										ui.Menu.CheckboxItemIndicator(
											ui.PartProps{},
											"✓",
										)
									},
								)
								ui.Menu.RadioGroup(
									ui.MenuRadioGroupProps{
										Name:          "density",
										Value:         func() *string { return ptr(density()) },
										OnValueChange: change(setDensity),
										PartProps:     columnPart(),
									},
									func() {
										for _, value := range []string{"compact", "cozy", "comfortable"} {
											ui.Menu.RadioItem(
												ui.MenuRadioItemProps{MenuItemProps: ui.MenuItemProps{
													Value:     value,
													Label:     value,
													PartProps: menuRow(),
												}},
												func() {
													menuLabel(value)
													ui.Menu.RadioItemIndicator(
														ui.PartProps{},
														"●",
													)
												},
											)
										}
									},
								)
								ui.Menu.SubmenuRoot(
									ui.MenuRootProps{CloseDelay: 100},
									func() {
										ui.Menu.SubmenuTrigger(
											ui.MenuSubmenuTriggerProps{
												Value: "recent",
												Label: "Open recent",
												MenuTriggerProps: ui.MenuTriggerProps{
													PartProps:   menuRow(),
													OpenOnHover: ptr(true),
												},
											},
											func() { menuLabel("Open recent"); label("›") },
										)
										ui.Menu.Positioner(
											ui.MenuPositionerProps{
												Side:       "right",
												Align:      "start",
												SideOffset: ptr(4.0),
											},
											func() {
												ui.Menu.Popup(
													popup(200),
													func() {
														item("notes", "notes.md")
														item("readme", "README.md")
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
			},
		)
		note(func() string {
			return "open " + strconv.FormatBool(open()) + " · activated " + activated() + " · wrap " + strconv.FormatBool(wrap()) + " · density " + density()
		})
	})
}
func MenubarDemo() {
	open, setOpen := ui.CreateSignal[*int](nil)
	active, setActive := ui.CreateSignal(0)
	command, setCommand := ui.CreateSignal("nothing yet")
	menus := []struct {
		label string
		items []ui.MenuItemDeclaration
	}{
		{"File", []ui.MenuItemDeclaration{{ID: "new", Label: "New window", Shortcut: "⌘N"}, {ID: "open", Label: "Open…", Shortcut: "⌘O"}, {Type: "separator"}, {ID: "close", Label: "Close", Shortcut: "⌘W"}}},
		{"Edit", []ui.MenuItemDeclaration{{ID: "undo", Label: "Undo", Shortcut: "⌘Z"}, {ID: "redo", Label: "Redo", Shortcut: "⇧⌘Z"}, {Type: "separator"}, {ID: "paste", Label: "Paste", Shortcut: "⌘V"}}},
		{"View", []ui.MenuItemDeclaration{{ID: "zoom-in", Label: "Zoom in"}, {ID: "zoom-out", Label: "Zoom out"}}},
	}
	panel("Menubar", "One roving Tab stop. Arrow keys switch menus while open; Escape closes without leaving the bar.", func() {
		ui.Menubar.Root(
			ui.MenubarRootProps{
				Count:          len(menus),
				Open:           open,
				OnOpenChange:   change(setOpen),
				OnActiveChange: change(setActive),
				PartProps:      rowPart(),
			},
			func() {
				for index, menu := range menus {
					ui.PopoverMenu.Root(
						ui.PopoverMenuRootProps{
							Items:      func() []ui.MenuItemDeclaration { return menu.items },
							Appearance: menuAppearance(),
							Open:       func() bool { return open() != nil && *open() == index },
							OnOpenChange: func(v bool, _ ui.PopoverMenuOpenChangeDetails) {
								if v {
									setOpen(ptr(index))
								} else {
									setOpen(nil)
								}
							},
							OnSelect: func(d ui.MenuSelectDetails, _ *native.Event) { setCommand(d.ID) },
						},
						func() {
							ui.PopoverMenu.Trigger(
								ui.PartProps{},
								func() {
									ui.Menubar.Item(
										ui.MenubarItemProps{
											Index:     ptr(index),
											PartProps: togglePart(func() bool { return open() != nil && *open() == index }),
										},
										menu.label,
									)
								},
							)
							ui.PopoverMenu.Popup(ui.PopoverMenuPopupProps{PartProps: popup(220)})
						},
					)
				}
			},
		)
		note(func() string {
			return "open " + textValue(open()) + " · tab stop " + strconv.Itoa(active()) + " · command " + command()
		})
	})
}
func NavigationMenuDemo() {
	value, setValue := ui.CreateSignal[*string](nil)
	direction, setDirection := ui.CreateSignal[*string](nil)
	items := []ui.ComponentItem{{Value: "products"}, {Value: "solutions"}, {Value: "support", Disabled: true}}
	panel("Navigation menu", "Hover opens a panel after the declared delay. The navigation landmark has one Tab stop and reports activation direction.", func() {
		ui.NavigationMenu.Root(
			ui.NavigationMenuRootProps{
				Value:                       value,
				OnValueChange:               change(setValue),
				OnActivationDirectionChange: change(setDirection),
				Delay:                       80,
				CloseDelay:                  80,
				Items:                       items,
			},
			func() {
				ui.NavigationMenu.List(
					rowPart(),
					func() {
						for _, item := range items {
							ui.NavigationMenu.Item(
								ui.NavigationMenuItemProps{Value: item.Value},
								func() {
									props := togglePart(func() bool { return value() != nil && *value() == item.Value })
									props.Disabled = item.Disabled
									ui.NavigationMenu.Trigger(
										ui.NavigationMenuPartProps{PartProps: props},
										item.Value,
									)
									ui.NavigationMenu.Positioner(
										ui.NavigationMenuPartProps{},
										func() {
											ui.NavigationMenu.Popup(
												ui.NavigationMenuPartProps{PartProps: popup(230)},
												func() {
													ui.NavigationMenu.Viewport(
														ui.NavigationMenuPartProps{},
														func() {
															ui.NavigationMenu.Content(
																ui.NavigationMenuPartProps{PartProps: columnPart()},
																func() {
																	ui.NavigationMenu.Link(
																		ui.NavigationMenuLinkProps{
																			Value:  item.Value + "-overview",
																			Active: true,
																		},
																		item.Value+" overview",
																	)
																	ui.NavigationMenu.Link(
																		ui.NavigationMenuLinkProps{Value: item.Value + "-pricing"},
																		item.Value+" pricing",
																	)
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
					},
				)
			},
		)
		note(func() string { return "open panel " + textValue(value()) + " · direction " + textValue(direction()) })
	})
}
func PopoverDemo() {
	open, setOpen := ui.CreateSignal(false)
	hover, setHover := ui.CreateSignal(false)
	panel("Popover", "Preferred placement adapts to the available space. Compare click and delayed hover triggers.", func() {
		ui.Popover.Root(
			ui.PopoverRootProps{
				Open:             open,
				OnOpenChange:     func(v bool, _ ui.PopoverOpenChangeDetails) { setOpen(v) },
				Side:             "bottom",
				Align:            "start",
				SideOffset:       ptr(8.0),
				CollisionPadding: ptr(12.0),
			},
			func() {
				ui.Popover.Trigger(ui.PopoverTriggerProps{PartProps: control()}, "Account")
				placement := ui.UsePopoverPlacement()
				note(func() string { return "resolved " + placement().Side + "/" + placement().Align })
				ui.Popover.Positioner(
					ui.PopoverPositionerProps{},
					func() {
						ui.Popover.Popup(
							ui.PopoverPopupProps{PartProps: popup(240)},
							func() {
								ui.Popover.Arrow(ui.PartProps{Style: ui.Style().
									Width(10).
									Height(10).
									BackgroundColor(color(func(p palette) string { return p.Popup }))})
								ui.Popover.Title(ui.PartProps{}, "Signed in")
								ui.Popover.Viewport(ui.PartProps{}, "ada@example.com")
								ui.Popover.Close(control(), "Done")
							},
						)
					},
				)
			},
		)
		ui.Popover.Root(
			ui.PopoverRootProps{
				OpenOnHover:  ptr(true),
				Delay:        200,
				CloseDelay:   120,
				Side:         "right",
				Align:        "center",
				SideOffset:   ptr(8.0),
				OnOpenChange: func(v bool, _ ui.PopoverOpenChangeDetails) { setHover(v) },
			},
			func() {
				ui.Popover.Trigger(ui.PopoverTriggerProps{PartProps: control()}, "Hover to open")
				ui.Popover.Positioner(
					ui.PopoverPositionerProps{},
					func() {
						ui.Popover.Popup(
							ui.PopoverPopupProps{PartProps: popup(200)},
							"Opened after the hover delay.",
						)
					},
				)
			},
		)
		note(func() string {
			return "click " + strconv.FormatBool(open()) + " · hover " + strconv.FormatBool(hover())
		})
	})
}
func PreviewCardDemo() {
	open, setOpen := ui.CreateSignal(false)
	panel("Preview card", "Rest the pointer to preview. Keyboard focus opens immediately.", func() {
		row(func() {
			muted("Written by")
			ui.PreviewCard.Root(
				ui.PreviewCardRootProps{
					Open:         open,
					OnOpenChange: change(setOpen),
					Placement:    "bottom-start",
					Gap:          8,
				},
				func() {
					ui.PreviewCard.Trigger(
						ui.PreviewCardTriggerProps{
							Delay:      350,
							CloseDelay: 200,
							PartProps: ui.PartProps{Style: ui.Style().
								Padding(2).
								TextColor(color(func(p palette) string { return p.Accent })).
								TextDecorationLine("underline")},
						},
						"@ada",
					)
					ui.PreviewCard.Positioner(
						ui.PartProps{},
						func() {
							ui.PreviewCard.Popup(
								popup(240),
								func() {
									label("Ada Lovelace")
									muted("Wrote the first algorithm intended for a machine.")
								},
							)
						},
					)
				},
			)
		})
		note(func() string { return "open " + strconv.FormatBool(open()) })
	})
}
func ToastDemo() {
	ui.Toast.Provider(
		ui.ToastProviderProps{
			Timeout:        5000,
			Limit:          3,
			SwipeDirection: "right",
			Pitch:          8,
		},
		toastDemoBody,
	)
}
func toastDemoBody() {
	manager := ui.UseToastManager()
	counter := 0
	last := ""
	panel("Toast", "Info, success, and error notifications with update, close, swipe, and automatic dismissal. Stack geometry comes from the core.", func() {
		row(func() {
			for _, kind := range []ui.ToastType{"info", "success", "error"} {
				button(string(kind), func() {
					counter++
					last = manager.Add(ui.ToastRequest{
						Title:       "Build " + strconv.Itoa(counter) + " · " + string(kind),
						Description: "Auto-dismisses in 5 seconds",
						Type:        kind,
					})
				})
			}
			button("Update last", func() {
				if last != "" {
					manager.Update(last, ui.ToastUpdate{Title: ptr("Updated in place")})
				}
			})
			button("Clear", manager.CloseAll)
		})
		ui.Toast.Viewport(
			ui.ToastViewportProps{PartProps: ui.PartProps{Style: ui.Style().
				Display("flex").
				FlexDirection("column").
				Gap(8).
				MinHeight(40)}},
			func() {
				ui.KeyedFor(
					manager.Stack,
					func(entry ui.ToastStackEntry) any { return entry.ID },
					func(entry func() ui.ToastStackEntry, _ func() int) {
						id := entry().ID
						current := func() ui.ToastDeclaration {
							for _, toast := range manager.Toasts() {
								if toast.ID == id {
									return toast
								}
							}
							return ui.ToastDeclaration{ID: id}
						}
						ui.Toast.Positioner(
							ui.ToastPartProps{ToastID: id},
							func() {
								ui.Toast.Root(
									ui.ToastPartProps{
										ToastID: id,
										PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
											s := popupStyle()
											s = s.Padding(10)
											s = s.Opacity(choose(entry().Limited, 0.55, 1.0))
											s = s.Transform("translateX(" + strconv.FormatFloat(entry().SwipeMovement, 'g', -1, 64) + "px)")
											s = s.BorderColor(choose(entry().Type == "error", p().Danger, choose(entry().Type == "success", p().Accent, p().Border)))
											return s
										}},
									},
									func() {
										ui.Toast.Content(
											ui.ToastPartProps{ToastID: id},
											func() {
												row(func() {
													ui.Toast.Title(
														ui.ToastPartProps{ToastID: id},
														func() {
															label(func() string { return current().Title })
														},
													)
													muted(func() string {
														return "#" + strconv.Itoa(entry().Index) + " · +" + strconv.FormatFloat(entry().Offset, 'g', -1, 64) + "px"
													})
													ui.Toast.Close(
														ui.ToastPartProps{
															ToastID:   id,
															PartProps: ui.PartProps{AriaLabel: "Dismiss notification"},
														},
														"×",
													)
												})
												ui.Toast.Description(
													ui.ToastPartProps{ToastID: id},
													func() {
														muted(func() string { return current().Description })
													},
												)
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
		note(func() string {
			limited := 0
			for _, entry := range manager.Stack() {
				if entry.Limited {
					limited++
				}
			}
			return "queued " + strconv.Itoa(len(manager.Toasts())) + " · stack " + strconv.Itoa(len(manager.Stack())) + " · limited " + strconv.Itoa(limited)
		})
	})
}
func TooltipDemo() {
	open, setOpen := ui.CreateSignal(false)
	placement, setPlacement := ui.CreateSignal("—")
	panel("Tooltip", "A provider shares a warm delay across triggers. The second tooltip tracks the horizontal cursor position.", func() {
		ui.Tooltip.Provider(
			ui.TooltipProviderProps{Delay: 500, CloseDelay: 120, Timeout: 400},
			func() {
				row(func() {
					for i, caption := range []string{"Hover me", "Tracks the cursor"} {
						props := ui.TooltipRootProps{Side: "top", SideOffset: ptr(8.0)}
						if i == 0 {
							props.OnOpenChange = change(setOpen)
							props.OnPlacementChange = func(d ui.AnchorPlacementDetails, _ *native.Event) { setPlacement(d.Side + "/" + d.Align) }
						} else {
							props.Side = "bottom"
							props.TrackCursorAxis = "x"
						}
						ui.Tooltip.Root(
							props,
							func() {
								ui.Tooltip.Trigger(
									ui.TooltipTriggerProps{PartProps: control()},
									caption,
								)
								ui.Tooltip.Positioner(
									ui.TooltipPositionerProps{},
									func() {
										s := popupStyle()
										s = s.BackgroundColor(color(func(p palette) string { return p.Ink }))
										s = s.TextColor(color(func(p palette) string { return p.Panel }))
										s = s.Padding(8)
										s = s.FontSize(11)
										ui.Tooltip.Popup(
											ui.PartProps{Style: s},
											func() {
												ui.Text(choose(i == 0, "Shared hover delay", "Tracks the horizontal cursor"))
												if i == 0 {
													ui.Tooltip.Arrow(ui.PartProps{Style: ui.Style().
														Width(8).
														Height(8).
														BackgroundColor(color(func(p palette) string { return p.Ink }))})
												}
											},
										)
									},
								)
							},
						)
					}
				})
			},
		)
		note(func() string { return "open " + strconv.FormatBool(open()) + " · resolved " + placement() })
	})
}
