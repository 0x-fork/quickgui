# QuickGUI website

Marketing site for [QuickGUI](https://github.com/egoist/quickgui), built with
TanStack Start (+ TanStack Query), Tailwind CSS v4, shadcn/ui, and
`@egoist/tailwindcss-icons`. Deploys to Cloudflare Workers.

This package is part of the repo's bun workspace — install from the repo root.

## Develop

```console
bun install        # at the repo root
cd website
bun run dev
```

## Build & deploy

```console
bun run build
bun run deploy   # wrangler deploy
```

`bun run preview` serves the production build locally via the Cloudflare Vite
plugin (workerd).

## Notes

- Code snippets are highlighted server-side with shiki (vesper theme) inside a
  TanStack server function; the grammars never ship to the client.
- GitHub stars and the crates.io version are fetched by a server function with
  a 10-minute in-memory cache and hydrated through TanStack Query.
- `public/og.png` is a static capture of the `/og` card; regenerate it by
  screenshotting that route at 1200×630 if the branding changes.
