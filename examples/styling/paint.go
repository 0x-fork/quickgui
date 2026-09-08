package main

import (
	_ "embed"
	"encoding/base64"

	"github.com/egoist/quickgui/go/ui"
)

// Color filters have a direct raster path; blur and backdrop effects composite
// subtrees. Embed the raster swatch so the example needs no external assets.
//
//go:embed assets/filter-swatch.png
var filterSwatchPNG []byte

var filterSwatch = "data:image/png;base64," + base64.StdEncoding.EncodeToString(filterSwatchPNG)

func Gradients() {
	tile := ui.Styles(ui.Height(64), ui.BorderRadius(12), ui.FontWeight(700))
	Panel("Gradients", func() {
		swatch("linear-gradient", tile, ui.Styles(ui.Background("linear-gradient(135deg, #1d4ed8, #38bdf8 60%, #a855f7)")))
		swatch("radial-gradient", tile, ui.Styles(ui.Background("radial-gradient(circle closest-side at 30% 40%, #f8fafc, #0f172a)")))
		swatch("conic-gradient", tile, ui.Styles(ui.Background("conic-gradient(from 200deg, #f97316, #38bdf8, #f97316)")))
		swatch("object form, oklab", tile, ui.Styles(
			ui.Background(ui.GradientDeclaration{
				Type:          "linear",
				Angle:         ptr(90.0),
				Interpolation: "oklab",
				Stops:         []ui.GradientStop{{Color: "#0ea5e9", Position: ptr(0.0)}, {Color: "#e879f9", Position: ptr(1.0)}},
			}),
		))
	})
}

func BordersAndOutlines() {
	Panel("Corners, borders, and outlines", func() {
		ui.View(
			ui.Height(56),
			ui.BorderRadius("22px 6px 22px 6px"),
			ui.BackgroundColor("#1b2434"),
			ui.BorderWidth(1),
			ui.BorderColor("#38bdf8"),
		)
		ui.View(
			ui.Height(56),
			ui.BorderRadius(12),
			ui.BorderWidth(2),
			ui.BorderColor("#f97316"),
			ui.BorderStyle("dashed"),
		)
		ui.View(
			ui.Height(56),
			ui.BorderRadius(12),
			ui.BorderWidth(2),
			ui.BorderColor("#22c55e"),
			ui.BorderStyle("dotted"),
		)
		ui.View(
			ui.Height(56),
			ui.Margin(6),
			ui.BorderRadius(12),
			ui.BackgroundColor("#1b2434"),
			ui.Outline("2px solid #a855f7"),
			ui.OutlineOffset(4),
		)
		ui.View(
			ui.Height(56),
			ui.Margin(6),
			ui.BorderTopLeftRadius(28),
			ui.BorderBottomRightRadius(28),
			ui.BackgroundColor("#1b2434"),
			ui.Outline("2px dashed #38bdf8"),
			ui.OutlineOffset(3),
		)
	})
}

func Filters() {
	tile := ui.Styles(
		ui.Height(52),
		ui.BorderRadius(10),
		ui.Background("linear-gradient(90deg, #f97316, #38bdf8)"),
		ui.FontWeight(700),
	)
	Panel("Filters and backdrop", func() {
		raster := ui.Styles(
			ui.BackgroundImage(filterSwatch),
			ui.BackgroundSize("cover"),
			ui.BackgroundRepeat("no-repeat"),
		)
		swatch("saturate + contrast", tile, raster, ui.Styles(ui.Filter("saturate(1.8) contrast(1.15)")))
		swatch("grayscale", tile, raster, ui.Styles(ui.Filter("grayscale(1) brightness(1.2)")))
		swatch("hue-rotate", tile, raster, ui.Styles(ui.Filter("hue-rotate(140deg)")))
		swatch("subtree blur", tile, ui.Styles(ui.Filter("blur(2px)")))
		swatch("drop-shadow", tile, ui.Styles(ui.Filter("drop-shadow(0 6px 12px #0b1220)")))
		ui.View(
			func() {
				swatch("backdropFilter", ui.Styles(
					ui.Width("100%"),
					ui.Height("100%"),
					ui.BorderRadius(10),
					ui.BackgroundColor("#0f172a80"),
					ui.BackdropFilter("blur(14px) brightness(1.1)"),
					ui.FontWeight(700),
				))
			},
			ui.Position("relative"),
			ui.Height(84),
			ui.BorderRadius(12),
			ui.Padding(12),
			ui.Background("conic-gradient(from 30deg, #1d4ed8, #f97316, #1d4ed8)"),
		)
	})
}

func Transforms() {
	card := ui.Styles(
		ui.Height(54),
		ui.BorderRadius(12),
		ui.BackgroundColor("#1b2434"),
		ui.BorderColor(panelBorder),
		ui.BorderWidth(1),
	)
	Panel("Transforms and blending", func() {
		swatch("rotate(-3deg)", card, ui.Styles(ui.Transform("rotate(-3deg)")))
		swatch("skew from the left edge", card, ui.Styles(
			ui.Transform("skew(8deg, 0)"),
			ui.TransformOrigin("left center"),
		))
		swatch("hover to lift, glow, and ring", card, ui.Styles(
			ui.Transform("scale(0.96)"),
			ui.OutlineOffset(3),
			ui.Cursor("pointer"),
			ui.Transition(ui.TransitionDeclaration{
				Properties: []string{"background-color"},
				Duration:   "120ms",
				Easing:     "ease-out",
			}),
			ui.Hover(
				ui.Transform("scale(1.03) translate(0, -2px)"),
				ui.Background("linear-gradient(90deg, #1d4ed8, #38bdf8)"),
				ui.Outline("2px solid #93c5fd"),
			),
		))
		ui.View(
			func() {
				for _, tile := range []struct{ color, blend string }{
					{"#f8fafc", "multiply"}, {"#334155", "screen"}, {"#94a3b8", "overlay"},
				} {
					ui.View(
						ui.Flex(1),
						ui.BorderRadius(10),
						ui.BackgroundColor(tile.color),
						ui.MixBlendMode(tile.blend),
					)
				}
			},
			ui.Display("flex"),
			ui.Height(84),
			ui.Gap(10),
			ui.Padding(10),
			ui.BorderRadius(12),
			ui.Background("linear-gradient(90deg, #f97316, #38bdf8)"),
		)
	})
}
