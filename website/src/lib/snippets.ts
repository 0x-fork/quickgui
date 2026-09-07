export const snippets = {
  counter: {
    lang: 'go',
    code: `package main

import (
	"github.com/egoist/quickgui/go/ui"
)

func Counter() {
	count, setCount := ui.CreateSignal(0)
	ui.View(
		func() {
			ui.Text("Count: ", count)
			ui.Button(
				"Increment",
				ui.Style{Padding: 12, BorderRadius: 8, BackgroundColor: "#18181b", Color: "white"},
				ui.OnClick(func() { setCount(count() + 1) }),
			)
		},
		ui.Style{
			Display:        "flex",
			FlexDirection:  "column",
			Height:         "100%",
			AlignItems:     "center",
			JustifyContent: "center",
			Gap:            12,
		},
	)
}
`,
  },
  window: {
    lang: 'go',
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
    lang: 'go',
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
  cliInit: {
    lang: 'bash',
    code: `bunx @quickgui/cli init my-app
cd my-app
bun run dev`,
  },
  cliFormat: {
    lang: 'bash',
    code: `bun run fmt
bun run build`,
  },
  cliBuild: {
    lang: 'bash',
    code: `bun run build --target darwin-arm64 \\
  --sign "Developer ID Application: Example (TEAMID)" \\
  --notarize quickgui-notary`,
  },
} as const

export type SnippetKey = keyof typeof snippets
export type HighlightedSnippets = Record<SnippetKey, string>
