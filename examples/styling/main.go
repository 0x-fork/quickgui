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
				Title:                "QuickGUI Styling",
				Width:                960,
				Height:               720,
				MinimumWidth:         720,
				MinimumHeight:        560,
				Background:           "#0b0f17",
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 18},
				Component:            Styling,
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

func Styling() {
	warm, setWarm := ui.CreateSignal(false)
	warmPalette.Provide(warm, func() {
		ui.View(
			func() {
				ui.View(
					func() {
						ui.Text(
							"Declared styling",
						).FontSize(14).FontWeight(700)
						ui.Button(
							"Switch palette",
						).Height(28).PaddingLeft(12).PaddingRight(12).BorderRadius(7).BackgroundColor("#1b2434").TextColor(ink).FontSize(12).AppRegion("no-drag").UserSelect("none").Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
							return s.BackgroundColor("#243047")
						}).OnClick(func() { setWarm(!warm()) })

					},
				).Display("flex").Height(52).FlexShrink(0).AlignItems("center").JustifyContent("space-between").PaddingLeft(96).PaddingRight(20).AppRegion("drag")
				ui.View(
					func() {
						TextAlignment()
						TextStyling()
						Direction()
						Gradients()
						BordersAndOutlines()
						Filters()
						Transforms()
						InteractionStates()
						StickyHeaders()
						ScrollSnap()
					},
				).Flex(1).MinHeight(0).OverflowY("scroll").Padding(20).Display("grid").GridTemplateColumns("1fr 1fr").Gap(16)
			},
		).Display("flex").FlexDirection("column").Width("100%").Height("100%").BackgroundColor("#0b0f17").TextColor(ink).When(
			warm,
			ui.Style().BackgroundColor("#251b13"),
		)

	})
}
