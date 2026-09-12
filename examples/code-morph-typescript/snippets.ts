import { codeToKeyedTokens } from "@shikijs/magic-move/core";
import type { KeyedTokensInfo } from "@shikijs/magic-move/types";
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import go from "shiki/langs/go.mjs";
import rust from "shiki/langs/rust.mjs";
import tsx from "shiki/langs/tsx.mjs";
import githubDark from "shiki/themes/github-dark.mjs";

export const languages = ["go", "typescript", "rust"] as const;
export type Language = (typeof languages)[number];
export const languageLabels = { go: "Go", typescript: "TypeScript", rust: "Rust" };
export const filenames = { go: "counter.go", typescript: "counter.tsx", rust: "counter.rs" };

// The website's Counter snippets, kept local so this app builds independently.
const counter: Record<Language, string> = {
  go: `func Counter() *ui.Element {
  count, setCount := ui.CreateSignal(0)
  return ui.View().
    FlexCol().
    SizeFull().
    ItemsCenter().
    JustifyCenter().
    Gap(20).
    Bg("#090d16").
    TextColor("#e2e8f0").
    Child(ui.Text("Count: ", count())).
    Child(ui.Button().
      OnClick(func() { setCount(count() + 1) }).
      Padding(12).
      RoundedLg().
      Bg("#2563eb").
      Child("Increment"))
}`,
  typescript: `function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <View flex-col size-full items-center justify-center gap-5
      bg="#090d16"
      color="#e2e8f0">
      <Text>Count: {count()}</Text>
      <Button p-3 rounded-lg
        bg="#2563eb"
        onClick={() => setCount(count() + 1)}>
        Increment
      </Button>
    </View>
  );
}`,
  rust: `impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx: &mut EventContext| {
            this.count += 1;
            cx.invalidate();
        });
        div()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_5()
            .bg(Color::rgb8(9, 13, 22))
            .text_color(Color::rgb8(226, 232, 240))
            .child(text(format!("Count: {}", self.count)))
            .child(
                button()
                    .p(12.0)
                    .rounded_lg()
                    .bg(Color::rgb8(37, 99, 235))
                    .on_click(increment)
                    .child("Increment"),
            )
    }
}`,
};

export type Snippets = Record<Language, KeyedTokensInfo>;

/** Tokenize three snippets once; release the grammar engine before opening any windows. */
export async function loadSnippets(): Promise<Snippets> {
  const highlighter = await createHighlighterCore({
    themes: [githubDark],
    langs: [go, tsx, rust],
    engine: createJavaScriptRegexEngine({ forgiving: true }),
  });
  try {
    return Object.fromEntries(
      languages.map((language) => [
        language,
        codeToKeyedTokens(highlighter, counter[language], {
          lang: language === "typescript" ? "tsx" : language,
          theme: "github-dark",
        }),
      ]),
    ) as Snippets;
  } finally {
    highlighter.dispose();
  }
}
