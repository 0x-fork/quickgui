package main

import (
	"fmt"
	"slices"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func AccordionDemo() {
	open, setOpen := ui.CreateSignal([]string{"declared"})
	entries := []struct{ value, title, body string }{
		{"declared", "Everything is declared", "Declare values and elements in Go. The native core owns roles, focus, and deadlines."},
		{"panels", "Closed panels are not mounted", "A closed Accordion.Panel contributes no layout, paint, input, or accessibility node."},
		{"heading", "Heading level belongs to the core", "HeadingLevel is clamped to 1 through 6 and projected on every header."},
	}
	panel("Accordion", "Multiple open items, semantic headings, and lazy panels. Click a header or use the arrow keys.", func() {
		ui.Accordion.Root(
			ui.AccordionRootProps{
				Value:         open,
				OnValueChange: change(setOpen),
				Multiple:      true,
				HeadingLevel:  3,
				PartProps:     columnPart(),
			},
			func() {
				for index, entry := range entries {
					ui.Accordion.Item(
						ui.AccordionItemProps{
							Value: entry.value,
							Index: ptr(index),
							PartProps: ui.PartProps{Style: func() ui.Style {
								return ui.Styles(
									ui.Display("flex"),
									ui.FlexDirection("column"),
									ui.BorderRadius(10),
									ui.BorderWidth(1),
									ui.BorderColor(p().Border),
									ui.BackgroundColor(p().PanelAlt),
									ui.Overflow("hidden"),
								)
							}},
						},
						func() {
							ui.Accordion.Header(
								ui.PartProps{Style: ui.Styles(ui.Display("flex"), ui.Width("100%"))},
								func() {
									props := control()
									s := controlStyle()
									s.Width = "100%"
									s.JustifyContent = "space-between"
									s.BorderWidth = 0
									s.Height = 34
									props.Style = s
									ui.Accordion.Trigger(
										props,
										func() {
											label(entry.title)
											label(func() string { return choose(slices.Contains(open(), entry.value), "−", "+") })
										},
									)
								},
							)
							ui.Accordion.Panel(
								ui.PartProps{Style: ui.Styles(ui.Padding(12), ui.PaddingTop(4))},
								func() { muted(entry.body) },
							)
						},
					)
				}
			},
		)
		note(func() string { return "open: " + strings.Join(open(), ", ") })
	})
}

func AlertDialogDemo() {
	open, setOpen := ui.CreateSignal(false)
	outcome, setOutcome := ui.CreateSignal("nothing yet")
	reason, setReason := ui.CreateSignal("—")
	panel("Alert dialog", "The backdrop refuses dismissal. Choose an answer or press Escape; focus returns to the trigger.", func() {
		ui.AlertDialog.Root(
			ui.DialogRootProps{
				Open: open,
				OnOpenChange: func(next bool, details ui.DialogOpenChangeDetails) {
					setOpen(next)
					setReason(string(details.Reason))
					if !next && outcome() == "nothing yet" {
						setOutcome("dismissed")
					}
				},
			},
			func() {
				ui.AlertDialog.Trigger(control(), "Delete branch…")
				ui.AlertDialog.Portal(
					overlay(),
					func() {
						ui.AlertDialog.Backdrop(backdrop())
						ui.AlertDialog.Popup(
							ui.DialogPopupProps{PartProps: popup(340)},
							func() {
								ui.AlertDialog.Title(
									ui.PartProps{},
									func() {
										ui.Text(
											"Delete “electron-parity”?",
											ui.FontSize(15),
											ui.FontWeight(700),
										)
									},
								)
								ui.AlertDialog.Description(
									ui.PartProps{},
									func() {
										muted("This gallery records the answer; it does not delete a real branch.")
									},
								)
								row(func() {
									ui.AlertDialog.Close(control(), "Cancel")
									button("Delete", func() { setOutcome("deleted"); setOpen(false) }, ui.Styles(
										ui.BackgroundColor(color(func(p palette) string { return p.Danger })),
										ui.TextColor("white"),
									))
								})
							},
						)
					},
				)
			},
		)
		note(func() string {
			return "open " + strconv.FormatBool(open()) + " · reason " + reason() + " · outcome " + outcome()
		})
	})
}

func AvatarDemo() {
	missing, setMissing := ui.CreateSignal("idle")
	fallback, setFallback := ui.CreateSignal("idle")
	shell := func() ui.Style {
		return ui.Styles(
			ui.Width(44),
			ui.Height(44),
			ui.BorderRadius(22),
			ui.Display("flex"),
			ui.AlignItems("center"),
			ui.JustifyContent("center"),
			ui.BackgroundColor(p().Selection),
			ui.BorderColor(p().Border),
			ui.BorderWidth(1),
			ui.Overflow("hidden"),
		)
	}
	panel("Avatar", "Images and fallbacks share one accessible name. Failed loads report their actual status.", func() {
		row(func() {
			ui.Avatar.Root(
				ui.AvatarRootProps{
					OnLoadingStatusChange: change(setMissing),
					PartProps: ui.PartProps{
						AriaLabel: "Ada Lovelace",
						Style:     shell,
					},
				},
				func() {
					ui.Avatar.Image(ui.AvatarImageProps{
						Src: "./avatar-does-not-exist.png",
						PartProps: ui.PartProps{Style: ui.Styles(
							ui.Width(44),
							ui.Height(44),
						)},
					})
					ui.Avatar.Fallback(ui.AvatarFallbackProps{Delay: 120}, "AL")
				},
			)
			ui.Avatar.Root(
				ui.AvatarRootProps{
					OnLoadingStatusChange: change(setFallback),
					PartProps: ui.PartProps{
						AriaLabel: "Grace Hopper",
						Style:     shell,
					},
				},
				func() {
					ui.Avatar.Fallback(
						ui.AvatarFallbackProps{},
						"GH",
					)
				},
			)
		})
		note(func() string { return "with image: " + missing() + " · fallback only: " + fallback() })
	})
}

func ButtonDemo() {
	clicks, setClicks := ui.CreateSignal(0)
	last, setLast := ui.CreateSignal("nothing yet")
	panel("Button", "Native press, focus, disabled state, and double-click handling.", func() {
		row(func() {
			primary("Primary", func() { setClicks(clicks() + 1); setLast("primary") })
			button("Secondary", func() { setClicks(clicks() + 1); setLast("secondary") })
			button("Disabled", func() { setLast("never") }, ui.Disabled(true))
			button("Double-click me", func() { setLast("single click") }, ui.OnDoubleClick(func(*native.Event) { setLast("double click") }))
		})
		note(func() string { return "clicks " + strconv.Itoa(clicks()) + " · last " + last() })
	})
}

func CheckboxDemo() {
	notify, setNotify := ui.CreateSignal[ui.CheckedState]("indeterminate")
	kids, setKids := ui.CreateSignal([]bool{true, false})
	panel("Checkbox", "Tri-state, read-only, and a parent whose mixed state is derived from ChildrenChecked.", func() {
		checkbox(ui.CheckboxProps{
			Checked:         notify,
			OnCheckedChange: func(v bool, _ *native.Event) { setNotify(v) },
		}, "Email me about releases")
		checkbox(ui.CheckboxProps{DefaultChecked: true, ReadOnly: true}, "Read-only: focusable, refuses changes")
		checkbox(ui.CheckboxProps{
			Parent:          true,
			ChildrenChecked: kids,
			Checked: func() ui.CheckedState {
				checked := 0
				for _, value := range kids() {
					if value {
						checked++
					}
				}
				return checkboxSelectionState(checked, len(kids()))
			},
			OnCheckedChange: func(value bool, _ *native.Event) {
				next := make([]bool, len(kids()))
				for i := range next {
					next[i] = value
				}
				setKids(next)
			},
		}, "Parent, derived from its children")
		row(func() {
			for i, name := range []string{"Analytics", "Crash reports"} {
				checkbox(ui.CheckboxProps{
					Checked:         func() ui.CheckedState { return kids()[i] },
					OnCheckedChange: func(v bool, _ *native.Event) { next := slices.Clone(kids()); next[i] = v; setKids(next) },
				}, name)
			}
		})
		note(func() string { return fmt.Sprintf("notify %v · children %v", notify(), kids()) })
	})
}
func CheckboxGroupDemo() {
	colors, setColors := ui.CreateSignal([]string{"green"})
	all := []string{"red", "green", "blue", "violet"}
	panel("Checkbox group", "The parent toggles all values and derives its mixed state from the group.", func() {
		ui.CheckboxGroup.Root(
			ui.CheckboxGroupProps{
				AllValues:     all,
				Value:         colors,
				OnValueChange: change(setColors),
				PartProps:     columnPart(),
			},
			func() {
				checkbox(ui.CheckboxProps{
					Parent: true,
					Checked: func() ui.CheckedState {
						return checkboxSelectionState(len(colors()), len(all))
					},
				}, "All colours")
				for _, value := range all {
					checkbox(ui.CheckboxProps{
						Value:   value,
						Checked: func() ui.CheckedState { return slices.Contains(colors(), value) },
					}, value)
				}
			},
		)
		note(func() string { return "checked: " + strings.Join(colors(), ", ") })
	})
}
func CollapsibleDemo() {
	open, setOpen := ui.CreateSignal(false)
	kept, setKept := ui.CreateSignal(true)
	panel("Collapsible", "Closed panels are omitted; KeepMounted instead retains the panel with display:none.", func() {
		ui.Collapsible.Root(
			ui.CollapsibleRootProps{
				Open:         open,
				OnOpenChange: change(setOpen),
				PartProps:    columnPart(),
			},
			func() {
				ui.Collapsible.Trigger(
					control(),
					func() {
						label(func() string { return choose(open(), "Hide advanced options", "Show advanced options") })
					},
				)
				ui.Collapsible.Panel(
					ui.PartProps{},
					func() {
						muted("While closed, this panel contributes no layout, paint, input, or accessibility node.")
					},
				)
			},
		)
		ui.Collapsible.Root(
			ui.CollapsibleRootProps{
				Open:         kept,
				OnOpenChange: change(setKept),
				KeepMounted:  true,
				PartProps:    columnPart(),
			},
			func() {
				ui.Collapsible.Trigger(control(), "Toggle kept panel")
				ui.Collapsible.Panel(ui.PartProps{}, "Retained as display:none instead of omitted.")
			},
		)
		note(func() string {
			return "plain " + strconv.FormatBool(open()) + " · keepMounted " + strconv.FormatBool(kept())
		})
	})
}
func DialogDemo() {
	open, setOpen := ui.CreateSignal(false)
	reason, setReason := ui.CreateSignal("—")
	completed, setCompleted := ui.CreateSignal("—")
	notes, setNotes := ui.CreateSignal("")
	panel("Dialog", "An in-window dialog with a focus trap, a scrollable body, and a 120ms exit deadline.", func() {
		ui.Dialog.Root(
			ui.DialogRootProps{
				Open:                 open,
				ExitDuration:         120,
				OnOpenChange:         func(v bool, d ui.DialogOpenChangeDetails) { setOpen(v); setReason(string(d.Reason)) },
				OnOpenChangeComplete: func(v bool, _ *native.Event) { setCompleted(choose(v, "opened", "closed")) },
			},
			func() {
				ui.Dialog.Trigger(control(), "Open dialog")
				ui.Dialog.Portal(
					overlay(),
					func() {
						ui.Dialog.Backdrop(backdrop())
						ui.Dialog.Popup(
							ui.DialogPopupProps{PartProps: popup(360)},
							func() {
								ui.Dialog.Title(
									ui.PartProps{},
									func() {
										ui.Text(
											"Publish this build?",
											ui.FontSize(15),
											ui.FontWeight(700),
										)
									},
								)
								ui.Dialog.Description(
									ui.PartProps{},
									func() {
										muted("Escape, the backdrop, and Cancel restore focus to the trigger.")
									},
								)
								ui.Dialog.Viewport(
									ui.PartProps{Style: ui.Styles(
										ui.Display("flex"),
										ui.FlexDirection("column"),
										ui.Gap(8),
										ui.MaxHeight(160),
										ui.Padding(4),
									)},
									func() {
										input(notes, setNotes, "Release notes", ui.Multiline(true), ui.Styles(
											ui.Width("100%"),
											ui.Height(64),
											ui.PaddingTop(6),
										))
									},
								)
								row(func() { ui.Dialog.Close(control(), "Cancel"); primary("Publish", func() { setOpen(false) }) })
							},
						)
					},
				)
			},
		)
		note(func() string {
			return "open " + strconv.FormatBool(open()) + " · reason " + reason() + " · transition " + completed() + " · notes " + strconv.Itoa(len(notes())) + " chars"
		})
	})
}
