# MoonBit view compiler

`quickgui dev`, `build`, `check`, and `test` share the source transform in
`packages/cli/src/moonbit`. It uses the pinned official Tree-sitter MoonBit
grammar, bundled as WASM in the TypeScript CLI. Applications contain only native
MoonBit code and the existing QuickGUI runtime. Changes to the SDK surface require
`bun scripts/generate-moonbit-view-api.ts`; parser upgrades use
`bun scripts/build-moonbit-parser.ts` and must run the compiler regression suite.

The application source remains valid MoonBit. View constructors accept only children or content. Styles, properties, and event handlers use fluent methods; `.style(shared)` merges reusable styles. `create_signal` returns a getter
and setter backed by the existing signal implementation. Applications use
`@ui.create_signal` and `@ui.create_memo`; memos expose read-only getters using
the same caching, equality, batching, and ownership as the reactive core.
At native view sites,
expressions containing calls are lifted into callbacks. Tracking discovers their
actual signal dependencies, including reads through helper functions. Literals and
setup snapshots stay static. Event callbacks and explicit bindings retain their
semantics. Functions returning `@ui.Element` are component boundaries. Their plain
value parameters stay ordinary types in authored source. The compiler generates
getter-based implementations, rewrites direct calls across local files and
workspace packages, and lifts prop reads into the existing native bindings.
Callbacks and element children retain their ordinary types. Static arguments are
captured once. Component setup still executes once; `let initial = quantity`
captures a snapshot. Passing a component as an ordinary function value retains
normal call semantics. Methods remain ordinary helpers and can accept explicit
getters. No annotations or special prop types are needed.

Text/content, native properties, styles, and conditional children bind independently.
Nodes own subscriptions and dispose them on removal. Style chains are merged in
declaration order inside one binding, including later static overrides. Boolean
branch selectors compare their results so an unchanged branch retains its nodes.
Computed arrays use a collection binding; stateful lists should explicitly use
`children_keyed`. Do not turn an entire component into an effect, add polling, or
rebuild its retained tree to refresh a property. Binding expressions must be pure.

Local application/workspace packages are mirrored into a stable
`.quickgui/moonbit-sources` directory. The SDK is not transformed. Unchanged source
and generated output retain their cache identity and modification times. A compiler
or grammar change invalidates the transform cache. Never write through a source
symlink or format generated code back into the user's source files.

Use `quickgui check` and `quickgui test` for applications. SDK implementation tests
can still run with `moon test` and explicit low-level bindings. Raw Moon commands
do not run the view compiler. The checked-in compiler fixtures must run in both
native debug and release builds and verify node identity, mutation counts,
controlled state, property clearing, branch disposal, and collection ownership.

Generated source spans map diagnostics and CLI stack locations to original `.mbt`
files. Structured compiler diagnostics render the original line and caret. Tests
cover moved expressions, Unicode columns, unchanged files, compiler errors, and
assertion failures. A native panic was also checked on macOS. Native debuggers
currently use the cached generated files for breakpoints and stepping: these source
maps do not rewrite DWARF. Original-file native breakpoint support is outstanding.
Release optimization may remove frames and locals, as with other native builds.
