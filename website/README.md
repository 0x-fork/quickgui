# QuickGUI website

Website for [QuickGUI](https://github.com/egoist/quickgui), built with
React Router framework mode, Tailwind CSS v4, shadcn/ui, and
`@egoist/tailwindcss-icons`. Server rendering runs on Cloudflare Workers.

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

- Code snippets are highlighted server-side with shiki (github-light theme)
  from the home route loader; the grammars never ship to the client.
- GitHub stars and the crates.io version are fetched by the home route loader
  with a 10-minute in-memory cache.
- `public/og.png` is a static capture of the `/og` card; regenerate it by
  screenshotting that route at 1200×630 if the branding changes.
