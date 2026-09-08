# Repository guidance

## Architecture

- Applications use Go or MoonBit. Go loads the shared library through `purego` with `CGO_ENABLED=0`; MoonBit uses native compilation and the bounded C FFI transport in `moonbit/native`. Both run in the same process. Do not introduce CGO, or an IPC frontend bridge.
- Implement shared native framework capabilities in the Rust core. Language bindings expose core behavior; language-specific component construction and fine-grained reactivity belong in the respective frontend. Go and MoonBit view constructors take children or content; styles, properties, and handlers use fluent methods. Use `ui.Style()` with fluent modifiers and `.Merge(other)` for reusable Go styles; apply them with `.Style(shared)` or compound-part `Style` fields. Do not restore the removed style-option or style-record APIs. Keep layout conveniences aligned with Rust through `scripts/generate-style-helpers.ts`.
- TypeScript and Bun are development, CLI, configuration, and packaging tooling only. Ordinary application edits reuse native shared libraries and rebuild only the selected frontend. See the [MoonBit guide](moonbit/README.md) for its toolchain and checks.
- MoonBit examples use constructor child lists, signal getter/setter pairs, and direct reactive expressions. Read the [view compiler contract](docs/architecture/moonbit-views.md) before changing their compiler, bindings, tests, or source mapping.

## Performance

Read and follow the [performance guide](docs/architecture/performance.md) before changing runtime scheduling, reactivity, native-host mutations, layout, rendering, or cache/resource reuse.
