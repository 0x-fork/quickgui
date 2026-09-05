//! QuickGUI native module runtime.
//!
//! A native module is an ordinary Zig source file whose public functions are called from
//! JavaScript. The CLI compiles it into a Node-API addon with this file as the glue: at compile
//! time it reflects over every `pub fn` of the module, derives a manifest describing each
//! function's parameters and result, and generates the code that decodes arguments, calls the
//! function, and encodes the result.
//!
//! Values cross the boundary in one binary encoding shared with `@quickgui/native/modules`:
//!
//! - `bool` is a boolean.
//! - Every integer and float type is a JavaScript number (integers must stay within the safe
//!   integer range, ±2^53).
//! - `[]const u8`, `[]u8`, and their sentinel-terminated forms are strings.
//! - `Bytes` is a `Uint8Array`.
//! - Everything else (structs, slices, arrays, optionals, enums, tagged unions, tuples) travels
//!   as JSON through `std.json`, so a function can return a struct and JavaScript receives an
//!   object with the same field names.
//! - A `std.mem.Allocator` parameter is not passed from JavaScript; the runtime injects an arena
//!   that lives until the result has been encoded, so a function can allocate its result freely
//!   and never free anything.
//! - An error union return reports the error name to JavaScript, which throws it.
//!
//! The addon exports three functions: `manifest()` returning the JSON manifest, `call(index,
//! bytes)` for a synchronous call, and `callAsync(index, bytes)` which runs the function on the
//! host's Node-API thread pool and resolves a promise. Functions invoked through `callAsync` may
//! run concurrently on any thread, so shared module state needs its own synchronization.

const std = @import("std");
const builtin = @import("builtin");
pub const napi = @import("napi.zig");

/// Encoding version shared with `@quickgui/native/modules`.
pub const abi_version: u32 = 1;

/// A byte string that crosses the boundary as a `Uint8Array` instead of a JavaScript string.
pub const Bytes = struct { data: []const u8 };

/// A thread-safe general-purpose allocator for state that outlives one call, such as a parsed
/// document a module keeps behind a handle. The arena injected into a call is freed when the call
/// returns; anything kept in module-level variables must come from this allocator (or one built
/// on it) and be freed by the module itself.
pub const allocator: std.mem.Allocator = std.heap.smp_allocator;

const backing_allocator = allocator;

/// Tag written before every encoded value.
const Wire = enum(u8) {
    void = 0,
    boolean = 1,
    number = 2,
    string = 3,
    bytes = 4,
    json = 5,
};

const status_ok: u8 = 0;
const status_error: u8 = 1;
const max_safe_integer: f64 = 9007199254740991.0;

/// Export `M`'s public functions as a Node-API module. Called from the generated root:
///
/// ```zig
/// comptime { @import("quickgui").exportModule(@import("module")); }
/// ```
pub fn exportModule(comptime M: type) void {
    const Impl = ModuleImpl(M);
    @export(&Impl.register, .{ .name = "napi_register_module_v1" });
}

// --- reflection ----------------------------------------------------------------------------------

/// Names of the module's exported functions, in declaration order. The index of a name is the
/// function's index in the manifest and in `call`/`callAsync`.
fn functionNames(comptime M: type) []const [:0]const u8 {
    comptime {
        @setEvalBranchQuota(100_000);
        var names: []const [:0]const u8 = &.{};
        for (@typeInfo(M).@"struct".decls) |decl| {
            const T = @TypeOf(@field(M, decl.name));
            if (@typeInfo(T) != .@"fn") continue;
            if (@typeInfo(T).@"fn".is_generic) continue;
            names = names ++ &[_][:0]const u8{decl.name};
        }
        return names;
    }
}

fn isByteSlice(comptime T: type) bool {
    const info = @typeInfo(T);
    if (info != .pointer) return false;
    return info.pointer.size == .slice and info.pointer.child == u8;
}

fn wireOf(comptime T: type) Wire {
    if (T == void) return .void;
    if (T == bool) return .boolean;
    if (T == Bytes) return .bytes;
    if (isByteSlice(T)) return .string;
    return switch (@typeInfo(T)) {
        .int, .float => .number,
        else => .json,
    };
}

fn hasJsonMethods(comptime T: type) bool {
    return std.meta.hasFn(T, "jsonStringify") or std.meta.hasFn(T, "jsonParse");
}

fn containsType(comptime list: []const type, comptime T: type) bool {
    for (list) |item| if (item == T) return true;
    return false;
}

fn jsonEscape(comptime text: []const u8) []const u8 {
    comptime {
        var out: []const u8 = "";
        for (text) |byte| {
            out = out ++ switch (byte) {
                '"' => "\\\"",
                '\\' => "\\\\",
                '\n' => "\\n",
                '\t' => "\\t",
                else => &[_]u8{byte},
            };
        }
        return out;
    }
}

fn quoted(comptime text: []const u8) []const u8 {
    return "\"" ++ jsonEscape(text) ++ "\"";
}

/// JSON description of `T` for the TypeScript generator. Named structs, enums, and unions carry
/// their Zig type name so the generator can hoist them into named TypeScript types; a type that is
/// already being described (a recursive structure) becomes a reference to that name.
fn descriptor(comptime T: type, comptime seen: []const type) []const u8 {
    comptime {
        @setEvalBranchQuota(100_000);
        if (T == void) return "{\"kind\":\"void\"}";
        if (T == bool) return "{\"kind\":\"boolean\"}";
        if (T == Bytes) return "{\"kind\":\"bytes\"}";
        if (T == std.json.Value) return "{\"kind\":\"any\"}";
        if (isByteSlice(T)) return "{\"kind\":\"string\"}";
        if (containsType(seen, T)) return "{\"kind\":\"ref\",\"name\":" ++ quoted(@typeName(T)) ++ "}";
        switch (@typeInfo(T)) {
            .int, .float => return "{\"kind\":\"number\"}",
            .optional => |info| return "{\"kind\":\"optional\",\"inner\":" ++ descriptor(info.child, seen) ++ "}",
            .array => |info| {
                if (info.child == u8) return "{\"kind\":\"string\"}";
                return "{\"kind\":\"array\",\"element\":" ++ descriptor(info.child, seen) ++ "}";
            },
            .vector => |info| return "{\"kind\":\"array\",\"element\":" ++ descriptor(info.child, seen) ++ "}",
            .pointer => |info| switch (info.size) {
                .slice => return "{\"kind\":\"array\",\"element\":" ++ descriptor(info.child, seen) ++ "}",
                .one => return descriptor(info.child, seen),
                else => @compileError("native modules cannot pass raw pointers across the boundary: " ++ @typeName(T)),
            },
            .error_union => @compileError("error unions are only supported as the return type of an exported function, found " ++ @typeName(T)),
            .@"enum" => |info| {
                var values: []const u8 = "";
                for (info.fields, 0..) |field, index| {
                    values = values ++ (if (index == 0) "" else ",") ++ quoted(field.name);
                }
                return "{\"kind\":\"enum\",\"name\":" ++ quoted(@typeName(T)) ++ ",\"values\":[" ++ values ++ "]}";
            },
            .@"struct" => |info| {
                if (hasJsonMethods(T)) return "{\"kind\":\"any\"}";
                const nested = seen ++ &[_]type{T};
                if (info.is_tuple) {
                    var elements: []const u8 = "";
                    for (info.fields, 0..) |field, index| {
                        elements = elements ++ (if (index == 0) "" else ",") ++ descriptor(field.type, nested);
                    }
                    return "{\"kind\":\"tuple\",\"elements\":[" ++ elements ++ "]}";
                }
                var fields: []const u8 = "";
                for (info.fields, 0..) |field, index| {
                    fields = fields ++ (if (index == 0) "" else ",") ++
                        "{\"name\":" ++ quoted(field.name) ++
                        ",\"type\":" ++ descriptor(field.type, nested) ++
                        ",\"hasDefault\":" ++ (if (field.default_value_ptr != null) "true" else "false") ++ "}";
                }
                return "{\"kind\":\"struct\",\"name\":" ++ quoted(@typeName(T)) ++ ",\"fields\":[" ++ fields ++ "]}";
            },
            .@"union" => |info| {
                if (hasJsonMethods(T)) return "{\"kind\":\"any\"}";
                if (info.tag_type == null) @compileError("only tagged unions can cross the boundary, found " ++ @typeName(T));
                const nested = seen ++ &[_]type{T};
                var variants: []const u8 = "";
                for (info.fields, 0..) |field, index| {
                    variants = variants ++ (if (index == 0) "" else ",") ++
                        "{\"name\":" ++ quoted(field.name) ++ ",\"type\":" ++ descriptor(field.type, nested) ++ "}";
                }
                return "{\"kind\":\"union\",\"name\":" ++ quoted(@typeName(T)) ++ ",\"variants\":[" ++ variants ++ "]}";
            },
            else => @compileError("unsupported type in a native module signature: " ++ @typeName(T)),
        }
    }
}

fn wireName(comptime wire: Wire) []const u8 {
    return @tagName(wire);
}

fn valueSpec(comptime T: type) []const u8 {
    return "{\"wire\":" ++ quoted(wireName(wireOf(T))) ++ ",\"type\":" ++ descriptor(T, &.{}) ++ "}";
}

fn functionSpec(comptime name: []const u8, comptime F: type) []const u8 {
    comptime {
        const info = @typeInfo(F).@"fn";
        var params: []const u8 = "";
        for (info.params, 0..) |param, index| {
            const P = param.type orelse @compileError("native module function " ++ name ++ " has a parameter without a type");
            const spec = if (P == std.mem.Allocator) "{\"injected\":\"allocator\"}" else valueSpec(P);
            params = params ++ (if (index == 0) "" else ",") ++ spec;
        }
        const R = info.return_type orelse @compileError("native module function " ++ name ++ " has no return type");
        const payload = if (@typeInfo(R) == .error_union) @typeInfo(R).error_union.payload else R;
        return "{\"name\":" ++ quoted(name) ++ ",\"params\":[" ++ params ++ "],\"result\":" ++ valueSpec(payload) ++ "}";
    }
}

fn buildManifest(comptime M: type, comptime names: []const [:0]const u8) []const u8 {
    comptime {
        @setEvalBranchQuota(1_000_000);
        var functions: []const u8 = "";
        for (names, 0..) |name, index| {
            functions = functions ++ (if (index == 0) "" else ",") ++ functionSpec(name, @TypeOf(@field(M, name)));
        }
        return "{\"abi\":" ++ std.fmt.comptimePrint("{d}", .{abi_version}) ++
            ",\"zig\":" ++ quoted(builtin.zig_version_string) ++
            ",\"functions\":[" ++ functions ++ "]}";
    }
}

// --- encoding ------------------------------------------------------------------------------------

const DecodeError = error{
    InvalidArgumentEncoding,
    NotAnInteger,
    IntegerOutOfRange,
    OutOfMemory,
};

const Cursor = struct {
    bytes: []const u8,
    offset: usize = 0,

    fn take(self: *Cursor, count: usize) DecodeError![]const u8 {
        if (self.bytes.len - self.offset < count) return error.InvalidArgumentEncoding;
        const slice = self.bytes[self.offset .. self.offset + count];
        self.offset += count;
        return slice;
    }

    fn expectTag(self: *Cursor, wire: Wire) DecodeError!void {
        const tag = try self.take(1);
        if (tag[0] != @intFromEnum(wire)) return error.InvalidArgumentEncoding;
    }

    fn takeU32(self: *Cursor) DecodeError!u32 {
        const raw = try self.take(4);
        return std.mem.readInt(u32, raw[0..4], .little);
    }

    fn takeF64(self: *Cursor) DecodeError!f64 {
        const raw = try self.take(8);
        return @bitCast(std.mem.readInt(u64, raw[0..8], .little));
    }

    fn takeSized(self: *Cursor) DecodeError![]const u8 {
        const length = try self.takeU32();
        return self.take(length);
    }
};

fn numberTo(comptime T: type, value: f64) DecodeError!T {
    switch (@typeInfo(T)) {
        .float => return @floatCast(value),
        .int => {
            if (!std.math.isFinite(value) or @floor(value) != value) return error.NotAnInteger;
            if (value > max_safe_integer or value < -max_safe_integer) return error.IntegerOutOfRange;
            const integer: i64 = @intFromFloat(value);
            return std.math.cast(T, integer) orelse error.IntegerOutOfRange;
        },
        else => unreachable,
    }
}

fn stringTo(comptime T: type, bytes: []const u8, arena: std.mem.Allocator) DecodeError!T {
    const info = @typeInfo(T).pointer;
    if (info.sentinel() != null) {
        return try arena.dupeZ(u8, bytes);
    }
    if (!info.is_const) return try arena.dupe(u8, bytes);
    return bytes;
}

fn decodeValue(comptime T: type, cursor: *Cursor, arena: std.mem.Allocator) !T {
    switch (comptime wireOf(T)) {
        .void => {
            try cursor.expectTag(.void);
            return {};
        },
        .boolean => {
            try cursor.expectTag(.boolean);
            const raw = try cursor.take(1);
            return raw[0] != 0;
        },
        .number => {
            try cursor.expectTag(.number);
            return numberTo(T, try cursor.takeF64());
        },
        .string => {
            try cursor.expectTag(.string);
            return stringTo(T, try cursor.takeSized(), arena);
        },
        .bytes => {
            try cursor.expectTag(.bytes);
            return Bytes{ .data = try cursor.takeSized() };
        },
        .json => {
            try cursor.expectTag(.json);
            const text = try cursor.takeSized();
            return std.json.parseFromSliceLeaky(T, arena, text, .{ .ignore_unknown_fields = true });
        },
    }
}

fn writeU32(out: *std.Io.Writer.Allocating, value: u32) !void {
    var raw: [4]u8 = undefined;
    std.mem.writeInt(u32, &raw, value, .little);
    try out.writer.writeAll(&raw);
}

fn writeSized(out: *std.Io.Writer.Allocating, wire: Wire, bytes: []const u8) !void {
    try out.writer.writeByte(@intFromEnum(wire));
    try writeU32(out, std.math.cast(u32, bytes.len) orelse return error.ResultTooLarge);
    try out.writer.writeAll(bytes);
}

fn encodeValue(comptime T: type, value: T, out: *std.Io.Writer.Allocating) !void {
    switch (comptime wireOf(T)) {
        .void => try out.writer.writeByte(@intFromEnum(Wire.void)),
        .boolean => {
            try out.writer.writeByte(@intFromEnum(Wire.boolean));
            try out.writer.writeByte(@intFromBool(value));
        },
        .number => {
            const number: f64 = switch (@typeInfo(T)) {
                .float => @floatCast(value),
                .int => blk: {
                    const number: f64 = @floatFromInt(value);
                    if (@abs(number) > max_safe_integer) return error.IntegerOutOfRange;
                    break :blk number;
                },
                else => unreachable,
            };
            try out.writer.writeByte(@intFromEnum(Wire.number));
            var raw: [8]u8 = undefined;
            std.mem.writeInt(u64, &raw, @bitCast(number), .little);
            try out.writer.writeAll(&raw);
        },
        .string => try writeSized(out, .string, value),
        .bytes => try writeSized(out, .bytes, value.data),
        .json => {
            try out.writer.writeByte(@intFromEnum(Wire.json));
            const length_offset = out.written().len;
            try writeU32(out, 0);
            try std.json.Stringify.value(value, .{}, &out.writer);
            const length = out.written().len - length_offset - 4;
            const patch = out.written()[length_offset..][0..4];
            std.mem.writeInt(u32, patch, std.math.cast(u32, length) orelse return error.ResultTooLarge, .little);
        },
    }
}

fn writeError(out: *std.Io.Writer.Allocating, name: []const u8) !void {
    out.clearRetainingCapacity();
    try out.writer.writeByte(status_error);
    try writeU32(out, std.math.cast(u32, name.len) orelse return error.ResultTooLarge);
    try out.writer.writeAll(name);
}

// --- dispatch ------------------------------------------------------------------------------------

fn invokeFunction(comptime func: anytype, args: []const u8, arena: std.mem.Allocator, out: *std.Io.Writer.Allocating) !void {
    const F = @TypeOf(func);
    const info = @typeInfo(F).@"fn";
    var tuple: std.meta.ArgsTuple(F) = undefined;
    var cursor = Cursor{ .bytes = args };
    inline for (info.params, 0..) |param, index| {
        const P = param.type.?;
        if (P == std.mem.Allocator) {
            tuple[index] = arena;
        } else {
            tuple[index] = try decodeValue(P, &cursor, arena);
        }
    }
    if (cursor.offset != args.len) return error.InvalidArgumentEncoding;
    const R = info.return_type.?;
    try out.writer.writeByte(status_ok);
    if (@typeInfo(R) == .error_union) {
        const value = try @call(.auto, func, tuple);
        try encodeValue(@typeInfo(R).error_union.payload, value, out);
    } else {
        const value = @call(.auto, func, tuple);
        try encodeValue(R, value, out);
    }
}

fn ModuleImpl(comptime M: type) type {
    return struct {
        const names = functionNames(M);
        const manifest_json = buildManifest(M, names);

        fn dispatch(index: u32, args: []const u8, arena: std.mem.Allocator, out: *std.Io.Writer.Allocating) !void {
            inline for (names, 0..) |name, position| {
                if (index == position) return invokeFunction(@field(M, name), args, arena, out);
            }
            return error.UnknownFunction;
        }

        /// Run function `index` with encoded `args` and return the encoded result, owned by the
        /// backing allocator. Every failure but running out of memory is reported inside the
        /// encoding so JavaScript can throw it.
        fn invoke(index: u32, args: []const u8) error{OutOfMemory}![]u8 {
            var out = std.Io.Writer.Allocating.init(backing_allocator);
            errdefer out.deinit();
            var arena = std.heap.ArenaAllocator.init(backing_allocator);
            defer arena.deinit();
            dispatch(index, args, arena.allocator(), &out) catch |err| {
                writeError(&out, @errorName(err)) catch return error.OutOfMemory;
            };
            return out.toOwnedSlice() catch return error.OutOfMemory;
        }

        // --- Node-API surface ---

        fn register(env: napi.Env, exports: napi.Value) callconv(.c) napi.Value {
            defineFunction(env, exports, "manifest", manifest);
            defineFunction(env, exports, "call", call);
            defineFunction(env, exports, "callAsync", callAsync);
            return exports;
        }

        fn manifest(env: napi.Env, _: napi.CallbackInfo) callconv(.c) ?napi.Value {
            var value: napi.Value = undefined;
            if (napi.napi_create_string_utf8(env, manifest_json.ptr, manifest_json.len, &value) != .ok) {
                return throwError(env, "could not create the native module manifest");
            }
            return value;
        }

        fn call(env: napi.Env, info: napi.CallbackInfo) callconv(.c) ?napi.Value {
            const request = readCallArguments(env, info) orelse return null;
            const result = invoke(request.index, request.args) catch return throwError(env, "OutOfMemory");
            defer backing_allocator.free(result);
            return resultBuffer(env, result);
        }

        fn callAsync(env: napi.Env, info: napi.CallbackInfo) callconv(.c) ?napi.Value {
            const request = readCallArguments(env, info) orelse return null;
            const work = backing_allocator.create(Work) catch return throwError(env, "OutOfMemory");
            work.* = .{
                .index = request.index,
                .args = backing_allocator.dupe(u8, request.args) catch {
                    backing_allocator.destroy(work);
                    return throwError(env, "OutOfMemory");
                },
            };
            var promise: napi.Value = undefined;
            if (napi.napi_create_promise(env, &work.deferred, &promise) != .ok) {
                work.destroy();
                return throwError(env, "could not create a promise for the native module call");
            }
            var name: napi.Value = undefined;
            const resource_name = "quickgui:native-module";
            if (napi.napi_create_string_utf8(env, resource_name.ptr, resource_name.len, &name) != .ok or
                napi.napi_create_async_work(env, null, name, execute, complete, work, &work.work) != .ok)
            {
                _ = napi.napi_reject_deferred(env, work.deferred, errorValue(env, "could not schedule the native module call"));
                work.destroy();
                return promise;
            }
            if (napi.napi_queue_async_work(env, work.work) != .ok) {
                _ = napi.napi_reject_deferred(env, work.deferred, errorValue(env, "could not queue the native module call"));
                _ = napi.napi_delete_async_work(env, work.work);
                work.destroy();
                return promise;
            }
            return promise;
        }

        const Work = struct {
            index: u32,
            args: []u8,
            result: ?[]u8 = null,
            deferred: napi.Deferred = undefined,
            work: napi.AsyncWork = undefined,

            fn destroy(self: *Work) void {
                backing_allocator.free(self.args);
                if (self.result) |result| backing_allocator.free(result);
                backing_allocator.destroy(self);
            }
        };

        /// Runs on a thread-pool thread: no Node-API calls are allowed here.
        fn execute(_: napi.Env, data: ?*anyopaque) callconv(.c) void {
            const work: *Work = @ptrCast(@alignCast(data.?));
            work.result = invoke(work.index, work.args) catch null;
        }

        fn complete(env: napi.Env, status: napi.Status, data: ?*anyopaque) callconv(.c) void {
            const work: *Work = @ptrCast(@alignCast(data.?));
            defer work.destroy();
            _ = napi.napi_delete_async_work(env, work.work);
            if (status != .ok) {
                _ = napi.napi_reject_deferred(env, work.deferred, errorValue(env, "the native module call was cancelled"));
                return;
            }
            const result = work.result orelse {
                _ = napi.napi_reject_deferred(env, work.deferred, errorValue(env, "OutOfMemory"));
                return;
            };
            const buffer = resultBuffer(env, result) orelse {
                _ = napi.napi_reject_deferred(env, work.deferred, errorValue(env, "could not copy the native module result"));
                return;
            };
            _ = napi.napi_resolve_deferred(env, work.deferred, buffer);
        }
    };
}

// --- Node-API helpers ----------------------------------------------------------------------------

const CallRequest = struct { index: u32, args: []const u8 };

fn readCallArguments(env: napi.Env, info: napi.CallbackInfo) ?CallRequest {
    var argc: usize = 2;
    var argv: [2]napi.Value = undefined;
    if (napi.napi_get_cb_info(env, info, &argc, &argv, null, null) != .ok or argc < 2) {
        return throwTypeError(env, "a native module call needs a function index and an argument buffer");
    }
    var index: u32 = 0;
    if (napi.napi_get_value_uint32(env, argv[0], &index) != .ok) {
        return throwTypeError(env, "the native module function index must be an unsigned integer");
    }
    var kind: napi.TypedArrayType = .uint8;
    var length: usize = 0;
    var data: ?*anyopaque = null;
    if (napi.napi_get_typedarray_info(env, argv[1], &kind, &length, &data, null, null) != .ok or kind != .uint8) {
        return throwTypeError(env, "native module arguments must be encoded in a Uint8Array");
    }
    const args: []const u8 = if (length == 0 or data == null) &.{} else @as([*]const u8, @ptrCast(data.?))[0..length];
    return .{ .index = index, .args = args };
}

fn resultBuffer(env: napi.Env, result: []const u8) ?napi.Value {
    var value: napi.Value = undefined;
    if (napi.napi_create_buffer_copy(env, result.len, result.ptr, null, &value) != .ok) {
        return throwError(env, "could not copy the native module result");
    }
    return value;
}

fn errorValue(env: napi.Env, message: []const u8) napi.Value {
    var text: napi.Value = undefined;
    _ = napi.napi_create_string_utf8(env, message.ptr, message.len, &text);
    var value: napi.Value = undefined;
    _ = napi.napi_create_error(env, null, text, &value);
    return value;
}

fn throwError(env: napi.Env, message: [*:0]const u8) ?napi.Value {
    _ = napi.napi_throw_error(env, null, message);
    return null;
}

fn throwTypeError(env: napi.Env, message: [*:0]const u8) ?CallRequest {
    _ = napi.napi_throw_type_error(env, null, message);
    return null;
}

fn defineFunction(env: napi.Env, exports: napi.Value, comptime name: [:0]const u8, callback: napi.Callback) void {
    var function: napi.Value = undefined;
    if (napi.napi_create_function(env, name.ptr, name.len, callback, null, &function) != .ok) return;
    _ = napi.napi_set_named_property(env, exports, name.ptr, function);
}
