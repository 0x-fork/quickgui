# {{README_TITLE}}

A QuickGUI application written in TypeScript and compiled to native code.

```console
bun install
bun run dev
```

`bun run dev` compiles `src/app.tsx` into a native development app, runs it, and rebuilds and
restarts it on every source change. `bun run build` produces the distributable application.

Requires macOS 14+, Node.js 24+ for scriptc, Bun for tooling, and Xcode Command Line Tools.
TypeScript 7 lowers JSX. Applications contain native code and the Rust host.
