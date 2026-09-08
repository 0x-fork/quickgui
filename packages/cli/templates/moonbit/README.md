# {{README_TITLE}}

Native MoonBit application using QuickGUI’s child lists, named arguments, and fine-grained signals.

Install [MoonBit](https://www.moonbitlang.com/download/), a C compiler (Xcode Command Line Tools on macOS), and Bun. Then run:

```sh
bun install
bun run dev
```

`bun run build` creates a self-contained application; `bun run fmt` runs `moon fmt`.
Ordinary edits rebuild the MoonBit application and reuse the shared native libraries.
Development uses MoonBit’s fast native backend on Apple Silicon macOS and x64 Linux/Windows; release builds use the optimizing C backend. Set `MOONBIT_NEW_NATIVE=0` to use C compilation during development for troubleshooting.
Build on the platform and architecture you intend to ship. The Go frontend remains available separately.

During QuickGUI source development, put this module and QuickGUI’s `moonbit` directory in a `moon.work` file. The SDK must be published to Mooncakes before the registry dependency can be used independently.
