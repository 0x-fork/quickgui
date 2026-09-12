import { expect, test } from "bun:test";
import type { KeyedTokensInfo } from "@shikijs/magic-move/types";
import {
  CHARACTER_WIDTH,
  LINE_HEIGHT,
  layoutTokens,
  planMorph,
  syncSimilarTokenKeys,
} from "./code-morph.ts";
import { languages, loadSnippets } from "./snippets.ts";

const snippets = await loadSnippets();

function step(code: string): KeyedTokensInfo {
  return {
    code,
    hash: code,
    lineNumbers: false,
    tokens: [...code.matchAll(/\S+|\s+/g)].map((match, index) => ({
      content: match[0],
      offset: match.index,
      key: `${code}-${index}`,
    })),
  };
}

function keyOf(value: KeyedTokensInfo, content: string) {
  const token = value.tokens.find((token) => token.content === content);
  expect(token).toBeDefined();
  return token!.key;
}

test("exact matches take priority over equivalent identifiers and repeated names stay distinct", () => {
  const from = step("TextColor TextColor text-color Color BackgroundColor");
  const { to } = syncSimilarTokenKeys(from, step("text_color color text-color Color Bg"));
  expect(keyOf(to, "text-color")).toBe(keyOf(from, "text-color"));
  expect(keyOf(to, "text_color")).toBe(from.tokens[0]!.key);
  expect(keyOf(to, "color")).toBe(from.tokens[2]!.key);
  expect(keyOf(to, "Color")).toBe(keyOf(from, "Color"));
  expect(keyOf(to, "Bg")).toBe(keyOf(from, "BackgroundColor"));
  expect(new Set(to.tokens.map((token) => token.key)).size).toBe(to.tokens.length);
});

test("real highlighted Counter snippets retain cross-language identities", () => {
  const counter = snippets;
  const ts = syncSimilarTokenKeys(counter.go, counter.typescript).to;
  const rust = syncSimilarTokenKeys(ts, counter.rust).to;
  expect(keyOf(ts, "color")).toBe(keyOf(counter.go, "TextColor"));
  expect(keyOf(rust, "text_color")).toBe(keyOf(ts, "color"));
  expect(keyOf(rust, "flex_col")).toBe(keyOf(counter.go, "FlexCol"));
});

test("whitespace and blank lines advance the code grid without creating nodes", () => {
  const positions = [...layoutTokens(step("  one\n\n\ttwo three")).values()];
  expect(positions.map(({ content, x, y }) => ({ content, x, y }))).toEqual([
    { content: "one", x: 2 * CHARACTER_WIDTH, y: 0 },
    { content: "two", x: 2 * CHARACTER_WIDTH, y: 2 * LINE_HEIGHT },
    { content: "three", x: 6 * CHARACTER_WIDTH, y: 2 * LINE_HEIGHT },
  ]);
});

test("repeated and interrupted switches keep cached syntax, unique keys, and bounded exits", () => {
  const snapshot = JSON.stringify(snippets);
  for (const source of Object.values(snippets)) {
    source.tokens.forEach(Object.freeze);
    Object.freeze(source.tokens);
    Object.freeze(source);
  }
  let current = snippets.go;
  let mounted = layoutTokens(current);
  for (let round = 0; round < 5; round++) {
    for (const language of [...languages, ...languages.toReversed()]) {
      const source = snippets[language];
      const before = JSON.stringify(current);
      const previousCount = layoutTokens(current).size;
      const plan = planMorph(current, source, mounted);
      expect(JSON.stringify(current)).toBe(before);
      expect(plan.current.tokens.map(({ key, ...token }) => token)).toEqual(
        source.tokens.map(({ key, ...token }) => token),
      );
      expect(new Set(plan.current.tokens.map((token) => token.key)).size).toBe(
        plan.current.tokens.length,
      );
      expect(plan.start.size).toBeLessThanOrEqual(previousCount + plan.target.size);
      for (const key of plan.entering) {
        expect(mounted.has(key)).toBe(false);
        expect(plan.start.get(key)!.opacity).toBe(0);
        expect(plan.target.get(key)!.opacity).toBe(1);
      }
      current = plan.current;
      mounted = plan.start; // Retarget before entry or exit has completed.
    }
  }
  expect(JSON.stringify(snippets)).toBe(snapshot);
});
