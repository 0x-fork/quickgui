package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:                "QuickGUI Popovers",
				Width:                760,
				Height:               540,
				MinimumWidth:         620,
				MinimumHeight:        480,
				Background:           "#0b0f17",
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 14},
				Component:            Popovers,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func Popovers() {
	status, setStatus := ui.CreateSignal("Open either surface to compare its native behavior.")
	systemOpen, setSystemOpen := ui.CreateSignal(false)
	inWindowOpen, setInWindowOpen := ui.CreateSignal(false)
	changeSystem := func(open bool) {
		setSystemOpen(open)
		if open {
			setStatus("System popover opened.")
		} else {
			setStatus("System popover closed.")
		}
	}
	changeInWindow := func(open bool) {
		setInWindowOpen(open)
		if open {
			setStatus("In-window popover opened.")
		} else {
			setStatus("In-window popover closed.")
		}
	}
	gap, margin := 8.0, 12.0
	placement := ui.PopoverContentProps{
		Width:          340,
		Height:         220,
		Placement:      "bottom-start",
		Gap:            &gap,
		ViewportMargin: &margin,
		PartProps:      ui.PartProps{Style: ui.Styles(ui.Width("100%"), ui.Height("100%"))},
	}
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text("Native popover surfaces", ui.FontSize(14), ui.FontWeight(600))
				},
				ui.Display("flex"),
				ui.Height(52),
				ui.FlexShrink(0),
				ui.AlignItems("center"),
				ui.JustifyContent("center"),
				ui.AppRegion("drag"),
				ui.BorderColor("#202838"),
				ui.BorderBottomWidth(1),
			)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.View(
								func() {
									ui.Text(
										"System and in-window popovers",
										ui.FontSize(28),
										ui.LineHeight(34),
										ui.FontWeight(700),
									)
									ui.Text(
										"Both use the same controlled Go API. SystemPopover opens a native child window; Popover stays in this window's retained overlay plane.",
										ui.Color("#9ba8bc"),
										ui.FontSize(14),
										ui.LineHeight(21),
									)
								},
								ui.Display("flex"),
								ui.FlexDirection("column"),
								ui.Gap(8),
							)
							ui.View(
								func() {
									card("SystemPopover", "A child window that may cross the owner's edge and stays within the display.", func() {
										ui.SystemPopover.Root(
											ui.PopoverRootProps{
												Open:         systemOpen,
												OnOpenChange: func(open bool, _ ui.PopoverOpenChangeDetails) { changeSystem(open) },
											},
											func() {
												ui.SystemPopover.Trigger(
													ui.PopoverTriggerProps{PartProps: ui.PartProps{Style: buttonStyle}},
													func() {
														ui.Text(func() string {
															if systemOpen() {
																return "Close system"
															}
															return "Open system"
														})
													},
												)
												ui.SystemPopover.Content(
													placement,
													func() {
														content("System popover", "It has its own retained tree on a native child surface and may cross the owner window's edge.", func() { changeSystem(false) })
													},
												)
											},
										)
									})
									card("In-window popover", "A retained overlay that flips and shifts but remains inside this window.", func() {
										ui.Popover.Root(
											ui.PopoverRootProps{
												Open:         inWindowOpen,
												OnOpenChange: func(open bool, _ ui.PopoverOpenChangeDetails) { changeInWindow(open) },
											},
											func() {
												ui.Popover.Trigger(
													ui.PopoverTriggerProps{PartProps: ui.PartProps{Style: buttonStyle}},
													func() {
														ui.Text(func() string {
															if inWindowOpen() {
																return "Close in-window"
															}
															return "Open in-window"
														})
													},
												)
												ui.Popover.Content(
													placement,
													func() {
														content("In-window popover", "It shares this window's tree and renders above ordinary content without creating another native window.", func() { changeInWindow(false) })
													},
												)
											},
										)
									})
								},
								ui.Display("flex"),
								ui.Gap(14),
							)
							ui.View(
								func() {
									ui.Text(status, ui.Color("#b8c4d6"), ui.FontSize(13))
								},
								ui.Display("flex"),
								ui.MinHeight(50),
								ui.AlignItems("center"),
								ui.JustifyContent("center"),
								ui.PaddingLeft(16),
								ui.PaddingRight(16),
								ui.BackgroundColor("#10151e"),
								ui.BorderColor("#293244"),
								ui.BorderWidth(1),
								ui.BorderRadius(9),
							)
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Width("100%"),
						ui.MaxWidth(680),
						ui.Gap(20),
					)
				},
				ui.Display("flex"),
				ui.Flex(1),
				ui.MinHeight(0),
				ui.AlignItems("center"),
				ui.JustifyContent("center"),
				ui.Padding(36),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.BackgroundColor("#0b0f17"),
		ui.Color("#f5f7fb"),
	)
}
