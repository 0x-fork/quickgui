export const snippets = {
  typescriptSwiftUi: {
    lang: "tsx",
    code: `import { Button, Host } from "@quickgui/solid/swift-ui";
import { buttonStyle } from "@quickgui/solid/swift-ui/modifiers";

function SaveButton() {
  return (
    <Host matchContents>
      <Button
        label="Save changes"
        systemImage="checkmark"
        modifiers={[buttonStyle("glass")]}
      />
    </Host>
  );
}
`,
  },
  typescriptCounter: {
    lang: "tsx",
    code: `function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <View width="100%" height="100%"
      display="flex" flexDirection="column"
      alignItems="center" justifyContent="center"
      gap={20} backgroundColor="#090d16"
      color="#e2e8f0">
      <Text>Count: {count()}</Text>
      <Button padding={12} borderRadius={8}
        backgroundColor="#2563eb"
        onClick={() => setCount(count() + 1)}>
        Increment
      </Button>
    </View>
  );
}
`,
  },
  typescriptCliInit: {
    lang: "bash",
    code: "bunx @quickgui/cli init my-app --frontend typescript\ncd my-app\nbun run dev",
  },
  typescriptCliCheck: { lang: "bash", code: "bun run check\nbun run test\nbun run fmt" },
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
  cliBuild: {
    lang: "bash",
    code: `bun run build --target darwin-arm64 \\
  --sign "Developer ID Application: Example (TEAMID)" \\
  --notarize quickgui-notary`,
  },
} as const;

export type SnippetKey = keyof typeof snippets;
export type HighlightedSnippets = Record<SnippetKey, string>;
