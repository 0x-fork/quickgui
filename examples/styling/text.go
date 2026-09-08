package main

import "github.com/egoist/quickgui/go/ui"

// The core resolves start and end against the inherited text direction.
func TextAlignment() {
	Panel("Text alignment", func() {
		for _, align := range []string{"left", "center", "right", "justify", "start", "end"} {
			ui.View(
				func() {
					ui.Text(
						"textAlign: \"" + align + "\" — the core resolves start and end against the inherited direction.",
					).Width("100%").TextAlign(align).FontSize(13).TextColor(ink)
				},
			).Width("100%").Padding(8).BorderRadius(8).BackgroundColor("#1b2434")
		}
	})
}

func TextStyling() {
	Panel("Extended text styling", func() {
		ui.Text(
			"letterSpacing 2",
		).FontSize(20).FontWeight(700).LetterSpacing(2).TextColor(ink)
		ui.Text(
			"wordSpacing 8 pushes every space apart",
		).FontSize(14).WordSpacing(8).TextColor(ink)
		ui.Text(
			"textTransform capitalize keeps selection on the original text",
		).FontSize(14).TextTransform("capitalize").TextColor(ink)
		ui.Text(
			"textShadow",
		).FontSize(24).FontWeight(700).TextColor("#f8fafc").TextShadow("0 3px 10px #38bdf8aa")
		ui.Text(
			"wavy underline in its own color",
		).FontSize(14).TextColor(ink).TextDecoration("underline").TextDecorationColor("#f97316").TextDecorationStyle("wavy").TextDecorationThickness(2)
		ui.Text(
			"line-through and overline together",
		).FontSize(14).TextColor(ink).TextDecoration("line-through overline").TextDecorationColor("#f43f5e")
		ui.View(
			func() {
				ui.Text(
					"wordBreak break-all with overflowWrap anywhere: supercalifragilisticexpialidocious",
				).Width("100%").FontSize(13).TextColor(muted).WordBreak("break-all").OverflowWrap("anywhere").Hyphens("manual")
			},
		).Width(200).Padding(8).BorderRadius(8).BackgroundColor("#1b2434")
	})
}

func Direction() {
	Panel("Right to left", func() {
		ui.View(
			func() {
				ui.View().Width(28).Height(20).BorderRadius(6).BackgroundColor("#38bdf8")
				ui.View().Width(20).Height(20).BorderRadius(6).BackgroundColor("#334155")
				ui.Text(
					"مرحبا بالعالم — hello",
				).FontSize(13).TextColor(ink).TextAlign("start").TextDirection("rtl")
			},
		).Direction("rtl").Display("flex").Gap(8).AlignItems("center").Padding(10).PaddingStart(20).BorderRadius(10).BorderStartWidth(3).BorderColor("#38bdf8").BackgroundColor("#1b2434")
		ui.Text(
			"paddingStart and borderStartWidth resolve to the right edge inside this subtree.",
		).FontSize(12).TextColor(muted)
	})
}
