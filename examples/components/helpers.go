package main

import (
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

type palette struct {
	Window, Sidebar, Panel, PanelAlt, Control, ControlHover, ControlActive string
	Ink, Muted, Faint, Border, Accent, AccentHover, OnAccent, Selection    string
	Popup, Backdrop, Track, Danger, DangerHover                            string
}

var lightPalette = palette{"#f4f5f7", "#ebedf1", "#ffffff", "#f8f9fb", "#ffffff", "#eef1f6", "#e2e8f4", "#131820", "#5a6472", "#8b94a3", "#d9dde4", "#2563eb", "#1d4fd7", "#ffffff", "#dbe6fd", "#ffffff", "#1e293b66", "#e4e7ec", "#b42318", "#9a1d14"}
var darkPalette = palette{"#0e1117", "#131924", "#171e2a", "#1b2331", "#1c2432", "#263042", "#2f3a50", "#e7edf7", "#98a4b6", "#6e7a8c", "#2a3446", "#5b93f7", "#7aa7ff", "#08111f", "#1f2d47", "#171e2a", "#010409aa", "#252f41", "#f0736a", "#f58c84"}

type galleryTheme struct {
	Appearance func() string
	Size       func() native.Point
}

var galleryContext = reactive.CreateContext(galleryTheme{Appearance: func() string { return "light" }, Size: func() native.Point {
	return native.Point{
		X: 1080,
		Y: 780,
	}
}})

func p() palette {
	return choose(galleryContext.Use().Appearance() == "dark", darkPalette, lightPalette)
}
func color(read func(palette) string) func() string { return func() string { return read(p()) } }
func ptr[T any](value T) *T                         { return &value }
func choose[T any](condition bool, yes, no T) T {
	if condition {
		return yes
	}
	return no
}
func change[T any](write func(T)) func(T, *native.Event) {
	return func(value T, _ *native.Event) { write(value) }
}
func textValue[T any](value *T) string {
	if value == nil {
		return "—"
	}
	return fmt.Sprint(*value)
}

func controlStyle() ui.Style {
	return ui.Style{
		Display:         "flex",
		FlexDirection:   "row",
		AlignItems:      "center",
		JustifyContent:  "center",
		Gap:             6,
		Height:          30,
		FlexShrink:      0,
		PaddingLeft:     12,
		PaddingRight:    12,
		BorderRadius:    8,
		BackgroundColor: color(func(p palette) string { return p.Control }),
		BorderColor:     color(func(p palette) string { return p.Border }),
		BorderWidth:     1,
		Color:           color(func(p palette) string { return p.Ink }),
		FontSize:        13,
		Cursor:          "default",
		UserSelect:      "none",
		AppRegion:       "no-drag",
		Hover:           &ui.Style{BackgroundColor: color(func(p palette) string { return p.ControlHover })},
		Focus: &ui.Style{
			OutlineWidth: 2,
			OutlineColor: color(func(p palette) string { return p.Accent }),
		},
		OutlineOffset: 2,
		Disabled:      &ui.Style{Opacity: 0.45},
	}
}
func inputStyle() ui.Style {
	return ui.Style{
		Height:          30,
		PaddingLeft:     10,
		PaddingRight:    10,
		BorderRadius:    8,
		BackgroundColor: color(func(p palette) string { return p.PanelAlt }),
		BorderColor:     color(func(p palette) string { return p.Border }),
		BorderWidth:     1,
		Color:           color(func(p palette) string { return p.Ink }),
		FontSize:        13,
		Focus: &ui.Style{
			OutlineWidth: 2,
			OutlineColor: color(func(p palette) string { return p.Accent }),
		},
		OutlineOffset: 2,
	}
}
func popupStyle() ui.Style {
	return ui.Style{
		Display:         "flex",
		FlexDirection:   "column",
		Gap:             8,
		Padding:         14,
		BorderRadius:    12,
		BackgroundColor: color(func(p palette) string { return p.Popup }),
		BorderColor:     color(func(p palette) string { return p.Border }),
		BorderWidth:     1,
		Color:           color(func(p palette) string { return p.Ink }),
		BoxShadow:       "0 18px 40px #00000033",
	}
}
func fillStyle() ui.Style {
	return ui.Style{Position: "absolute", Top: 0, Right: 0, Bottom: 0, Left: 0}
}
func control() ui.PartProps { return ui.PartProps{Style: controlStyle()} }
func columnPart() ui.PartProps {
	return ui.PartProps{Style: ui.Style{Display: "flex", FlexDirection: "column", Gap: 8}}
}
func rowPart() ui.PartProps {
	return ui.PartProps{Style: ui.Style{
		Display:       "flex",
		FlexDirection: "row",
		AlignItems:    "center",
		Gap:           10,
	}}
}
func popup(width float64) ui.PartProps {
	s := popupStyle()
	s.Width = width
	return ui.PartProps{Style: s}
}
func backdrop() ui.PartProps {
	s := fillStyle()
	s.BackgroundColor = color(func(p palette) string { return p.Backdrop })
	return ui.PartProps{Style: s}
}
func overlay() ui.PartProps {
	s := fillStyle()
	s.Display = "flex"
	s.AlignItems = "center"
	s.JustifyContent = "center"
	return ui.PartProps{Style: s}
}
func panel(title, hint string, children func()) {
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(14),
		ui.Padding(20),
		ui.BorderRadius(12),
		ui.BorderWidth(1),
		ui.BorderColor(color(func(p palette) string { return p.Border })),
		ui.BackgroundColor(color(func(p palette) string { return p.Panel })),
		ui.FlexShrink(0),
		func() {
			ui.Text(ui.FontSize(17), ui.FontWeight(700), title)
			ui.Text(
				ui.FontSize(12),
				ui.LineHeight(18),
				ui.Color(color(func(p palette) string { return p.Muted })),
				hint,
			)
			children()
		},
	)
}
func row(children func()) {
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("row"),
		ui.AlignItems("center"),
		ui.FlexWrap("wrap"),
		ui.Gap(10),
		children,
	)
}
func col(children func()) {
	ui.View(ui.Display("flex"), ui.FlexDirection("column"), ui.Gap(8), children)
}
func note(value any) {
	ui.Text(
		ui.FontSize(12),
		ui.LineHeight(18),
		ui.FontFamily("monospace"),
		ui.Color(color(func(p palette) string { return p.Muted })),
		value,
	)
}
func label(value any) { ui.Text(ui.FontSize(12), value) }
func muted(value any) {
	ui.Text(
		ui.FontSize(12),
		ui.LineHeight(17),
		ui.Color(color(func(p palette) string { return p.Muted })),
		value,
	)
}
func button(label any, click func(), options ...any) {
	args := []any{ui.WithStyle(controlStyle()), ui.OnClick(click)}
	args = append(args, options...)
	args = append(args, label)
	ui.Button(args...)
}
func primary(label any, click func()) {
	button(label, click, ui.BackgroundColor(color(func(p palette) string { return p.Accent })), ui.Color(color(func(p palette) string { return p.OnAccent })), ui.Hover(ui.BackgroundColor(color(func(p palette) string { return p.AccentHover }))))
}
func input(value any, set func(string), placeholder string, options ...any) {
	args := []any{ui.WithStyle(inputStyle()), ui.Value(value), ui.Placeholder(placeholder), ui.OnInput(func(e *native.Event) { set(e.Value) })}
	args = append(args, options...)
	ui.Input(args...)
}
func checkboxStyle() ui.Style {
	s := controlStyle()
	s.Height = 28
	s.BorderWidth = 0
	s.BackgroundColor = "transparent"
	s.JustifyContent = "flex-start"
	s.PaddingLeft = 8
	return s
}
func checkbox(props ui.CheckboxProps, caption any) {
	if props.Style == nil {
		props.Style = checkboxStyle()
	}
	ui.Checkbox.Root(
		props,
		func() {
			ui.Checkbox.Indicator(
				ui.PartProps{Style: ui.Style{
					Width:           16,
					Height:          16,
					BorderRadius:    5,
					BorderWidth:     1,
					BorderColor:     color(func(p palette) string { return p.Accent }),
					BackgroundColor: color(func(p palette) string { return p.Accent }),
					Color:           color(func(p palette) string { return p.OnAccent }),
					Display:         "flex",
					AlignItems:      "center",
					JustifyContent:  "center",
					FontSize:        11,
				}},
				"✓",
			)
			label(caption)
		},
	)
}
