package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:             "Sidebar vibrancy",
			Width:             960,
			Height:            720,
			Vibrancy:          "sidebar",
			VisualEffectState: "followWindow",
			TitleBarStyle:     "hiddenInset",
			Component:         Vibrancy,
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func Vibrancy() {
	window := native.CurrentWindow()
	material, setMaterial := ui.CreateSignal("sidebar")
	state, setState := ui.CreateSignal("followWindow")
	var materials, states []any
	materials = append(materials, ui.Text(ui.FontSize(18), "Materials"))
	for _, value := range []string{"titlebar", "selection", "menu", "popover", "sidebar", "header", "sheet", "window", "hud", "fullscreen-ui", "tooltip", "content", "under-window", "under-page"} {
		var node *native.Node
		button(value, func() { setMaterial(value); window.Action("set-vibrancy", value) }, ui.Ref(func(ref *native.Node) {
			node = ref
		}))
		node.Bind(func() {
			color := uint32(0)
			if material() == value {
				color = native.ParseColor("#2563eb")
			}
			native.SetColor(node, protocol.BackgroundColor, color)
		})
		materials = append(materials, node)
	}
	for _, value := range []string{"followWindow", "active", "inactive"} {
		states = append(states, func() {
			button(value, func() { setState(value); window.Action("set-visual-effect-state", value) })
		})
	}
	ui.View(
		ui.Display("flex"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.Color("#e2e8f0"),
		func() {
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Gap(5),
				ui.Padding(20),
				ui.PaddingTop(48),
				ui.Width(240),
				ui.Height("100%"),
				ui.OverflowY("scroll"),
				materials,
			)
			ui.View(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Flex(1),
				ui.Gap(20),
				ui.Padding(36),
				ui.PaddingTop(70),
				ui.BackgroundColor("#0b1020"),
				func() {
					ui.Text(ui.FontSize(30), "Native vibrancy")
					ui.Text(func() string { return material() + " · " + state() })
					ui.View(ui.Display("flex"), ui.Gap(8), states)
					ui.Text("Move the window over other content to inspect the macOS material.")
				},
			)
		},
	)
}
