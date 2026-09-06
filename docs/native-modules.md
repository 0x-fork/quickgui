# Native modules in Zig

A native module is a Zig source file that your application imports like any other TypeScript
module. `@quickgui/cli` compiles `modules/<name>/main.zig` into a static library, generates a
typed `modules/<name>/index.ts` from the functions the module exports, and links the library in the
packaged application. The generated bindings are compiled by scriptc and cannot execute directly under Bun or Node.js.
The generated module offers every function twice: a synchronous call and an
`…Async` variant that runs on the native background threads and resolves a promise.

```
my-app/
├── modules/
│   └── stats/
│       ├── main.zig      # you write this
│       └── index.ts      # generated: typed wrappers, rewritten on every build
├── src/app.tsx
└── quickgui.config.ts
```

```zig
// modules/stats/main.zig
const std = @import("std");

pub const Summary = struct { lines: u32, longest: u32 };

/// Count the lines of `text` and the length of the longest one.
pub fn summarize(text: []const u8) Summary {
    var lines: u32 = 0;
    var longest: u32 = 0;
    var iterator = std.mem.splitScalar(u8, text, '\n');
    while (iterator.next()) |line| {
        lines += 1;
        longest = @max(longest, @as(u32, @intCast(line.len)));
    }
    return .{ .lines = lines, .longest = longest };
}
```

```ts
import { summarize, summarizeAsync, type Summary } from "./modules/stats";

const summary: Summary = summarize(source);          // { lines: 12, longest: 80 }
const later = await summarizeAsync(await file.text()); // off the JavaScript thread
```

`quickgui dev` and `quickgui build` compile modules before the application; `quickgui modules`
does only that, to refresh the generated bindings without building an application. The compiler is found as `zig` on `PATH` or
through `QUICKGUI_ZIG`, and must be Zig 0.16 or newer.

## What a module exports

Every `pub fn` of `main.zig` becomes an export, in declaration order. Generic functions
(`anytype` or `comptime` parameters) are skipped, as are types, constants, and variables. A
function is called with its arguments decoded from JavaScript, and its return value encoded back:

| Zig                                             | JavaScript                                     |
| ----------------------------------------------- | ---------------------------------------------- |
| `bool`                                          | `boolean`                                      |
| any integer or float type                       | `number` (integers within ±2^53)               |
| `[]const u8`, `[]u8`, `[:0]const u8`, `[:0]u8`  | `string` |
| `quickgui.Bytes`                                | `Uint8Array`                                   |
| `void` (return only)                            | `undefined`                                    |
| struct                                          | object with the same field names               |
| slice, array, `@Vector`                         | array (`[]const u8` inside a struct is a string) |
| tuple                                           | array with one element per field               |
| `?T`                                            | `T \| null`                                    |
| `enum`                                          | union of its tag names                         |
| `union(enum)`                                   | `{ tag: payload }`, `{ tag: {} }` for `void`   |
| `std.json.Value`                                | `unknown`                                      |
| `std.mem.Allocator` (parameter only)            | not passed; the runtime injects an arena       |
| `!T` (return only)                              | `T`; an error becomes a thrown `NativeModuleError` |

Scalars and strings cross the boundary directly; everything else travels as JSON through
`std.json`, so a struct field named `oldPath` is the property `oldPath`, an optional field is
`null`, and a field with a default value is optional in the TypeScript type. Struct, enum, and
union types declared with a name become exported `interface`s and `type`s in `index.ts`;
anonymous ones are written inline. Recursive types are supported.

An `Allocator` parameter is the way to return anything you had to allocate: the runtime hands
each call its own arena and frees it after the result has been encoded, so a function builds its
result with `try allocator.alloc(...)` or an `ArrayList` and never frees. Slices returned into a
struct may also point at the input, which lives until the call returns.

```zig
const std = @import("std");
const quickgui = @import("quickgui");

pub const Kind = enum { context, added, removed };
pub const Line = struct { kind: Kind, text: []const u8, number: ?u32 = null };

pub fn parse(allocator: std.mem.Allocator, input: []const u8) ![]Line {
    var lines: std.ArrayList(Line) = .empty;
    var iterator = std.mem.splitScalar(u8, input, '\n');
    while (iterator.next()) |raw| {
        if (raw.len == 0) return error.EmptyLine;
        try lines.append(allocator, .{ .kind = .context, .text = raw });
    }
    return lines.items;
}

pub fn digest(allocator: std.mem.Allocator, input: quickgui.Bytes) !quickgui.Bytes {
    const out = try allocator.alloc(u8, 32);
    std.crypto.hash.sha2.Sha256.hash(input.data, out[0..32], .{});
    return .{ .data = out };
}
```

`@import("quickgui")` is the runtime shipped in `@quickgui/native/zig`; modules only need it for
`Bytes`. A module may import other files beside `main.zig`; every `.zig` file in the module's
directory is part of its build and its change detection.

Raw pointers, untagged unions, error sets outside a return type, and functions are rejected at
compile time with a message naming the type. Function names must be valid JavaScript export names,
and a module cannot declare both `foo` and `fooAsync`.

## Calling a module

```ts
import { parse, parseAsync, digest } from "./modules/stats";

try {
  const lines = parse(text);
} catch (error) {
  if (error instanceof Error) console.error(error.message);
}
```

- `parse(text)` runs on the calling thread and returns the decoded value.
- `parseAsync(text)` copies the arguments, runs the function on the native background threads,
  and resolves on the JavaScript thread. Use it for anything that takes more than a few
  milliseconds, so the application's event loop keeps handling native events and QuickGUI UI updates.
- Arguments are validated before the call: a wrong type, an invalid wire value, or a value that
  cannot be JSON-serialized throws a `TypeError` naming the argument position.
- A Zig error becomes a `NativeModuleError` from `@quickgui/native/modules` whose `code` is the
  error name (`EmptyLine`), and whose `module` and `functionName` say where it came from. The
  runtime's own errors use the same channel: `NotAnInteger` and `IntegerOutOfRange` for a number
  that does not fit the parameter type, `InvalidArgumentEncoding` for a corrupt call, and any
  `std.json` parse error for a malformed structured argument.
- A safety panic in Zig (an out-of-bounds index in `ReleaseSafe`, an `unreachable`) aborts the
  process, like any native code. Return errors instead.

Functions run through `…Async` may execute concurrently on any thread, and a synchronous call can
overlap an asynchronous one. A module that keeps state in module-level variables needs its own
synchronization (`std.atomic.Mutex` for short critical sections, `std.Io.Mutex` otherwise), and
must allocate that state with `quickgui.allocator`, the runtime's thread-safe long-lived
allocator, or an arena built on it; the arena injected into a call is freed when the call
returns.

Every call checks that the addon's own manifest matches the signature `index.ts` was generated
from; a `main.zig` edited after the last build fails at import with a message to run
`quickgui modules` instead of decoding garbage.

## Build, cache, and packaging

```console
quickgui modules                 # compile every module and refresh modules/*/index.ts
quickgui modules --release       # production optimization mode
quickgui modules --target darwin-x64
```

- Modules are compiled with `zig build-lib` in `ReleaseSafe` for development and `ReleaseFast`
  for production; `modules.optimize` in the config fixes one mode for both. `Debug` compiles
  fastest, `ReleaseSafe` keeps bounds checks.
- The static library is written to `.quickgui/modules/<name>/<target>/lib<name>.a`, with Zig's
  cache in `.quickgui/zig-cache`. The input hash includes the module sources, runtime, Zig
  version, target, and optimization mode. macOS archives are repacked for Apple's linker.
- `modules/<name>/index.ts` is rewritten only when its content changes. It declares the module's
  C symbols and typed wrappers. The CLI adds those symbols and the static library to scriptc's
  FFI manifest. Commit the generated file if editors should resolve types before the first build.
- `quickgui dev` watches `.zig` files like any other source and ignores the generated
  `index.ts`, so editing a module rebuilds it and restarts the application; a compile error keeps
  the previous application running and prints Zig's diagnostics.
- `quickgui modules --target` can generate a library for another supported Zig target. Building
  the complete application still requires a matching macOS host.

```ts
// quickgui.config.ts
export default defineConfig({
  name: "My App",
  identifier: "com.example.my-app",
  modules: {
    directory: "modules",   // default
    optimize: "ReleaseFast", // default: ReleaseSafe for dev, ReleaseFast for build
  },
});
```

## What belongs in a module

The boundary is cheap for scalars and strings and proportional to the JSON size for everything
else: a struct result is serialized by `std.json`, copied into a JavaScript buffer, decoded, and
`JSON.parse`d. That gives a simple rule for what pays off:

- **Compute that returns little** wins clearly: hashing, searching, matching, diffing, layout,
  compression, anything whose result is much smaller than its input or than the work it took.
- **Keeping large data native** wins on both time and memory. A module parses once, keeps the
  result in module state allocated from `quickgui.allocator` behind a numeric handle, and answers
  small queries (`rowCount`, `rows(start, end)`, one item on demand) so JavaScript never holds the
  whole structure. Release the state from an explicit `close` call, and register the handle with
  a `FinalizationRegistry` on the JavaScript side so a forgotten one is still freed.
- **Reshaping text into many small objects does not win by itself.** JavaScript substrings share
  their backing store and `JSON.parse` is already the fastest way to build objects from bytes, so
  a parser that returns a 32 MB diff as 700 000 line objects is slower in Zig plus JSON (about
  300 ms, two thirds of it serializing and parsing a 92 MB result) than in TypeScript (47 ms).

Weigh the first rule against the fact that an application's own TypeScript is compiled to native
code too: a module pays off for work Zig does fundamentally better, not for work that is merely
hot. Quick Git parses `git diff` output behind exactly the handle of the second pattern, and
does it in TypeScript (`examples/quick-git/git/diff.ts`) for that reason.

Return to the [documentation index](README.md).
