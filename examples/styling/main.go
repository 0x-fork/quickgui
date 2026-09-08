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
						ui.Text("Declared styling", ui.FontSize(14), ui.FontWeight(700))
						ui.Button(
							"Switch palette",
							ui.Height(28),
							ui.PaddingLeft(12),
							ui.PaddingRight(12),
							ui.BorderRadius(7),
							ui.BackgroundColor("#1b2434"),
							ui.TextColor(ink),
							ui.FontSize(12),
							ui.AppRegion("no-drag"),
							ui.UserSelect("none"),
							ui.Hover(ui.BackgroundColor("#243047")),
							ui.OnClick(func() { setWarm(!warm()) }),
						)
					},
					ui.Display("flex"),
					ui.Height(52),
					ui.FlexShrink(0),
					ui.AlignItems("center"),
					ui.JustifyContent("space-between"),
					ui.PaddingLeft(96),
					ui.PaddingRight(20),
					ui.AppRegion("drag"),
				)
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
					ui.Flex(1),
					ui.MinHeight(0),
					ui.OverflowY("scroll"),
					ui.Padding(20),
					ui.Display("grid"),
					ui.GridTemplateColumns("1fr 1fr"),
					ui.Gap(16),
				)
			},
			ui.Display("flex"),
			ui.FlexDirection("column"),
			ui.Width("100%"),
			ui.Height("100%"),
			ui.BackgroundColor("#0b0f17"),
			ui.TextColor(ink),
			ui.When(warm, ui.BackgroundColor("#251b13")),
		)
	})
}
