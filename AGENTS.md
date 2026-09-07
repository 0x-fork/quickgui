# Repository guidance

## Architecture

- Applications and components use the Go frontend, which loads the Rust shared library in the same process through `purego` with `CGO_ENABLED=0`. There is no JavaScript application runtime. Do not reintroduce CGO or an IPC frontend bridge.
- Implement shared native framework capabilities in the Rust core whenever possible. Go bindings should expose core behavior instead of maintaining a parallel source of truth; Go-specific component construction and fine-grained reactivity belong in the Go frontend.
- TypeScript and Bun are development, CLI, configuration, and packaging tooling only. Preserve fast application builds: ordinary Go edits reuse the native shared library and recompile only Go.

## Performance

Read and follow the [performance guide](docs/architecture/performance.md) before changing runtime scheduling, reactivity, native-host mutations, layout, rendering, or cache/resource reuse.
