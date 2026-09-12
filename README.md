# QuickGUI

QuickGUI is a native desktop GUI framework for **Go**, **TypeScript**, and **Rust**. Write windows and components in the language you already use; the same renderer, layout, and controls run on macOS (Windows and Linux compile, with native polish still in progress).

## Try it

Clone this repository, then run a counter:

```console
bun install
bun run build:native
cd examples/counter && bun run dev
```

TypeScript: `examples/counter-typescript`. Rust: `bun packages/cli/src/cli.ts init ../my-app --language rust`.

You need Go 1.23+ (for Go apps), Bun 1.4+, a current Rust toolchain (for Rust apps), and Xcode Command Line Tools on macOS.

## Documentation

Guides and the component reference live in the website. From this checkout:

```console
cd website
bun run dev
```

Start with:

- [Go](website/src/content/docs/go/en/getting-started.mdx)
- [TypeScript](website/src/content/docs/typescript/en/getting-started.mdx)
- [Rust](website/src/content/docs/rust/en/getting-started.mdx)

Contributor notes (architecture, release, internals) are in [docs/](docs/README.md).

## License

MIT OR Apache-2.0.
