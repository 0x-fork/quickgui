package main

import (
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

const (
	ink             = "#e6ecf7"
	muted           = "#94a3b8"
	panelBackground = "#141a26"
	panelBorder     = "#27324a"
)

var warmPalette = reactive.CreateContext[func() bool](func() bool { return false })

var panelStyle = ui.Styles(
	ui.Display("flex"),
	ui.FlexDirection("column"),
	ui.MinWidth(0),
	ui.Gap(12),
	ui.Padding(18),
	ui.BackgroundColor(panelBackground),
	ui.BorderColor(panelBorder),
	ui.BorderWidth(1),
	ui.BorderRadius(14),
)
var captionStyle = ui.Styles(
	ui.Color(muted),
	ui.FontSize(11),
	ui.LetterSpacing(0.8),
	ui.TextTransform("uppercase"),
	ui.FontWeight(700),
)
var centered = ui.Styles(
	ui.Display("flex"),
	ui.AlignItems("center"),
	ui.JustifyContent("center"),
)

func Panel(title string, children func()) {
	warm := warmPalette.Use()
	ui.View(
		func() {
			ui.Text(title, captionStyle)
			children()
		},
		panelStyle,
		ui.When(warm, ui.BackgroundColor("#30231c"), ui.BorderColor("#594338")),
	)
}

func swatch(label string, options ...any) {
	args := []any{func() { ui.Text(label, ui.FontSize(12), ui.Color(ink)) }, centered}
	ui.View(append(args, options...)...)
}

func ptr[T any](value T) *T { return &value }
