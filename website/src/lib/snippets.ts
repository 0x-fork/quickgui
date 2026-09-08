export const snippets = {
  counter: {
    lang: "go",
    code: `func Counter() {
	count, setCount := ui.CreateSignal(0)
	ui.View(
		func() {
			ui.Text("Count: ", count)
			ui.Button(
				"Increment",
				ui.Padding(12),
				ui.BorderRadius(8),
				ui.BackgroundColor("#18181b"),
				ui.TextColor("white"),
				ui.OnClick(func() { setCount(count() + 1) }),
			)
		},
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Height("100%"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Gap(12),
	)
}
`,
  },
  moonbitCounter: {
    lang: "moonbit",
    code: `fn counter() -> @ui.Element {
  let (count, set_count) = @ui.create_signal(0)
  @ui.div([
    @ui.text("Count: \\{count()}"),
    @ui.button("Increment")
    .on_click(() => set_count(count() + 1))
    .padding(12)
    .border_radius(8)
    .bg(@ui.rgb8(24, 24, 27))
    .text_color(@ui.rgb8(255, 255, 255)),
  ])
  .size_full()
  .flex_col()
  .items_center()
  .justify_center()
  .gap(12)
}
`,
  },
  window: {
    lang: "go",
    code: `package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
)

func main() {
	if err := native.Run(func() {
		openWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openWindow()
			}
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func openWindow() {
	native.NewWindow(native.WindowOptions{
		Title:     "Counter",
		Width:     720,
		Height:    480,
		Component: Counter,
	})
}
`,
  },
  swiftUi: {
    lang: "go",
    code: `ui.SwiftUI.Host(
	ui.SwiftUIHostProps{MatchContents: true},
	func() {
		ui.SwiftUI.Button(ui.SwiftUIButtonProps{
			Label:       "Save changes",
			SystemImage: "checkmark",
			Modifiers: []ui.SwiftUIModifier{
				ui.SwiftUI.ButtonStyle("glass"),
			},
		})
	},
)
`,
  },
  moonbitSwiftUi: {
    lang: "moonbit",
    code: `@ui.swift_ui_host([
  @ui.swift_ui_button()
  .label("Save changes")
  .swift_ui_system_image("checkmark"),
])`,
  },
  cliInit: {
    lang: "bash",
    code: `bunx @quickgui/cli init my-app
cd my-app
bun run dev`,
  },
  cliFormat: {
    lang: "bash",
    code: `bun run fmt
bun run build`,
  },
  moonbitCliInit: {
    lang: "bash",
    code: `bunx @quickgui/cli init my-app \\
  --frontend moonbit
cd my-app
bun run dev`,
  },
  moonbitCliFormat: {
    lang: "bash",
    code: `moon fmt
bun run build`,
  },
  cliBuild: {
    lang: "bash",
    code: `bun run build --target darwin-arm64 \\
  --sign "Developer ID Application: Example (TEAMID)" \\
  --notarize quickgui-notary`,
  },
} as const;

export type SnippetKey = keyof typeof snippets;
export type HighlightedSnippets = Record<SnippetKey, string>;
