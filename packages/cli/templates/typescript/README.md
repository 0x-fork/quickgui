# {{README_TITLE}}

A native QuickGUI application using Bun and Solid 2.

```sh
bun install
bun run check
bun run dev
bun run build
```

Requires Bun 1.4 or later and the matching QuickGUI native library. `app.tsx` runs in a Bun worker in the same process as Rust's native UI loop. The CLI compiles JSX and packages the executable; launch it through `quickgui dev`.
