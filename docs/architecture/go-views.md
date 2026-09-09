# Go view compiler

`quickgui dev`, `build`, `check`, and `test` use the same Go source compiler.
Applications remain valid Go with ordinary parameter types and editor tooling.
No annotations, CGO, application JavaScript, or extra host processes are involved.

Go components declare their children implicitly and have no return value.
Window roots, routes, conditional branches, list renderers, and child blocks use
`func()` callbacks. The runtime collects their detached roots in declaration
order. Nested child blocks collect independently; nodes attached to a parent or
owned by a fragment do not become duplicate siblings. A component can declare
zero, one, or multiple roots without a layout wrapper.

The compiler uses `go list` for the active target and build tags, and `go/types`
object identities for parameters, imports, shadowing, and component calls.
Top-level declaration functions that construct native UI are components.
Discovery follows calls between declaration functions until it also recognizes
forwarding components across application packages. Nested callbacks do not turn
an outer setup function into a component. Methods and helpers returning node
handles retain ordinary Go call semantics. Node-returning component factories
are unsupported; dynamically typed child positions receive an original-source
diagnostic, and typed component positions are checked by Go itself.

Component implementations receive getters for plain value parameters. Direct
calls carry live expressions into those getters. Static arguments are evaluated
once, in order, and callbacks and element children keep their original types.
Setup assignments remain snapshots. Parameters assigned to or addressed as
local storage keep ordinary Go value semantics. Ordinary function values retain
their original signatures and normal eager argument evaluation.

Call components inside child blocks: `ui.View(func() { Label(count()) })`.
Native constructors also accept existing children directly, such as
`ui.View(ui.Text(count()))`. Their returned builders support fluent modifiers
and node references; they do not define a component return boundary.
`ui.Text(count())`, `.Width(width())`, `.When(selected(), style)`, and
`ui.Show(visible(), Details)` preserve live reads without accessor wrappers.
Use `ui.Child(node)` to declare an existing detached node inside a block.

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
