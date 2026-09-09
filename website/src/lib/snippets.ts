export const snippets = {
  counter: {
    lang: "go",
    code: `func Counter() *ui.Element {
	count, setCount := ui.CreateSignal(0)
	return ui.View(
		ui.Text("Count: ", count()),
		ui.Button("Increment").
			OnClick(func() { setCount(count() + 1) }).
			Padding(12).
			RoundedLg().
			Bg("#2563eb"),
	).FlexCol().
		SizeFull().
		ItemsCenter().
		JustifyCenter().
		Gap(20).
		Bg("#090d16").
		TextColor("#e2e8f0")
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
    .rounded_lg()
    .bg(@ui.rgb8(37, 99, 235)),
  ])
  .flex_col()
  .size_full()
  .items_center()
  .justify_center()
  .gap(20)
  .bg(@ui.rgb8(9, 13, 22))
  .text_color(@ui.rgb8(226, 232, 240))
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
	func() *native.Node {
		return ui.SwiftUI.Button(ui.SwiftUIButtonProps{
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
