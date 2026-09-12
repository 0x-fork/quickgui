# TypeScript code morph

A native version of the website's animated code block, built with Bun and Solid 2.
Switch between Go, TypeScript, and Rust versions of the Counter snippet.
Use **Next language** to cycle, or enable **Slow motion** to follow individual tokens.
The app follows the system's Reduce Motion preference.

From the repository root:

```sh
bun install
bun run build:native # once, if the shared library has not been staged
bun packages/cli/src/cli.ts dev --project examples/code-morph-typescript
```

From this directory:

```sh
bun run check
bun run test
bun run build
```

Shiki highlights the three local snippets once with its JavaScript regex engine; no WASM
loader, DOM, or WebView is needed. `code-morph.ts` adapts the matching logic from
[`website/src/lib/code-morph.ts`](../../website/src/lib/code-morph.ts): exact matches
win, then equivalent names such as `TextColor`, `color`, and `text_color` share keys.
Cached tokens are never mutated.

Solid's keyed `For` retains native text nodes. Tokens sit on a Menlo grid and change
their transform, color, and opacity; Rust owns interpolation, interrupted transitions,
and animation scheduling. Two short-lived deadlines stage entering tokens and remove
exiting tokens. There is no JavaScript frame loop or idle timer, and window cleanup
cancels pending work. The fixed canvas keeps scrolling stable across snippets. This
example uses ASCII snippets and macOS Menlo metrics; a general editor would need
native text measurement for other fonts, Unicode, and wrapping.

See [the TypeScript guide](../../docs/typescript.md) for the compiler and runtime setup.
