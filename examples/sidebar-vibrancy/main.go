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
				Title:                "QuickGUI Sidebar Vibrancy",
				Width:                860,
				Height:               620,
				MinimumWidth:         700,
				MinimumHeight:        480,
				Background:           "transparent",
				Vibrancy:             "sidebar",
				VisualEffectState:    "followWindow",
				Appearance:           "light",
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 19},
				Component:            Vibrancy,
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

func Vibrancy() {
	window := native.CurrentWindow()
	material, setMaterial := ui.CreateSignal("sidebar")
	state, setState := ui.CreateSignal("followWindow")
	ui.View(
		func() {
			ui.View(
				func() {
					ui.View(
						func() {
							ui.Text("Vibrancy", ui.FontSize(13), ui.FontWeight(700))
						},
						ui.Display("flex"),
						ui.Height(52),
						ui.FlexShrink(0),
						ui.AlignItems("center"),
						ui.PaddingLeft(90),
						ui.PaddingRight(14),
						ui.AppRegion("drag"),
					)
					ui.View(
						func() {
							for _, option := range materials {
								selected := func() bool { return material() == option.value }
								ui.Button(
									func() {
										ui.Text(
											option.label,
											ui.FontSize(12),
											ui.FontWeight(600),
										)
										ui.Show(selected, checkmark)
									},
									ui.Display("flex"),
									ui.Width("100%"),
									ui.Height(31),
									ui.FlexShrink(0),
									ui.AlignItems("center"),
									ui.JustifyContent("space-between"),
									ui.PaddingLeft(11),
									ui.PaddingRight(11),
									ui.BackgroundColor("transparent"),
									ui.BorderColor("transparent"),
									ui.BorderWidth(1),
									ui.BorderRadius(7),
									ui.TextColor("#263247"),
									ui.Cursor("default"),
									ui.AppRegion("no-drag"),
									ui.UserSelect("none"),
									ui.When(
										selected,
										ui.BackgroundColor("#ffffff52"),
										ui.BorderColor("#ffffff70"),
									),
									ui.Selected(selected),
									ui.OnClick(func() {
										setMaterial(option.value)
										window.SetVibrancy(option.value)
									}),
								)
							}
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.Flex(1),
						ui.MinHeight(0),
						ui.Gap(3),
						ui.PaddingLeft(10),
						ui.PaddingRight(10),
						ui.PaddingBottom(10),
						ui.OverflowY("auto"),
					)
					ui.View(
						func() {
							ui.Text(
								"EFFECT STATE",
								ui.TextColor("#59667b"),
								ui.FontSize(11),
								ui.FontWeight(700),
							)
							ui.View(
								func() {
									for _, option := range effectStates {
										selected := func() bool { return state() == option.value }
										ui.Button(
											option.label,
											ui.Display("flex"),
											ui.Flex(1),
											ui.Height(27),
											ui.MinWidth(0),
											ui.AlignItems("center"),
											ui.JustifyContent("center"),
											ui.BackgroundColor("#ffffff24"),
											ui.BorderColor("#ffffff3d"),
											ui.BorderWidth(1),
											ui.BorderRadius(6),
											ui.TextColor("#445168"),
											ui.FontSize(10),
											ui.FontWeight(600),
											ui.Cursor("default"),
											ui.AppRegion("no-drag"),
											ui.UserSelect("none"),
											ui.When(
												selected,
												ui.BackgroundColor("#ffffff5c"),
												ui.BorderColor("#ffffff7a"),
											),
											ui.Selected(selected),
											ui.OnClick(func() {
												setState(option.value)
												window.SetVisualEffectState(option.value)
											}),
										)
									}
								},
								ui.Display("flex"),
								ui.Gap(5),
							)
						},
						ui.Display("flex"),
						ui.FlexDirection("column"),
						ui.FlexShrink(0),
						ui.Gap(7),
						ui.Padding(12),
						ui.BorderColor("#c1c1c2"),
						ui.BorderTopWidth(1),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width(254),
				ui.Height("100%"),
				ui.FlexShrink(0),
				ui.BackgroundColor("transparent"),
				ui.BorderColor("#cccccc"),
				ui.BorderRightWidth(1),
			)
			ui.View(
				func() {
					ui.View(
						func() {
							ui.Text(material, ui.FontSize(13), ui.FontWeight(700))
						},
						ui.Display("flex"),
						ui.Height(52),
						ui.FlexShrink(0),
						ui.AlignItems("center"),
						ui.JustifyContent("center"),
						ui.BorderColor("#e2e8f0"),
						ui.BorderBottomWidth(1),
						ui.AppRegion("drag"),
					)
					ui.View(
						func() {
							ui.View(
								func() {
									ui.Text(
										"GO + QUICKGUI",
										ui.TextColor("#2563eb"),
										ui.FontSize(12),
										ui.FontWeight(700),
									)
									ui.Text(
										"Every macOS vibrancy type",
										ui.FontSize(26),
										ui.LineHeight(33),
										ui.FontWeight(700),
									)
									ui.Text(
										"Select any Electron-compatible semantic material. QuickGUI updates one native "+
											"NSVisualEffectView while the retained QuickGUI tree and Metal surface stay mounted. "+
											"This pane is opaque, so the selected material remains visually confined to the translucent sidebar.",
										ui.TextColor("#667085"),
										ui.FontSize(14),
										ui.LineHeight(21),
									)
									ui.View(
										func() {
											readout("Material", material)
											readout("Effect state", state)
											readout("Content", "Opaque")
										},
										ui.Display("flex"),
										ui.Gap(10),
										ui.PaddingTop(4),
									)
								},
								ui.Display("flex"),
								ui.FlexDirection("column"),
								ui.Width("100%"),
								ui.MaxWidth(480),
								ui.Gap(16),
								ui.Padding(28),
								ui.BackgroundColor("#ffffff"),
								ui.BorderColor("#dfe5ed"),
								ui.BorderWidth(1),
								ui.BorderRadius(14),
								ui.BoxShadow("0 18px 45px -24px rgba(15, 23, 42, 0.35)"),
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
				ui.Flex(1),
				ui.MinWidth(0),
				ui.Height("100%"),
				ui.BackgroundColor("#ffffff"),
			)
		},
		ui.Display("flex"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.BackgroundColor("transparent"),
		ui.TextColor("#172033"),
	)
}
