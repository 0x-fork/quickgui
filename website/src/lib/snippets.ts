export const snippets = {
  counter: {
    lang: 'go',
    code: `package main

import (
	"fmt"

	"github.com/egoist/quickgui/go/ui"
)

func Counter() {
	count, setCount := ui.CreateSignal(0)
	ui.View(
		ui.Display("flex"),
		ui.FlexDirection("column"),
		ui.Height("100%"),
		ui.AlignItems("center"),
		ui.JustifyContent("center"),
		ui.Gap(12),
		func() {
			ui.Text(func() string {
				return fmt.Sprintf("Count: %d", count())
			})
			ui.Button(
				ui.Padding(12),
				ui.BorderRadius(8),
				ui.BackgroundColor("#18181b"),
				ui.Color("white"),
				ui.OnClick(func() { setCount(count() + 1) }),
				"Increment",
			)
		},
	)
}`,
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
}`,
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
)`,
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
