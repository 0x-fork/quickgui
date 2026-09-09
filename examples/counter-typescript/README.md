# TypeScript counter

Native QuickGUI windows using Bun and Solid 2. Includes reactive text, input, conditional content, and macOS Dock reopening.

From the repository root:

```sh
bun install
bun run build:native
bun packages/cli/src/cli.ts dev --project examples/counter-typescript
```

The CLI compiles JSX using Solid's universal renderer and embeds the application worker in a Bun executable. Rust's shared library runs AppKit on the main thread. See [the TypeScript guide](../../docs/typescript.md).
