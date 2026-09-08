package main

import "github.com/egoist/quickgui/go/ui"

type choice struct{ value, label string }

var materials = []choice{
	{"appearance-based", "Appearance based"},
	{"titlebar", "Titlebar"},
	{"selection", "Selection"},
	{"menu", "Menu"},
	{"popover", "Popover"},
	{"sidebar", "Sidebar"},
	{"header", "Header"},
	{"sheet", "Sheet"},
	{"window", "Window"},
	{"hud", "HUD"},
	{"fullscreen-ui", "Fullscreen UI"},
	{"tooltip", "Tooltip"},
	{"content", "Content"},
	{"under-window", "Under window"},
	{"under-page", "Under page"},
}

var effectStates = []choice{
	{"followWindow", "Auto"},
	{"active", "Active"},
	{"inactive", "Inactive"},
}

func checkmark() {
	ui.SVG(
		ui.Value(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#2563eb" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4L19 6"/></svg>`),
		ui.Width(14),
		ui.Height(14),
		ui.FlexShrink(0),
		ui.TextColor("#2563eb"),
	)
}

func readout(label string, value any) {
	ui.View(
		func() {
			ui.Text(label, ui.TextColor("#8a94a6"), ui.FontSize(11))
			ui.Text(value, ui.TextColor("#354056"), ui.FontSize(11), ui.FontWeight(700))
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Flex(1),
		ui.MinWidth(0),
		ui.Gap(3),
		ui.Padding(12),
		ui.BackgroundColor("#f8fafc"),
		ui.BorderColor("#e2e8f0"),
		ui.BorderWidth(1),
		ui.BorderRadius(9),
	)
}
