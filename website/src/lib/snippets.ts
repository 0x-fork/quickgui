export const snippets = {
  /* Just the component (see docs/view-api.md for the full main.rs); lines
     stay ≤ 62 chars so the half-width panes never scroll horizontally. */
  rust: {
    lang: 'rust',
    code: `struct Counter {
    count: usize,
}

impl View for Counter {
    fn render(
        &mut self,
        cx: &mut ViewContext<'_, Self>,
    ) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx| {
            this.count += 1;
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(text(format!("Count: {}", self.count)))
            .child(
                div()
                    .on_click(increment)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(24, 24, 27))
                    .text_color(Color::rgb8(250, 250, 250))
                    .hover(|s| s.bg(Color::rgb8(63, 63, 70)))
                    .child("Increment"),
            )
    }
}`,
  },
  ui: {
    lang: 'tsx',
    code: `function Counter() {
  const [count, setCount] = createSignal(0);

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        gap: 12,
      }}
    >
      <Text>Count: {count()}</Text>
      <Button onClick={() => setCount(count() + 1)}>
        Increment
      </Button>
    </View>
  );
}`,
  },
  swiftUi: {
    lang: 'tsx',
    code: `import { Button, Host } from "@quickgui/ui/swift-ui";
import { buttonStyle } from "@quickgui/ui/swift-ui/modifiers";

<Host matchContents>
  <Button
    label="Save changes"
    modifiers={[buttonStyle("glass")]}
  />
</Host>;`,
  },
  cargoAdd: {
    lang: 'bash',
    code: `cargo add quickgui`,
  },
  cargoRun: {
    lang: 'bash',
    code: `cargo run --release`,
  },
  cliInit: {
    lang: 'bash',
    code: `bunx @quickgui/cli init my-app
cd my-app
bun run dev`,
  },
  cliBuild: {
    lang: 'bash',
    code: `quickgui build --target darwin-arm64 \\
  --sign "Developer ID Application: Example (TEAMID)" \\
  --notarize quickgui-notary`,
  },
} as const

export type SnippetKey = keyof typeof snippets
export type HighlightedSnippets = Record<SnippetKey, string>
