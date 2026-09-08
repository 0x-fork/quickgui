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
	return ui.Styles(
		ui.Display("flex"),
		ui.FlexDirection("row"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Gap(6),
		ui.Height(30),
		ui.FlexShrink(0),
		ui.PaddingLeft(12),
		ui.PaddingRight(12),
		ui.BorderRadius(8),
		ui.BackgroundColor(color(func(p palette) string { return p.Control })),
		ui.BorderColor(color(func(p palette) string { return p.Border })),
		ui.BorderWidth(1),
		ui.TextColor(color(func(p palette) string { return p.Ink })),
		ui.FontSize(13),
		ui.Cursor("default"),
		ui.UserSelect("none"),
		ui.AppRegion("no-drag"),
		ui.Hover(ui.BackgroundColor(color(func(p palette) string { return p.ControlHover }))),
		ui.Focus(
			ui.OutlineWidth(2),
			ui.OutlineColor(color(func(p palette) string { return p.Accent })),
		),
		ui.OutlineOffset(2),
		ui.DisabledStyle(ui.Opacity(0.45)),
	)
}
func inputStyle() ui.Style {
	return ui.Styles(
		ui.Height(30),
		ui.PaddingLeft(10),
		ui.PaddingRight(10),
		ui.BorderRadius(8),
		ui.BackgroundColor(color(func(p palette) string { return p.PanelAlt })),
		ui.BorderColor(color(func(p palette) string { return p.Border })),
		ui.BorderWidth(1),
		ui.TextColor(color(func(p palette) string { return p.Ink })),
		ui.FontSize(13),
		ui.Focus(
			ui.OutlineWidth(2),
			ui.OutlineColor(color(func(p palette) string { return p.Accent })),
		),
		ui.OutlineOffset(2),
	)
}
func popupStyle() ui.Style {
	return ui.Styles(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(8),
		ui.Padding(14),
		ui.BorderRadius(12),
		ui.BackgroundColor(color(func(p palette) string { return p.Popup })),
		ui.BorderColor(color(func(p palette) string { return p.Border })),
		ui.BorderWidth(1),
		ui.TextColor(color(func(p palette) string { return p.Ink })),
		ui.BoxShadow("0 18px 40px #00000033"),
	)
}
func fillStyle() ui.Style {
	return ui.Styles(ui.Position("absolute"), ui.Top(0), ui.Right(0), ui.Bottom(0), ui.Left(0))
}
func control() ui.PartProps { return ui.PartProps{Style: controlStyle()} }
func columnPart() ui.PartProps {
	return ui.PartProps{Style: ui.Styles(ui.Display("flex"), ui.FlexDirection("column"), ui.Gap(8))}
}
func rowPart() ui.PartProps {
	return ui.PartProps{Style: ui.Styles(
		ui.Display("flex"),
		ui.FlexDirection("row"),
		ui.AlignItems("center"),
		ui.Gap(10),
	)}
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
		func() {
			ui.Text(title, ui.FontSize(17), ui.FontWeight(700))
			ui.Text(
				hint,
				ui.FontSize(12),
				ui.LineHeight(18),
				ui.TextColor(color(func(p palette) string { return p.Muted })),
			)
			children()
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Gap(14),
		ui.Padding(20),
		ui.BorderRadius(12),
		ui.BorderWidth(1),
		ui.BorderColor(color(func(p palette) string { return p.Border })),
		ui.BackgroundColor(color(func(p palette) string { return p.Panel })),
		ui.FlexShrink(0),
	)
}
func row(children func()) {
	ui.View(
		children,
		ui.Display("flex"),
		ui.FlexDirection("row"),
		ui.AlignItems("center"),
		ui.FlexWrap("wrap"),
		ui.Gap(10),
	)
}
func col(children func()) {
	ui.View(children, ui.Display("flex"), ui.FlexDirection("column"), ui.Gap(8))
}
func note(value any) {
	ui.Text(
		value,
		ui.FontSize(12),
		ui.LineHeight(18),
		ui.FontFamily("monospace"),
		ui.TextColor(color(func(p palette) string { return p.Muted })),
	)
}
func label(value any) { ui.Text(value, ui.FontSize(12)) }
func muted(value any) {
	ui.Text(
		value,
		ui.FontSize(12),
		ui.LineHeight(17),
		ui.TextColor(color(func(p palette) string { return p.Muted })),
	)
}
func button(label any, click func(), options ...any) {
	args := []any{controlStyle(), ui.OnClick(click)}
	args = append(args, options...)
	args = append(args, label)
	ui.Button(args...)
}
func primary(label any, click func()) {
	button(label, click, ui.Styles(
		ui.BackgroundColor(color(func(p palette) string { return p.Accent })),
		ui.TextColor(color(func(p palette) string { return p.OnAccent })),
		ui.Hover(ui.BackgroundColor(color(func(p palette) string { return p.AccentHover }))),
	))
}
func input(value any, set func(string), placeholder string, options ...any) {
	args := []any{inputStyle(), ui.Value(value), ui.Placeholder(placeholder), ui.OnInput(func(e *native.Event) { set(e.Value) })}
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
	if props.Checked == nil {
		initial := props.DefaultChecked
		if initial == nil {
			initial = false
		}
		checked, setChecked := ui.CreateSignal(initial)
		onChange := props.OnCheckedChange
		props.Checked = checked
		props.OnCheckedChange = func(next bool, event *native.Event) {
			setChecked(next)
			if onChange != nil {
				onChange(next, event)
			}
		}
	}
	ui.Checkbox.Root(
		props,
		func() {
			ui.Checkbox.Indicator(
				ui.PartProps{Style: func() ui.Style {
					border, background := p().Border, p().Control
					if props.Checked() != false {
						border, background = p().Accent, p().Accent
					}
					return ui.Styles(
						ui.Width(16),
						ui.Height(16),
						ui.BorderRadius(5),
						ui.BorderWidth(1),
						ui.BorderColor(border),
						ui.BackgroundColor(background),
						ui.TextColor(p().OnAccent),
						ui.Display("flex"),
						ui.AlignItems("center"),
						ui.JustifyContent("center"),
					)
				}},
				func() {
					ui.Show(
						func() bool { return props.Checked() != false },
						func() {
							ui.SVG(
								ui.Value(func() string {
									path := "M3 6l2 2 4-4"
									if props.Checked() == ui.CheckedIndeterminate {
										path = "M3 6h6"
									}
									return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="` + path + `" fill="none" stroke="` + p().OnAccent + `" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`
								}),
								ui.Width(12),
								ui.Height(12),
								ui.FlexShrink(0),
							)
						},
					)
				},
			)
			label(caption)
		},
	)
}

func checkboxSelectionState(checked, total int) ui.CheckedState {
	if checked == 0 {
		return false
	}
	if checked == total {
		return true
	}
	return ui.CheckedIndeterminate
}
