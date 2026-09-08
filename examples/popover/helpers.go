package main

import "github.com/egoist/quickgui/go/ui"

var buttonStyle = ui.Styles(
	ui.Display("flex"),
	ui.Width("100%"),
	ui.Height(42),
	ui.AlignItems("center"),
	ui.JustifyContent("center"),
	ui.PaddingLeft(16),
	ui.PaddingRight(16),
	ui.BackgroundColor("#2563eb"),
	ui.TextColor("#ffffff"),
	ui.BorderRadius(9),
	ui.Cursor("default"),
	ui.AppRegion("no-drag"),
	ui.UserSelect("none"),
	ui.Hover(ui.BackgroundColor("#3b82f6")),
)

func card(title, description string, children func()) {
	ui.View(
		func() {
			ui.Text(title, ui.FontSize(17), ui.FontWeight(700))
			ui.Text(description, ui.TextColor("#9ba8bc"), ui.FontSize(13), ui.LineHeight(19))
			children()
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Flex(1),
		ui.MinWidth(0),
		ui.Gap(14),
		ui.Padding(20),
		ui.BackgroundColor("#151a23"),
		ui.BorderColor("#30394a"),
		ui.BorderWidth(1),
		ui.BorderRadius(12),
	)
}

func content(kind, description string, close func()) {
	count, setCount := ui.CreateSignal(0)
	actionStyle := ui.Styles(
		ui.BackgroundColor("#30394a"),
		ui.BorderColor("#465166"),
		ui.BorderWidth(1),
		ui.Flex(1),
		ui.Width(0),
		ui.Hover(ui.BackgroundColor("#465166")),
	)
	ui.View(
		func() {
			ui.View(
				func() {
					ui.Text(kind, ui.TextColor("#93c5fd"), ui.FontSize(12), ui.FontWeight(700))
					ui.Text(
						"Interactive popover content",
						ui.FontSize(19),
						ui.LineHeight(24),
						ui.FontWeight(700),
					)
					ui.Text(
						description,
						ui.TextColor("#aeb8c9"),
						ui.FontSize(13),
						ui.LineHeight(19),
					)
				},
				ui.Display("flex"),
				ui.FlexDirection("column"),
				ui.Gap(6),
			)
			ui.View(
				func() {
					ui.Button(
						func() { ui.Text("Count: ", count) },
						buttonStyle,
						actionStyle,
						ui.OnClick(func() { setCount(count() + 1) }),
					)
					ui.Button("Close", buttonStyle, actionStyle, ui.OnClick(close))
				},
				ui.Display("flex"),
				ui.Gap(10),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Width("100%"),
		ui.Height("100%"),
		ui.Gap(14),
		ui.Padding(20),
		ui.BackgroundColor("#151a23"),
		ui.TextColor("#f5f7fb"),
		ui.BorderColor("#3b4558"),
		ui.BorderWidth(1),
		ui.BorderRadius(12),
	)
}
