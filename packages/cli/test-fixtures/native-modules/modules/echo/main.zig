//! Fixture module exercising every value shape a native module can pass.
const std = @import("std");
const quickgui = @import("quickgui");

pub const Kind = enum { context, added, removed };

pub const Line = struct {
    kind: Kind,
    text: []const u8,
    number: ?u32 = null,
};

pub const Hunk = struct {
    heading: []const u8,
    lines: []const Line,
    stats: struct { added: u32, removed: u32 },
};

pub const Node = struct {
    name: []const u8,
    children: []const Node = &.{},
};

pub const Shape = union(enum) {
    circle: f64,
    square: struct { side: f64 },
    none: void,
};

/// Adds two numbers.
pub fn add(a: i32, b: i32) i32 {
    return a + b;
}

pub fn scale(value: f64, factor: f32) f64 {
    return value * factor;
}

pub fn negate(flag: bool) bool {
    return !flag;
}

pub fn nothing() void {}

/// Repeats `text` `count` times.
pub fn repeat(allocator: std.mem.Allocator, text: []const u8, count: u8) ![]const u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    var i: u8 = 0;
    while (i < count) : (i += 1) try out.writer.writeAll(text);
    return out.written();
}

pub fn lengthZ(text: [:0]const u8) usize {
    return std.mem.len(text.ptr);
}

pub fn upper(text: []u8) []const u8 {
    for (text) |*byte| byte.* = std.ascii.toUpper(byte.*);
    return text;
}

pub fn reverseBytes(allocator: std.mem.Allocator, bytes: quickgui.Bytes) !quickgui.Bytes {
    const out = try allocator.dupe(u8, bytes.data);
    std.mem.reverse(u8, out);
    return .{ .data = out };
}

pub fn parseHunk(allocator: std.mem.Allocator, text: []const u8) !Hunk {
    var lines: std.ArrayList(Line) = .empty;
    var added: u32 = 0;
    var removed: u32 = 0;
    var iterator = std.mem.splitScalar(u8, text, '\n');
    var number: u32 = 1;
    while (iterator.next()) |raw| {
        if (raw.len == 0) continue;
        const kind: Kind = switch (raw[0]) {
            '+' => .added,
            '-' => .removed,
            else => .context,
        };
        if (kind == .added) added += 1;
        if (kind == .removed) removed += 1;
        try lines.append(allocator, .{ .kind = kind, .text = raw[1..], .number = if (kind == .removed) null else number });
        if (kind != .removed) number += 1;
    }
    return .{ .heading = "parsed", .lines = lines.items, .stats = .{ .added = added, .removed = removed } };
}

pub fn countLines(hunk: Hunk) u32 {
    return @intCast(hunk.lines.len);
}

pub fn depth(node: Node) u32 {
    var deepest: u32 = 0;
    for (node.children) |child| deepest = @max(deepest, depth(child));
    return deepest + 1;
}

pub fn area(shape: Shape) f64 {
    return switch (shape) {
        .circle => |radius| std.math.pi * radius * radius,
        .square => |square| square.side * square.side,
        .none => 0,
    };
}

pub fn describe(shape: Shape) Shape {
    return shape;
}

pub fn pair(allocator: std.mem.Allocator, first: u8, second: []const u8) !struct { u8, []const u8 } {
    return .{ first, try allocator.dupe(u8, second) };
}

pub fn maybe(value: ?u32) ?u32 {
    return if (value) |v| v * 2 else null;
}

pub fn fail(code: u8) !u32 {
    return switch (code) {
        0 => 1,
        1 => error.NotFound,
        else => error.Invalid,
    };
}

pub fn tooBig() u64 {
    return std.math.maxInt(u64);
}

pub fn sumAll(values: []const f64) f64 {
    var total: f64 = 0;
    for (values) |value| total += value;
    return total;
}

pub fn echoAny(value: std.json.Value) std.json.Value {
    return value;
}

pub fn slowSquare(n: u32) u64 {
    var total: u64 = 0;
    var i: u64 = 0;
    while (i < n) : (i += 1) total += i;
    return total;
}

// Not exported: generic, private, or not a function.
pub fn generic(value: anytype) @TypeOf(value) {
    return value;
}
pub var counter: u32 = 0;
fn private() void {}
