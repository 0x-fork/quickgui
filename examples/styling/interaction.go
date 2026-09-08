package main

import (
	"strconv"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

// Row members follow the nearest group; hints explicitly follow the outer list.
func InteractionStates() {
	dropStatus, setDropStatus := ui.CreateSignal("Drop the swatch or a file here.")
	rowStyle := ui.Styles(
		ui.Display("flex"),
		ui.AlignItems("center"),
		ui.Gap(10),
		ui.Height(40),
		ui.PaddingLeft(12),
		ui.PaddingRight(8),
		ui.BorderRadius(10),
		ui.BackgroundColor("#1b2434"),
		ui.Transition("background-color 120ms"),
		ui.Hover(ui.BackgroundColor("#243047")),
		ui.FocusWithin(ui.Outline("1px solid #93c5fd")),
	)
	actionStyle := ui.Styles(
		ui.Display("flex"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Height(26),
		ui.PaddingLeft(10),
		ui.PaddingRight(10),
		ui.BorderRadius(7),
		ui.BackgroundColor("#2b3a5c"),
		ui.Opacity(0),
		ui.Transition("opacity 120ms, background-color 120ms"),
		ui.UserSelect("none"),
		ui.GroupHover(ui.Opacity(1)),
		ui.GroupActive(ui.Opacity(0.7)),
		ui.Hover(ui.BackgroundColor("#3b82f6"), ui.Transform("translate(0, -1px)")),
		ui.Active(ui.BackgroundColor("#1d4ed8"), ui.Transform("scale(0.97)")),
		ui.Focus(ui.Outline("2px solid #93c5fd")),
		ui.DisabledStyle(ui.Opacity(0.35), ui.Cursor("not-allowed")),
	)
	Panel("Interaction states", func() {
		ui.View(
			func() {
				for index, title := range []string{"Quarterly report", "Roadmap draft"} {
					ui.View(
						func() {
							ui.Text(title, ui.Flex(1), ui.FontSize(13), ui.TextColor(ink))
							ui.Text(
								"Cmd ",
								index+1,
								ui.FontSize(11),
								ui.TextColor(muted),
								ui.Opacity(0),
								ui.Transition("opacity 120ms"),
								ui.GroupHoverNamed("list", ui.Opacity(1)),
							)
							ui.Button(
								func() {
									ui.Text("Rename", ui.FontSize(12), ui.TextColor(ink))
								},
								actionStyle,
								ui.AriaLabel(title+" Rename"),
							)
							ui.Button(
								func() {
									ui.Text("Share", ui.FontSize(12), ui.TextColor(ink))
								},
								actionStyle,
								ui.Disabled(true),
								ui.AriaLabel(title+" Share"),
							)
						},
						rowStyle,
						ui.Group(true),
					)
				}
			},
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.Gap(12),
			ui.Group("list"),
		)
		ui.View(
			"Drag swatch",
			centered,
			ui.Height(30),
			ui.BorderRadius(8),
			ui.BackgroundColor("#1d4ed8"),
			ui.TextColor(ink),
			ui.FontSize(12),
			ui.UserSelect("none"),
			ui.Dragging(ui.Opacity(0.45)),
			ui.Draggable(ui.DragSource{ID: "swatch", Text: "swatch"}),
		)
		ui.View(
			func() {
				ui.Text(
					dropStatus,
					ui.FontSize(12),
					ui.TextColor(muted),
				)
			},
			centered,
			ui.Height(54),
			ui.PaddingLeft(12),
			ui.PaddingRight(12),
			ui.BorderRadius(12),
			ui.BorderWidth(1),
			ui.BorderStyle("dashed"),
			ui.BorderColor(panelBorder),
			ui.Transition("background-color 120ms, border-color 120ms"),
			ui.Dragging(ui.Opacity(0.6)),
			ui.DragOver(ui.BorderColor("#38bdf8"), ui.Background("#38bdf826")),
			ui.DropKinds([]string{"files", "local"}),
			ui.OnDrop(func(event *native.Event) {
				if drop := ui.DropFromEvent(event); drop != nil {
					setDropStatus("Dropped " + drop.ID)
				}
			}),
			ui.OnFilesDropped(func(event *native.Event) {
				if drop := ui.DropFromEvent(event); drop != nil {
					setDropStatus("Dropped " + strconv.Itoa(len(drop.Paths)) + " file(s)")
				}
			}),
			ui.Draggable(ui.DragSource{
				ID:   "drop-zone",
				Text: "drop-zone",
			}),
		)
	})
}

func StickyHeaders() {
	Panel("Sticky headers", func() {
		ui.View(
			func() {
				for index, section := range []string{"Inbox", "Archive", "Trash"} {
					color := "#1e3a5f"
					if index%2 != 0 {
						color = "#3c2858"
					}
					ui.View(
						func() {
							ui.View(
								func() {
									ui.Text(
										section,
										ui.FontSize(12),
										ui.FontWeight(700),
										ui.TextColor(ink),
									)
								},
								ui.Display("flex"),
								ui.AlignItems("center"),
								ui.Position("sticky"),
								ui.Top(0),
								ui.Height(28),
								ui.FlexShrink(0),
								ui.PaddingLeft(12),
								ui.BackgroundColor(color),
							)
							for row := range 6 {
								ui.View(
									func() {
										ui.Text(
											section,
											" row ",
											row,
											ui.FontSize(12),
											ui.TextColor(muted),
										)
									},
									ui.Display("flex"),
									ui.AlignItems("center"),
									ui.Height(30),
									ui.FlexShrink(0),
									ui.PaddingLeft(12),
								)
							}
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.FlexShrink(0),
					)
				}
			},
			ui.Height(200),
			ui.OverflowY("scroll"),
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.BorderRadius(10),
			ui.BorderWidth(1),
			ui.BorderColor(panelBorder),
		)
	})
}

func ScrollSnap() {
	tints := []string{"#1d4ed8", "#7c3aed", "#0f766e", "#b45309", "#be123c"}
	Panel("Mandatory scroll snap", func() {
		ui.View(
			func() {
				for index, tint := range tints {
					ui.View(
						func() {
							ui.Text(
								"page ",
								index,
								ui.FontSize(14),
								ui.FontWeight(700),
								ui.TextColor("#f8fafc"),
							)
						},
						centered,
						ui.Width(200),
						ui.Height(110),
						ui.FlexShrink(0),
						ui.ScrollSnapAlign("start"),
						ui.ScrollSnapStop("always"),
						ui.BorderRadius(12),
						ui.BackgroundColor(tint),
					)
				}
			},
			ui.Height(130),
			ui.OverflowX("scroll"),
			ui.ScrollSnapType("x mandatory"),
			ui.Display("flex"),
			ui.Gap(12),
			ui.Padding(6),
			ui.BorderRadius(10),
			ui.BorderWidth(1),
			ui.BorderColor(panelBorder),
		)
		ui.Text(
			"The core resolves the snap target at the momentum end phase and animates to it on exact deadlines, leaving the window settled.",
			ui.FontSize(12),
			ui.TextColor(muted),
		)
	})
}
