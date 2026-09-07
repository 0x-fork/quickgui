package main

import (
	"strconv"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

// Row members follow the nearest group; hints explicitly follow the outer list.
func InteractionStates() {
	dropStatus, setDropStatus := ui.CreateSignal("Drop the swatch or a file here.")
	rowStyle := ui.Style{
		Display:         "flex",
		AlignItems:      "center",
		Gap:             10,
		Height:          40,
		PaddingLeft:     12,
		PaddingRight:    8,
		BorderRadius:    10,
		BackgroundColor: "#1b2434",
		Transition:      "background-color 120ms",
		Hover:           &ui.Style{BackgroundColor: "#243047"},
		FocusWithin:     &ui.Style{Outline: "1px solid #93c5fd"},
	}
	actionStyle := ui.Style{
		Display:         "flex",
		AlignItems:      "center",
		JustifyContent:  "center",
		Height:          26,
		PaddingLeft:     10,
		PaddingRight:    10,
		BorderRadius:    7,
		BackgroundColor: "#2b3a5c",
		Opacity:         0,
		Transition:      "opacity 120ms, background-color 120ms",
		UserSelect:      "none",
		GroupHover:      &ui.Style{Opacity: 1},
		GroupActive:     &ui.Style{Opacity: 0.7},
		Hover:           &ui.Style{BackgroundColor: "#3b82f6", Transform: "translate(0, -1px)"},
		Active:          &ui.Style{BackgroundColor: "#1d4ed8", Transform: "scale(0.97)"},
		Focus:           &ui.Style{Outline: "2px solid #93c5fd"},
		Disabled:        &ui.Style{Opacity: 0.35, Cursor: "not-allowed"},
	}
	Panel("Interaction states", func() {
		ui.View(
			func() {
				for index, title := range []string{"Quarterly report", "Roadmap draft"} {
					ui.View(
						func() {
							ui.Text(title, ui.Style{Flex: 1, FontSize: 13, Color: ink})
							ui.Text(
								"Cmd ",
								index+1,
								ui.Style{
									FontSize:   11,
									Color:      muted,
									Opacity:    0,
									Transition: "opacity 120ms",
								},
								ui.GroupHoverNamed("list", ui.Style{Opacity: 1}),
							)
							ui.Button(
								func() {
									ui.Text("Rename", ui.Style{FontSize: 12, Color: ink})
								},
								actionStyle,
								ui.AriaLabel(title+" Rename"),
							)
							ui.Button(
								func() {
									ui.Text("Share", ui.Style{FontSize: 12, Color: ink})
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
			ui.Style{Display: "flex", FlexDirection: "column", Gap: 12},
			ui.Group("list"),
		)
		ui.View(
			"Drag swatch",
			centered,
			ui.Style{
				Height:          30,
				BorderRadius:    8,
				BackgroundColor: "#1d4ed8",
				Color:           ink,
				FontSize:        12,
				UserSelect:      "none",
				Dragging:        &ui.Style{Opacity: 0.45},
			},
			ui.Draggable(ui.DragSource{ID: "swatch", Text: "swatch"}),
		)
		ui.View(
			func() {
				ui.Text(
					dropStatus,
					ui.Style{FontSize: 12, Color: muted},
				)
			},
			centered,
			ui.Style{
				Height:       54,
				PaddingLeft:  12,
				PaddingRight: 12,
				BorderRadius: 12,
				BorderWidth:  1,
				BorderStyle:  "dashed",
				BorderColor:  panelBorder,
				Transition:   "background-color 120ms, border-color 120ms",
				Dragging:     &ui.Style{Opacity: 0.6},
				DragOver:     &ui.Style{BorderColor: "#38bdf8", Background: "#38bdf826"},
			},
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
										ui.Style{
											FontSize:   12,
											FontWeight: 700,
											Color:      ink,
										},
									)
								},
								ui.Style{
									Display:         "flex",
									AlignItems:      "center",
									Position:        "sticky",
									Top:             0,
									Height:          28,
									FlexShrink:      0,
									PaddingLeft:     12,
									BackgroundColor: color,
								},
							)
							for row := range 6 {
								ui.View(
									func() {
										ui.Text(
											section,
											" row ",
											row,
											ui.Style{
												FontSize: 12,
												Color:    muted,
											},
										)
									},
									ui.Style{
										Display:     "flex",
										AlignItems:  "center",
										Height:      30,
										FlexShrink:  0,
										PaddingLeft: 12,
									},
								)
							}
						},
						ui.Style{Display: "flex", FlexDirection: "column", FlexShrink: 0},
					)
				}
			},
			ui.Style{
				Height:        200,
				OverflowY:     "scroll",
				Display:       "flex",
				FlexDirection: "column",
				BorderRadius:  10,
				BorderWidth:   1,
				BorderColor:   panelBorder,
			},
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
								ui.Style{
									FontSize:   14,
									FontWeight: 700,
									Color:      "#f8fafc",
								},
							)
						},
						centered,
						ui.Style{
							Width:           200,
							Height:          110,
							FlexShrink:      0,
							ScrollSnapAlign: "start",
							ScrollSnapStop:  "always",
							BorderRadius:    12,
							BackgroundColor: tint,
						},
					)
				}
			},
			ui.Style{
				Height:         130,
				OverflowX:      "scroll",
				ScrollSnapType: "x mandatory",
				Display:        "flex",
				Gap:            12,
				Padding:        6,
				BorderRadius:   10,
				BorderWidth:    1,
				BorderColor:    panelBorder,
			},
		)
		ui.Text(
			"The core resolves the snap target at the momentum end phase and animates to it on exact deadlines, leaving the window settled.",
			ui.Style{FontSize: 12, Color: muted},
		)
	})
}
