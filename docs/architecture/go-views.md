# Go view compiler

`quickgui dev`, `build`, `check`, and `test` use the same Go source compiler.
Applications remain valid Go with ordinary parameter types and editor tooling.
No annotations, CGO, application JavaScript, or extra host processes are involved.

Go components return `*ui.Element` or `*native.Node`. Compose native children
and component results directly, such as `return ui.View(Label(count()))`.
Window roots, routes, conditional branches, list renderers, and deferred child
callbacks also return nodes. Use `ui.Fragment(...)` for multiple siblings without
a layout container. Only the returned tree mounts. Implicit declarations and
void construction callbacks are unsupported.

The compiler uses `go list` for the active target and build tags, and `go/types`
object identities for parameters, imports, shadowing, and component calls.
Top-level functions with a native node return type are component boundaries.
Methods retain ordinary call semantics. Native view expressions still bind
independently wherever they appear. Discarded UI construction calls, blank
assignments, and void construction callbacks receive original-source diagnostics.
Existing-node setters, event handlers, effects, and lifecycle callbacks retain
their ordinary behavior.

Component implementations receive getters for plain value parameters. Direct
calls carry live expressions into those getters. Static arguments are evaluated
once, in order, and callbacks and element children keep their original types.
Setup assignments remain snapshots. Parameters assigned to or addressed as
local storage keep ordinary Go value semantics. Ordinary function values retain
their original signatures and normal eager argument evaluation.

`ui.Text(count())`, `.Width(width())`, `.When(selected(), style)`, and
`ui.Show(visible(), Details)` preserve live reads without accessor wrappers.
Native scalar text and fluent properties accepting accessors bind independently
through the existing frontend. The component body runs once. Each retained node
owns its bindings; unmounting releases them. The compiler adds no scheduling,
polling, whole-component effects, or native-tree reconstruction. Explicit
accessors continue to work, including inside untransformed helper methods.

`native.NodeProvider` exposes a frontend builder's native node without a native
package dependency on `ui`. The runtime invokes unfamiliar frontend factories
through reflection only at mount time. Ordinary component calls and retained
binding updates remain direct calls. Unused detached allocations made by a
factory are retired; they never mount as implicit siblings.

The SDK and registry dependencies are not transformed. Application packages,
local replacements, and workspace dependencies share component metadata.
External dependencies use cached Go export data; application source is checked
without compiling an extra original executable. Generated files are written
atomically under `.quickgui/go-sources` and supplied through Go's `-overlay`
flag. Unchanged output keeps its modification time. Each target, build-tag,
package-pattern, and test configuration has a separate cache partition, so a dev
watcher cannot prune a concurrent test overlay. The compiler runs on the host
even when the application target differs.

Inline Go line directives preserve source locations around moved expressions
and generated implementations. Native debuggers still see generated getter
parameters; source-level variable types have not been remapped. Raw Go commands
bypass the view compiler and require explicit getters for reactive composition.

Run `bash scripts/check-go.sh` for the SDK and compiled applications, and
`bun test packages/cli/src/go-build.test.ts` for overlay preservation and runtime
regressions. The cart fixture checks struct props, imported and forwarding
components, setup snapshots, node identity, mutation counts, source locations,
and disposal without opening a window.
