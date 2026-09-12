import { expect, test } from "bun:test";
import { counterSnippets, snippets, swiftUiSnippets } from "../src/lib/snippets";
import { getCounterTokens, getHighlightedSnippets, getSwiftUiTokens } from "../src/server/highlight.server";

test("the homepage can highlight every frontend, including Solid TSX", async () => {
  const highlighted = await getHighlightedSnippets();
  expect(Object.keys(highlighted)).toEqual(Object.keys(snippets));
  for (const html of Object.values(highlighted)) {
    expect(html).toContain("<pre");
    expect(html).toContain("</pre>");
  }
  expect(highlighted.typescriptCounter).toContain("createSignal");
  expect(highlighted.typescriptCounter).toContain("background-color");
  expect(snippets.typescriptCounter.code).not.toContain("backgroundColor");
  expect(snippets.typescriptCounter.code).toMatch(/\brounded-lg(?:\s|>)/);
  expect(snippets.typescriptCounter.code).not.toContain('rounded="');
});

test("precompiled homepage tokens preserve source and both color themes", async () => {
  const counters = await getCounterTokens();
  const swiftUi = await getSwiftUiTokens();
  expect(await getCounterTokens()).toBe(counters);
  expect(await getSwiftUiTokens()).toBe(swiftUi);
  for (const [examples, tokens] of [[counterSnippets, counters], [swiftUiSnippets, swiftUi]] as const) {
    for (const [frontend, key] of Object.entries(examples)) {
      const step = tokens[frontend as keyof typeof tokens];
      const source = snippets[key].code.trimEnd().replaceAll("\t", "  ");
      expect(step.code).toBe(source);
      expect(step.tokens.map((token) => token.content).join("").trimEnd()).toBe(source);
      expect(new Set(step.tokens.map((token) => token.key)).size).toBe(step.tokens.length);
      expect(JSON.stringify(step.tokens)).toContain("--shiki-light");
      expect(JSON.stringify(step.tokens)).toContain("--shiki-dark");
    }
  }
});
