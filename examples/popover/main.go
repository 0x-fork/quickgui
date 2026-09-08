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
		PartProps:      ui.PartProps{Style: ui.Style().Width("100%").Height("100%")},
	}
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text(
						"Native popover surfaces",
					).FontSize(14).FontWeight(600)
				},
			).Display("flex").Height(52).FlexShrink(0).AlignItems("center").JustifyContent("center").AppRegion("drag").BorderColor("#202838").BorderBottomWidth(1)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.View(
								func() {
									ui.Text(
										"System and in-window popovers",
									).FontSize(28).LineHeight(34).FontWeight(700)
									ui.Text(
										"Both use the same controlled Go API. SystemPopover opens a native child window; Popover stays in this window's retained overlay plane.",
									).TextColor("#9ba8bc").FontSize(14).LineHeight(21)
								},
							).Display("flex").FlexDirection("column").Gap(8)
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
							).Display("flex").Gap(14)
							ui.View(
								func() {
									ui.Text(
										status,
									).TextColor("#b8c4d6").FontSize(13)
								},
							).Display("flex").MinHeight(50).AlignItems("center").JustifyContent("center").PaddingLeft(16).PaddingRight(16).BackgroundColor("#10151e").BorderColor("#293244").BorderWidth(1).BorderRadius(9)
						},
					).Display("flex").FlexDirection("column").Width("100%").MaxWidth(680).Gap(20)
				},
			).Display("flex").Flex(1).MinHeight(0).AlignItems("center").JustifyContent("center").Padding(36)
		},
	).Display("flex").FlexDirection("column").Width("100%").Height("100%").BackgroundColor("#0b0f17").TextColor("#f5f7fb")
}
