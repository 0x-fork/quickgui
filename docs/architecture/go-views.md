# Go view compiler

`quickgui dev`, `build`, `check`, and `test` use the same Go source compiler.
Applications remain valid Go with ordinary parameter types and editor tooling.
No annotations, CGO, application JavaScript, or extra host processes are involved.

The compiler uses `go list` for the active target and build tags, and `go/types`
object identities for parameters, imports, shadowing, and component calls.
Top-level functions returning `*ui.Element` or `*native.Node` are components.
This is the same return-type boundary used by MoonBit; declaring UI from a void
helper does not change the evaluation of that helper’s parameters. Native view
expressions still bind independently wherever they appear. Methods retain
ordinary call semantics.

Component implementations receive getters for plain value parameters. Direct
calls carry live expressions into those getters. Static arguments are evaluated
once, in order, and callbacks and element children keep their original types.
Setup assignments remain snapshots. Parameters assigned to or addressed as
local storage keep ordinary Go value semantics. Ordinary function values retain
their original signatures and normal eager argument evaluation.

Compose ordinary children directly: `return ui.View(Label(count()), ui.Button("Add"))`.
`ui.Text(count())`, `.Width(width())`, `.When(selected(), style)`, and
`ui.Show(visible(), Details)` preserve their live reads without accessor wrappers.
Callbacks are optional for child construction, and remain useful for lazy branches
and compound controls that provide child context.

Window roots, routes, lazy children, and list renderers accept node-returning
functions. `native.NodeProvider` connects a frontend builder to its native node.
The native window invokes an unfamiliar frontend factory through reflection only
when mounting; ordinary component calls and binding updates remain direct calls.
A returned root mounts exactly the selected node without an extra wrapper.
Unused detached declarations are retired, and existing scoped disposal applies.
Declaration callbacks remain supported.

Native scalar text and fluent properties accepting accessors bind independently
through the existing frontend. The component body runs once. Each retained node
owns its bindings; unmounting releases them. The compiler adds no scheduling,
polling, whole-component effects, or native-tree reconstruction. Explicit
accessors continue to work, including inside untransformed helper methods.

The SDK and registry dependencies are not transformed. Application packages,
local replacements, and workspace dependencies share component metadata.
External dependencies use cached Go export data; application source is checked
without compiling an extra original executable. Generated files are written
atomically under `.quickgui/go-sources` and supplied through Go's `-overlay`
flag. Unchanged output keeps its modification time, and stale overlays are
removed. The compiler runs on the host even when the application target differs.

Inline Go line directives preserve source locations around moved expressions
and generated implementations. Native debuggers still see generated getter
parameters; source-level variable types have not been remapped. Raw Go commands
bypass the view compiler and require explicit getters for reactive composition.

Run `bash scripts/check-go.sh` for the SDK and compiled applications, and
`bun test packages/cli/src/go-build.test.ts` for overlay preservation and runtime
regressions. The cart fixture checks struct props, imported components, setup
snapshots, node identity, mutation counts, and disposal without opening a window.
