import { syncTokenKeys } from "@shikijs/magic-move/core";
import type { KeyedToken, KeyedTokensInfo } from "@shikijs/magic-move/types";

export const FONT_SIZE = 13;
export const LINE_HEIGHT = 22;
// Menlo's advance at 13 logical pixels (1233 / 2048 em). Keep typography and grid in sync.
export const CHARACTER_WIDTH = (FONT_SIZE * 1233) / 2048;

function identifier(token: KeyedToken): string | undefined {
  const name = token.content.match(/^[({.]*([A-Za-z_$][\w$-]*)[)}.,:;]*$/)?.[1];
  if (!name) return;
  const normalized = name.replace(/[-_]/g, "").toLowerCase();
  if (name === "color") return "textcolor";
  if (normalized === "bg") return "backgroundcolor";
  return normalized.length >= 3 ? normalized : undefined;
}

/** The website's exact-first matcher, including its cross-language identifier aliases. */
export function syncSimilarTokenKeys(
  from: KeyedTokensInfo,
  to: KeyedTokensInfo,
  mountedKeys: Iterable<string> = [],
) {
  // Exiting nodes may still be mounted when a switch interrupts their fade.
  const reserved = new Set([...mountedKeys, ...from.tokens.map((token) => token.key)]);
  const next = {
    ...to,
    tokens: to.tokens.map((token, index) => {
      const base = `${to.hash}-${index}`;
      let key = base;
      for (let suffix = 1; reserved.has(key); suffix++) key = `${base}:${suffix}`;
      reserved.add(key);
      return { ...token, key };
    }),
  };
  // Magic Move mutates its inputs. Cached snippets and the previous step stay immutable.
  const result = syncTokenKeys(
    { ...from, tokens: from.tokens.map((token) => ({ ...token })) },
    next,
  );
  const sourceKeys = new Set(result.from.tokens.map((token) => token.key));
  const matched = new Set(
    result.to.tokens.filter((token) => sourceKeys.has(token.key)).map((token) => token.key),
  );
  const candidates = new Map<string, KeyedToken[]>();
  for (const token of result.to.tokens) {
    if (matched.has(token.key)) continue;
    const name = identifier(token);
    if (!name) continue;
    const group = candidates.get(name) ?? [];
    group.push(token);
    candidates.set(name, group);
  }
  for (const token of result.from.tokens) {
    if (matched.has(token.key)) continue;
    const name = identifier(token);
    if (!name) continue;
    const match = candidates.get(name)?.shift();
    if (!match) continue;
    match.key = token.key;
    matched.add(token.key);
  }
  return result;
}

export interface PositionedToken {
  key: string;
  content: string;
  color: string;
  x: number;
  y: number;
  opacity: number;
}

/** Whitespace advances the monospace grid without allocating native text nodes. */
export function layoutTokens(step: KeyedTokensInfo): Map<string, PositionedToken> {
  const result = new Map<string, PositionedToken>();
  let row = 0;
  let column = 0;
  for (const token of step.tokens) {
    if (/\S/.test(token.content)) {
      result.set(token.key, {
        key: token.key,
        content: token.content,
        color: token.color ?? step.fg ?? "#e1e4e8",
        x: column * CHARACTER_WIDTH,
        y: row * LINE_HEIGHT,
        opacity: 1,
      });
    }
    for (const character of token.content) {
      if (character === "\n") {
        row++;
        column = 0;
      } else if (character === "\t") {
        column += 2 - (column % 2);
      } else {
        column++;
      }
    }
  }
  return result;
}

/** A switch submits targets once; the Rust core interpolates each retained token. */
export function planMorph(
  from: KeyedTokensInfo,
  to: KeyedTokensInfo,
  mounted: ReadonlyMap<string, PositionedToken>,
) {
  const current = syncSimilarTokenKeys(from, to, mounted.keys()).to;
  const target = layoutTokens(current);
  const start = new Map<string, PositionedToken>();
  const entering: string[] = [];
  for (const [key, token] of target) {
    if (mounted.has(key)) start.set(key, token);
    else {
      start.set(key, { ...token, y: token.y + 6, opacity: 0 });
      entering.push(key);
    }
  }
  // Retain only this switch's exits. Interrupted fades never accumulate extra layers.
  for (const token of from.tokens) {
    const previous = mounted.get(token.key);
    if (previous && !target.has(token.key)) {
      start.set(token.key, { ...previous, y: previous.y - 6, opacity: 0 });
    }
  }
  return { current, target, start, entering };
}
