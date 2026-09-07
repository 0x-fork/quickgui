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

var linkStyle = ui.Style{
	Display:      "flex",
	Height:       34,
	AlignItems:   "center",
	PaddingLeft:  12,
	PaddingRight: 12,
	BorderRadius: 8,
	Color:        muted,
	Cursor:       "default",
	UserSelect:   "none",
	Hover:        &ui.Style{BackgroundColor: panelRaised},
}
var activeLinkStyle = ui.Style{BackgroundColor: blueSurface, Color: "#dceaff"}
var buttonStyle = ui.Style{
	Display:         "flex",
	Height:          34,
	AlignItems:      "center",
	JustifyContent:  "center",
	PaddingLeft:     12,
	PaddingRight:    12,
	BackgroundColor: panelRaised,
	Color:           textColor,
	BorderRadius:    8,
	Cursor:          "default",
	UserSelect:      "none",
	AppRegion:       "no-drag",
	Hover:           &ui.Style{BackgroundColor: blueSurface},
	Disabled:        &ui.Style{Opacity: 0.35},
}

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
				ui.Style{
					Width:  17,
					Height: 17,
				},
			)
		},
		buttonStyle,
		ui.Style{Width: 34, Height: 30, PaddingLeft: 0, PaddingRight: 0, BorderRadius: 7},
		ui.AriaLabel(label),
		ui.Disabled(disabled),
		ui.OnClick(click),
	)
}

func page(title, description any, children ...any) {
	ui.View(
		func() {
			ui.Text(title, ui.Style{FontSize: 28, LineHeight: 36, FontWeight: 750})
			ui.Text(description, ui.Style{MaxWidth: 620, Color: muted, LineHeight: 21})
			ui.Child(children)
		},
		ui.Style{
			Display:       "flex",
			FlexDirection: "column",
			Width:         "100%",
			Height:        "100%",
			Padding:       28,
			Gap:           16,
			OverflowY:     "auto",
		},
	)
}

func card(title, detail, href string) {
	ui.Link(
		ui.LinkProps{
			Href:        href,
			ActiveStyle: &ui.Style{BorderColor: blue},
			PartProps: ui.PartProps{Style: ui.Style{
				Display:         "flex",
				FlexDirection:   "column",
				Width:           260,
				MinHeight:       112,
				Padding:         18,
				Gap:             8,
				BackgroundColor: panel,
				BorderWidth:     1,
				BorderColor:     border,
				BorderRadius:    12,
				Color:           textColor,
				Cursor:          "default",
				Hover:           &ui.Style{BackgroundColor: panelRaised},
			}},
		},
		func() {
			ui.Text(title, ui.Style{FontWeight: 700})
			ui.Text(detail, ui.Style{Color: muted, LineHeight: 19})
		},
	)
}
