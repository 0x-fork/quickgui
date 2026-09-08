package main

import "github.com/egoist/quickgui/go/ui"

const (
	background  = "#0b1020"
	panel       = "#11182b"
	panelRaised = "#18223a"
	border      = "#26344f"
	textColor   = "#e8edf7"
	muted       = "#8e9bb4"
	blue        = "#6ea8fe"
	blueSurface = "#1d3b68"
)

var linkStyle = ui.Styles(
	ui.Display("flex"),
	ui.Height(34),
	ui.AlignItems("center"),
	ui.PaddingLeft(12),
	ui.PaddingRight(12),
	ui.BorderRadius(8),
	ui.Color(muted),
	ui.Cursor("default"),
	ui.UserSelect("none"),
	ui.Hover(ui.BackgroundColor(panelRaised)),
)
var activeLinkStyle = ui.Styles(ui.BackgroundColor(blueSurface), ui.Color("#dceaff"))
var buttonStyle = ui.Styles(
	ui.Display("flex"),
	ui.Height(34),
	ui.AlignItems("center"),
	ui.JustifyContent("center"),
	ui.PaddingLeft(12),
	ui.PaddingRight(12),
	ui.BackgroundColor(panelRaised),
	ui.Color(textColor),
	ui.BorderRadius(8),
	ui.Cursor("default"),
	ui.UserSelect("none"),
	ui.AppRegion("no-drag"),
	ui.Hover(ui.BackgroundColor(blueSurface)),
	ui.DisabledStyle(ui.Opacity(0.35)),
)

func link(label, href string, end bool) {
	ui.Link(
		ui.LinkProps{
			Href:        href,
			End:         end,
			PartProps:   ui.PartProps{Style: linkStyle},
			ActiveStyle: &activeLinkStyle,
		},
		label,
	)
}

func button(label string, click func(), options ...any) {
	args := []any{label, buttonStyle, ui.OnClick(click)}
	ui.Button(append(args, options...)...)
}

func historyButton(label, path string, click func(), disabled func() bool) {
	ui.Button(
		func() {
			ui.SVG(
				ui.Value(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#e8edf7" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="`+path+`"/></svg>`),
				ui.Width(17),
				ui.Height(17),
			)
		},
		buttonStyle,
		ui.Width(34),
		ui.Height(30),
		ui.PaddingLeft(0),
		ui.PaddingRight(0),
		ui.BorderRadius(7),
		ui.AriaLabel(label),
		ui.Disabled(disabled),
		ui.OnClick(click),
	)
}

func page(title, description any, children ...any) {
	ui.View(
		func() {
			ui.Text(title, ui.FontSize(28), ui.LineHeight(36), ui.FontWeight(750))
			ui.Text(description, ui.MaxWidth(620), ui.Color(muted), ui.LineHeight(21))
			ui.Child(children)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.Padding(28),
		ui.Gap(16),
		ui.OverflowY("auto"),
	)
}

func card(title, detail, href string) {
	active := ui.Styles(ui.BorderColor(blue))
	ui.Link(
		ui.LinkProps{
			Href:        href,
			ActiveStyle: &active,
			PartProps: ui.PartProps{Style: ui.Styles(
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Width(260),
				ui.MinHeight(112),
				ui.Padding(18),
				ui.Gap(8),
				ui.BackgroundColor(panel),
				ui.BorderWidth(1),
				ui.BorderColor(border),
				ui.BorderRadius(12),
				ui.Color(textColor),
				ui.Cursor("default"),
				ui.Hover(ui.BackgroundColor(panelRaised)),
			)},
		},
		func() {
			ui.Text(title, ui.FontWeight(700))
			ui.Text(detail, ui.Color(muted), ui.LineHeight(19))
		},
	)
}
