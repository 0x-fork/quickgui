import { expect, test } from "bun:test";
import type { KeyedTokensInfo } from "@shikijs/magic-move/types";
import { syncSimilarTokenKeys } from "../src/lib/code-morph";
import { getCounterTokens, getSwiftUiTokens } from "../src/server/highlight.server";
import { DOCS_FRONTENDS } from "../src/lib/docs";

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

function keyOf(tokens: KeyedTokensInfo, content: string) {
  const token = tokens.tokens.find((token) => token.content === content);
  expect(token).toBeDefined();
  return token!.key;
}

test("identifiers morph across camel, Pascal, kebab, and snake case", () => {
  let current = step("text-color flex-col on-click system-image");
  for (const source of [
    "TextColor FlexCol OnClick SystemImage",
    "textColor flexCol onClick systemImage",
    "text_color flex_col on_click system_image",
    "text-color flex-col on-click system-image",
  ]) {
    const next = syncSimilarTokenKeys(current, step(source)).to;
    for (const [index, token] of next.tokens.entries()) {
      expect(token.key).toBe(current.tokens[index].key);
    }
    expect(next.tokens.map((token) => token.content).join("")).toBe(source);
    current = next;
  }
});

test("exact matches take priority and repeated names keep distinct identities", () => {
  const from = step("TextColor TextColor text-color");
  const { to } = syncSimilarTokenKeys(from, step("text_color textColor text-color"));
  expect(keyOf(to, "text-color")).toBe(keyOf(from, "text-color"));
  expect(keyOf(to, "text_color")).toBe(from.tokens[0].key);
  expect(keyOf(to, "textColor")).toBe(from.tokens[2].key);
  expect(new Set(to.tokens.map((token) => token.key)).size).toBe(to.tokens.length);
});

test("only equivalent names match, with SDK aliases separate from types and literals", () => {
  const from = step('TextColor BackgroundColor "flex-col" count');
  const { to } = syncSimilarTokenKeys(from, step("Color color Bg FlexCol counter"));
  expect(keyOf(to, "color")).toBe(keyOf(from, "TextColor"));
  expect(keyOf(to, "Bg")).toBe(keyOf(from, "BackgroundColor"));
  const oldKeys = new Set(from.tokens.map((token) => token.key));
  for (const content of ["Color", "FlexCol", "counter"]) {
    expect(oldKeys.has(keyOf(to, content))).toBe(false);
  }
});

test("actual counter and SwiftUI tokens retain equivalent identifiers", async () => {
  const counters = await getCounterTokens();
  const swiftUi = await getSwiftUiTokens();
  const ts = syncSimilarTokenKeys(counters.go, counters.typescript).to;
  const rust = syncSimilarTokenKeys(ts, counters.rust).to;
  expect(keyOf(ts, "color:")).toBe(keyOf(counters.go, "TextColor"));
  expect(keyOf(rust, "text_color")).toBe(keyOf(ts, "color:"));
  expect(keyOf(ts, "flexCol:")).toBe(keyOf(counters.go, "FlexCol"));
  expect(keyOf(rust, "flex_col")).toBe(keyOf(ts, "flexCol:"));

  const swiftTs = syncSimilarTokenKeys(swiftUi.go, swiftUi.typescript).to;
  const swiftRust = syncSimilarTokenKeys(swiftTs, swiftUi.rust).to;
  expect(keyOf(swiftTs, "systemImage")).toBe(keyOf(swiftUi.go, "SystemImage:"));
  expect(keyOf(swiftRust, "system_image")).toBe(keyOf(swiftTs, "systemImage"));
  expect(keyOf(swiftTs, "matchContents")).toBe(keyOf(swiftUi.go, "{MatchContents:"));
});

test("repeated switches preserve source, styles, and unique keys without mutating cached steps", async () => {
  for (const original of [await getCounterTokens(), await getSwiftUiTokens()]) {
    const snapshot = JSON.stringify(original);
    const steps = structuredClone(original);
    for (const value of Object.values(steps)) {
      for (const token of value.tokens) Object.freeze(token);
      Object.freeze(value.tokens);
      Object.freeze(value);
    }

    let current = steps.go;
    for (let round = 0; round < 3; round++) {
      for (const frontend of [...DOCS_FRONTENDS, ...DOCS_FRONTENDS.toReversed()]) {
        const before = JSON.stringify(current);
        const next = syncSimilarTokenKeys(current, steps[frontend]).to;
        expect(JSON.stringify(current)).toBe(before);
        expect(next.code).toBe(steps[frontend].code);
        expect(next.tokens.map(({ key: _key, ...token }) => token)).toEqual(
          steps[frontend].tokens.map(({ key: _key, ...token }) => token),
        );
        expect(new Set(next.tokens.map((token) => token.key)).size).toBe(next.tokens.length);
        current = next;
      }
    }
    expect(JSON.stringify(original)).toBe(snapshot);
  }
});
