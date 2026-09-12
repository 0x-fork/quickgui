import { syncTokenKeys } from "@shikijs/magic-move/core";
import type { KeyedToken, KeyedTokensInfo } from "@shikijs/magic-move/types";

function identifier(token: KeyedToken): string | undefined {
  // Shiki sometimes groups Go struct fields with their punctuation, e.g.
  // "{MatchContents:". Keep that spelling intact while matching the name.
  const name = token.content.match(/^[({.]*([A-Za-z_$][\w$-]*)[)}.,:;]*$/)?.[1];
  if (!name) return;

  const normalized = name.replace(/[-_]/g, "").toLowerCase();
  // TypeScript's color style key corresponds to TextColor/text_color, but
  // Rust's Color type does not. Background helpers use both names.
  if (name === "color") return "textcolor";
  if (normalized === "bg") return "backgroundcolor";
  return normalized.length >= 3 ? normalized : undefined;
}

/** Match exact tokens first, then pair remaining equivalent identifiers. */
export function syncSimilarTokenKeys(from: KeyedTokensInfo, to: KeyedTokensInfo) {
  const reservedKeys = new Set(from.tokens.map((token) => token.key));
  const next = {
    ...to,
    tokens: to.tokens.map((token, index) => {
      // A revisited example may already have keys carried by the current
      // step. Start with disjoint keys so unmatched tokens cannot collide.
      const base = `${to.hash}-${index}`;
      let key = base;
      for (let suffix = 1; reservedKeys.has(key); suffix++) key = `${base}:${suffix}`;
      reservedKeys.add(key);
      return { ...token, key };
    }),
  };
  // Shiki's matcher mutates keys. Never mutate cached loader data or a
  // previously rendered step: both can be reused on the next switch.
  const result = syncTokenKeys(
    { ...from, tokens: from.tokens.map((token) => ({ ...token })) },
    next,
  );
  const sourceKeys = new Set(result.from.tokens.map((token) => token.key));
  const matchedKeys = new Set(
    result.to.tokens.filter((token) => sourceKeys.has(token.key)).map((token) => token.key),
  );
  const candidates = new Map<string, KeyedToken[]>();

  for (const token of result.to.tokens) {
    if (matchedKeys.has(token.key)) continue;
    const name = identifier(token);
    if (!name) continue;
    const group = candidates.get(name) ?? [];
    group.push(token);
    candidates.set(name, group);
  }

  for (const token of result.from.tokens) {
    if (matchedKeys.has(token.key)) continue;
    const name = identifier(token);
    if (!name) continue;
    // Pair repeated names in source order and consume each destination once.
    const match = candidates.get(name)?.shift();
    if (!match) continue;
    match.key = token.key;
    matchedKeys.add(token.key);
  }

  return result;
}
