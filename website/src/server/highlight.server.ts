import type { FrontendTokens, HighlightedSnippets, SnippetKey } from "../lib/snippets";

let cached: Promise<{
  highlighted: HighlightedSnippets;
  counters: FrontendTokens;
  swiftUi: FrontendTokens;
}> | null = null;

async function highlightAll() {
  const [
    { snippets, counterSnippets, swiftUiSnippets },
    { codeToKeyedTokens },
    { createHighlighterCore },
    { createJavaScriptRegexEngine },
    go,
    bash,
    tsx,
    rust,
    githubLight,
    githubDark,
  ] = await Promise.all([
    import("../lib/snippets"),
    import("@shikijs/magic-move/core"),
    import("shiki/core"),
    import("shiki/engine/javascript"),
    import("shiki/langs/go.mjs"),
    import("shiki/langs/bash.mjs"),
    import("shiki/langs/tsx.mjs"),
    import("shiki/langs/rust.mjs"),
    import("shiki/themes/github-light.mjs"),
    import("shiki/themes/github-dark.mjs"),
  ]);

  const highlighter = await createHighlighterCore({
    themes: [githubLight.default, githubDark.default],
    langs: [go.default, bash.default, tsx.default, rust.default],
    engine: createJavaScriptRegexEngine({ forgiving: true }),
  });

  const out = {} as HighlightedSnippets;
  for (const key of Object.keys(snippets) as Array<SnippetKey>) {
    const { lang, code } = snippets[key];
    out[key] = highlighter.codeToHtml(code, {
      lang,
      themes: { light: "github-light", dark: "github-dark" },
      defaultColor: false,
    });
  }
  function highlightFrontends(examples: Record<keyof FrontendTokens, SnippetKey>): FrontendTokens {
    return Object.fromEntries(
      Object.entries(examples).map(([frontend, key]) => {
        const { lang, code } = snippets[key];
        return [frontend, codeToKeyedTokens(highlighter, code.trimEnd().replaceAll("\t", "  "), {
          lang,
          themes: { light: "github-light", dark: "github-dark" },
          defaultColor: false,
        })];
      }),
    ) as FrontendTokens;
  }

  return {
    highlighted: out,
    counters: highlightFrontends(counterSnippets),
    swiftUi: highlightFrontends(swiftUiSnippets),
  };
}

export async function getHighlightedSnippets(): Promise<HighlightedSnippets> {
  cached ??= highlightAll();
  return (await cached).highlighted;
}

export async function getCounterTokens(): Promise<FrontendTokens> {
  cached ??= highlightAll();
  return (await cached).counters;
}

export async function getSwiftUiTokens(): Promise<FrontendTokens> {
  cached ??= highlightAll();
  return (await cached).swiftUi;
}
