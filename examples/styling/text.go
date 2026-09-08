package main

import "github.com/egoist/quickgui/go/ui"

// The core resolves start and end against the inherited text direction.
func TextAlignment() {
	Panel("Text alignment", func() {
		for _, align := range []string{"left", "center", "right", "justify", "start", "end"} {
			ui.View(
				func() {
					ui.Text(
						"textAlign: \""+align+"\" — the core resolves start and end against the inherited direction.",
						ui.Width("100%"),
						ui.TextAlign(align),
						ui.FontSize(13),
						ui.TextColor(ink),
					)
				},
				ui.Width("100%"),
				ui.Padding(8),
				ui.BorderRadius(8),
				ui.BackgroundColor("#1b2434"),
			)
		}
	})
}

func TextStyling() {
	Panel("Extended text styling", func() {
		ui.Text(
			"letterSpacing 2",
			ui.FontSize(20),
			ui.FontWeight(700),
			ui.LetterSpacing(2),
			ui.TextColor(ink),
		)
		ui.Text(
			"wordSpacing 8 pushes every space apart",
			ui.FontSize(14),
			ui.WordSpacing(8),
			ui.TextColor(ink),
		)
		ui.Text(
			"textTransform capitalize keeps selection on the original text",
			ui.FontSize(14),
			ui.TextTransform("capitalize"),
			ui.TextColor(ink),
		)
		ui.Text(
			"textShadow",
			ui.FontSize(24),
			ui.FontWeight(700),
			ui.TextColor("#f8fafc"),
			ui.TextShadow("0 3px 10px #38bdf8aa"),
		)
		ui.Text(
			"wavy underline in its own color",
			ui.FontSize(14),
			ui.TextColor(ink),
			ui.TextDecoration("underline"),
			ui.TextDecorationColor("#f97316"),
			ui.TextDecorationStyle("wavy"),
			ui.TextDecorationThickness(2),
		)
		ui.Text(
			"line-through and overline together",
			ui.FontSize(14),
			ui.TextColor(ink),
			ui.TextDecoration("line-through overline"),
			ui.TextDecorationColor("#f43f5e"),
		)
		ui.View(
			func() {
				ui.Text(
					"wordBreak break-all with overflowWrap anywhere: supercalifragilisticexpialidocious",
					ui.Width("100%"),
					ui.FontSize(13),
					ui.TextColor(muted),
					ui.WordBreak("break-all"),
					ui.OverflowWrap("anywhere"),
					ui.Hyphens("manual"),
				)
			},
			ui.Width(200),
			ui.Padding(8),
			ui.BorderRadius(8),
			ui.BackgroundColor("#1b2434"),
		)
	})
}

func Direction() {
	Panel("Right to left", func() {
		ui.View(
			func() {
				ui.View(
					ui.Width(28),
					ui.Height(20),
					ui.BorderRadius(6),
					ui.BackgroundColor("#38bdf8"),
				)
				ui.View(
					ui.Width(20),
					ui.Height(20),
					ui.BorderRadius(6),
					ui.BackgroundColor("#334155"),
				)
				ui.Text(
					"مرحبا بالعالم — hello",
					ui.FontSize(13),
					ui.TextColor(ink),
					ui.TextAlign("start"),
					ui.TextDirection("rtl"),
				)
			},
			ui.Direction("rtl"),
			ui.Display("flex"),
			ui.Gap(8),
			ui.AlignItems("center"),
			ui.Padding(10),
			ui.PaddingStart(20),
			ui.BorderRadius(10),
			ui.BorderStartWidth(3),
			ui.BorderColor("#38bdf8"),
			ui.BackgroundColor("#1b2434"),
		)
		ui.Text(
			"paddingStart and borderStartWidth resolve to the right edge inside this subtree.",
			ui.FontSize(12),
			ui.TextColor(muted),
		)
	})
}
