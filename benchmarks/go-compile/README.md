# Go compile-time benchmark

Measured on Apple M5 (10 logical CPUs, 32 GiB), go version go1.27.1 darwin/arm64, with `CGO_ENABLED=0`. Wall-clock medians: 9 samples for each cached-dependency phase and 3 fresh-cache builds per variant. Raw samples, ranges, source hashes, and build-trace checks are in [results.json](results.json).

| Build | Before fluent API | Current SDK | Current, pruned file |
| --- | ---: | ---: | ---: |
| Recompile `ui` (dependencies cached) | 324.6 ms | 489.5 ms | 464.4 ms |
| Edit Counter (SDK cached, compile + link) | 159.3 ms | 163.6 ms | 160.0 ms |
| Fresh Go cache and output (stdlib + SDK + app + link) | 2.512 s | 2.701 s | 2.614 s |

The full SDK change adds 164.9 ms (50.8%) to a `ui` recompile. The specific `element_generated.go` control accounts for about 25.1 ms. Ordinary Counter edits stay near 0.16 seconds because the SDK is cached; small differences between these app-edit samples should not be treated as a stable speedup or slowdown.

## Controls

- **Before:** Go SDK from `23502e2fd0798ceee3f3e3b4d8d380b44504925a`.
- **Current:** the working SDK, including the fluent API, binding changes, and all layout helpers.
- **Pruned:** the same current SDK with 207 of 219 methods removed from `element_generated.go` in a temporary copy. The 12 methods referenced by SDK call sites remain. Other generated files and the runtime remain identical. This is a compile-cost experiment, not a proposed API reduction.
- Every variant builds the same Counter source from the baseline commit. A revision constant is appended to the window title so app edits change the linked executable.
- SDK rebuilds change a private constant in `ui`; Go's trace confirms only `github.com/egoist/quickgui/go/ui` compiles. This build runs from the app module, keeping dependency build contexts consistent.
- App edits change only the title revision. The trace confirms only `main` compiles and the linker runs.
- Fresh builds use a new private `GOCACHE` and remove the output binary before each sample. A separate untimed trace verified 95 package compilations, including `runtime`, `reflect`, `ui`, and `main`, plus a fresh link.
- Builds run sequentially, with variant order rotated each round. The existing module cache is reused with `GOPROXY=off`; OS file caches are not cleared. Rust compilation, code generation, application startup, and watcher overhead are excluded. The user's normal Go build cache is untouched.

## Reproduce

From the repository root:

```sh
bun scripts/benchmark-go-compile.ts \
  --baseline 23502e2fd0798ceee3f3e3b4d8d380b44504925a \
  --samples 9 --cold-samples 3
```

The script copies all variants into a temporary directory, validates the build traces, writes the JSON results, and removes its temporary SDKs and caches. Use `--keep` to inspect the copies or `--output path.json` to save another run.
