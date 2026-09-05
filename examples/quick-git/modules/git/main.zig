//! Native diff engine for Quick Git.
//!
//! A diff is parsed once from the raw bytes git wrote and kept here, behind a numeric handle,
//! for as long as the app shows it. JavaScript never holds the parsed diff: it asks for the row
//! count, the rows the table is about to paint, the changed lines inside a selection, one file
//! when it builds a patch, or the rendered text for an agent prompt. Each answer is small, so a
//! 32 MB diff costs one parse off the JavaScript thread and a few kilobytes per scroll instead of
//! hundreds of thousands of JavaScript objects.
//!
//! Field names are camelCase on purpose: they are the JavaScript property names. `parseDiff` is
//! also exported eagerly, for tests and small diffs, and `git/diff.test.ts` exercises everything
//! through the generated `index.ts`.

const std = @import("std");
const quickgui = @import("quickgui");
const Allocator = std.mem.Allocator;

// --- diffs ---------------------------------------------------------------------------------------

pub const DiffLineKind = enum { context, added, removed };

pub const DiffLine = struct {
    kind: DiffLineKind,
    /// Line content without the leading marker or trailing newline.
    text: []const u8,
    /// Line number in the old file, `null` for added lines.
    oldLineNumber: ?u64,
    /// Line number in the new file, `null` for removed lines.
    newLineNumber: ?u64,
    /// Whether git printed `\ No newline at end of file` after this line.
    noNewline: bool,
};

pub const DiffHunk = struct {
    oldStart: u64,
    oldLines: u64,
    newStart: u64,
    newLines: u64,
    /// Text after the closing `@@`, typically the enclosing function.
    heading: []const u8,
    lines: []DiffLine,
};

pub const DiffFileKind = enum { modified, added, deleted, renamed, copied };

pub const DiffFile = struct {
    kind: DiffFileKind,
    /// Path on the old side, `null` for an added file.
    oldPath: ?[]const u8,
    /// Path on the new side, `null` for a deleted file.
    newPath: ?[]const u8,
    /// The path the app shows: the new path, or the old one for a deletion.
    path: []const u8,
    binary: bool,
    oldMode: ?[]const u8 = null,
    newMode: ?[]const u8 = null,
    similarity: ?u32 = null,
    hunks: []DiffHunk,
    /// Whether the diff was cut short by the output bound.
    truncated: bool,
};

pub const HeaderPaths = struct {
    oldPath: []const u8,
    newPath: []const u8,
};

/// Parse the complete output of `git diff` into files. `truncated` marks the last file when the
/// output hit the app's byte bound.
pub fn parseDiff(allocator: Allocator, output: []const u8, truncated: bool) ![]DiffFile {
    const text = try sanitizeUtf8(allocator, output);
    const lines = try splitLines(allocator, text);
    var files: std.ArrayList(DiffFile) = .empty;
    var index: usize = 0;
    while (index < lines.len) {
        const line = lines[index];
        if (!std.mem.startsWith(u8, line, "diff --git ")) {
            index += 1;
            continue;
        }
        const paths = parseGitHeaderPaths(line["diff --git ".len..]);
        var file = DiffFile{
            .kind = .modified,
            .oldPath = paths.oldPath,
            .newPath = paths.newPath,
            .path = paths.newPath,
            .binary = false,
            .hunks = &.{},
            .truncated = false,
        };
        index += 1;
        // Extended header lines up to the first hunk, `---`, or next file.
        while (index < lines.len) {
            const header = lines[index];
            if (std.mem.startsWith(u8, header, "diff --git ") or std.mem.startsWith(u8, header, "@@")) break;
            index += 1;
            if (cutPrefix(header, "old mode ")) |rest| {
                file.oldMode = trim(rest);
            } else if (cutPrefix(header, "new mode ")) |rest| {
                file.newMode = trim(rest);
            } else if (cutPrefix(header, "deleted file mode ")) |rest| {
                file.kind = .deleted;
                file.oldMode = trim(rest);
            } else if (cutPrefix(header, "new file mode ")) |rest| {
                file.kind = .added;
                file.newMode = trim(rest);
            } else if (cutPrefix(header, "similarity index ")) |rest| {
                file.similarity = leadingInteger(u32, rest);
            } else if (cutPrefix(header, "rename from ")) |rest| {
                file.kind = .renamed;
                file.oldPath = rest;
            } else if (cutPrefix(header, "rename to ")) |rest| {
                file.kind = .renamed;
                file.newPath = rest;
            } else if (cutPrefix(header, "copy from ")) |rest| {
                file.kind = .copied;
                file.oldPath = rest;
            } else if (cutPrefix(header, "copy to ")) |rest| {
                file.kind = .copied;
                file.newPath = rest;
            } else if (std.mem.startsWith(u8, header, "Binary files ")) {
                file.binary = true;
            } else if (cutPrefix(header, "--- ")) |rest| {
                file.oldPath = stripPrefix(rest, "a/");
            } else if (cutPrefix(header, "+++ ")) |rest| {
                file.newPath = stripPrefix(rest, "b/");
            }
        }
        if (file.kind == .added) file.oldPath = null;
        if (file.kind == .deleted) file.newPath = null;

        var hunks: std.ArrayList(DiffHunk) = .empty;
        while (index < lines.len and std.mem.startsWith(u8, lines[index], "@@")) {
            const header = parseHunkHeader(lines[index]);
            index += 1;
            var hunk = header orelse break;
            var hunkLines: std.ArrayList(DiffLine) = .empty;
            var oldLine = hunk.oldStart;
            var newLine = hunk.newStart;
            while (index < lines.len) {
                const body = lines[index];
                if (std.mem.startsWith(u8, body, "diff --git ") or std.mem.startsWith(u8, body, "@@")) break;
                index += 1;
                if (body.len > 0 and body[0] == '\\') {
                    if (hunkLines.items.len > 0) hunkLines.items[hunkLines.items.len - 1].noNewline = true;
                    continue;
                }
                const marker: u8 = if (body.len > 0) body[0] else ' ';
                const content = if (body.len > 0) body[1..] else body;
                switch (marker) {
                    '+' => {
                        try hunkLines.append(allocator, .{ .kind = .added, .text = content, .oldLineNumber = null, .newLineNumber = newLine, .noNewline = false });
                        newLine += 1;
                    },
                    '-' => {
                        try hunkLines.append(allocator, .{ .kind = .removed, .text = content, .oldLineNumber = oldLine, .newLineNumber = null, .noNewline = false });
                        oldLine += 1;
                    },
                    ' ' => {
                        try hunkLines.append(allocator, .{ .kind = .context, .text = content, .oldLineNumber = oldLine, .newLineNumber = newLine, .noNewline = false });
                        oldLine += 1;
                        newLine += 1;
                    },
                    // Anything else ends the hunk (for example a truncated tail).
                    else => break,
                }
            }
            hunk.lines = hunkLines.items;
            try hunks.append(allocator, hunk);
        }
        file.hunks = hunks.items;
        file.path = file.newPath orelse file.oldPath orelse "";
        try files.append(allocator, file);
    }
    if (truncated and files.items.len > 0) files.items[files.items.len - 1].truncated = true;
    return files.items;
}

/// Split the `a/<old> b/<new>` pair of a `diff --git` line.
///
/// Paths may contain spaces, so the split point is the ` b/` whose two halves agree when the file
/// kept its name; otherwise the first ` b/` after `a/` is used and later header lines correct it.
pub fn parseGitHeaderPaths(pair: []const u8) HeaderPaths {
    var first: ?usize = null;
    var search: usize = 0;
    while (std.mem.indexOfPos(u8, pair, search, " b/")) |at| {
        if (first == null) first = at;
        const oldPath = pair[0..at];
        const newPath = pair[at + 1 ..];
        if (std.mem.startsWith(u8, oldPath, "a/") and std.mem.startsWith(u8, newPath, "b/") and
            std.mem.eql(u8, oldPath[2..], newPath[2..]))
        {
            return .{ .oldPath = oldPath[2..], .newPath = newPath[2..] };
        }
        search = at + 1;
    }
    if (first) |at| {
        if (std.mem.startsWith(u8, pair, "a/")) return .{ .oldPath = pair[2..at], .newPath = pair[at + 3 ..] };
    }
    // Quoted or unusual header: fall back to the whole text on both sides.
    var unquoted = pair;
    if (unquoted.len > 0 and unquoted[0] == '"') unquoted = unquoted[1..];
    if (unquoted.len > 0 and unquoted[unquoted.len - 1] == '"') unquoted = unquoted[0 .. unquoted.len - 1];
    return .{ .oldPath = unquoted, .newPath = unquoted };
}

fn parseHunkHeader(line: []const u8) ?DiffHunk {
    var rest = cutPrefix(line, "@@ -") orelse return null;
    const oldStart = takeInteger(&rest) orelse return null;
    var oldLines: u64 = 1;
    if (rest.len > 0 and rest[0] == ',') {
        rest = rest[1..];
        oldLines = takeInteger(&rest) orelse return null;
    }
    rest = cutPrefix(rest, " +") orelse return null;
    const newStart = takeInteger(&rest) orelse return null;
    var newLines: u64 = 1;
    if (rest.len > 0 and rest[0] == ',') {
        rest = rest[1..];
        newLines = takeInteger(&rest) orelse return null;
    }
    rest = cutPrefix(rest, " @@") orelse return null;
    if (rest.len > 0 and rest[0] == ' ') rest = rest[1..];
    return .{
        .oldStart = oldStart,
        .oldLines = oldLines,
        .newStart = newStart,
        .newLines = newLines,
        .heading = rest,
        .lines = &.{},
    };
}

fn stripPrefix(path: []const u8, prefix: []const u8) ?[]const u8 {
    if (std.mem.eql(u8, path, "/dev/null")) return null;
    // A tab may follow the path when it carries a timestamp; git never adds one, but be safe.
    const clean = if (std.mem.indexOfScalar(u8, path, '\t')) |tab| path[0..tab] else path;
    return cutPrefix(clean, prefix) orelse clean;
}

// --- retained diffs ------------------------------------------------------------------------------

/// What the app knows about a file without holding its hunks.
pub const DiffFileSummary = struct {
    kind: DiffFileKind,
    oldPath: ?[]const u8,
    newPath: ?[]const u8,
    path: []const u8,
    binary: bool,
    oldMode: ?[]const u8 = null,
    newMode: ?[]const u8 = null,
    similarity: ?u32 = null,
    hunkCount: u32,
    lineCount: u32,
    added: u32,
    removed: u32,
    truncated: bool,
};

/// The answer to `openDiff`: the handle plus everything the app shows without rows.
pub const DiffSummary = struct {
    id: u32,
    files: []DiffFileSummary,
    /// Rows of the diff table: file headers when there is more than one file, hunk headers,
    /// lines up to `maxRows`, and notices.
    rowCount: u32,
    added: u32,
    removed: u32,
};

pub const DiffRowKind = enum { file, hunk, line, notice };

/// One row of the diff table, ready to paint.
pub const DiffRow = struct {
    kind: DiffRowKind,
    fileIndex: u32,
    hunkIndex: u32,
    lineIndex: u32,
    /// `context` for rows that are not lines.
    lineKind: DiffLineKind,
    /// The line text, the hunk header, the file path, or the notice.
    text: []const u8,
    oldLineNumber: ?u64,
    newLineNumber: ?u64,
    noNewline: bool,
};

/// Changed lines of one hunk inside a selection, for patch formatting.
pub const DiffLineGroup = struct {
    fileIndex: u32,
    hunkIndex: u32,
    lines: []u32,
};

const NoticeKind = enum(u32) { binary, truncatedFile, tooManyRows };

const RowRef = struct {
    kind: DiffRowKind,
    fileIndex: u32,
    hunkIndex: u32,
    /// The line index, or the notice kind for notice rows.
    lineIndex: u32,
};

const ParsedDiff = struct {
    arena: std.heap.ArenaAllocator,
    files: []DiffFile,
    rows: []RowRef,
    added: u32,
    removed: u32,
};

/// Most bytes of one line handed to the table; the view clips long lines itself.
const max_row_text_bytes = 4096;

var registry_mutex: std.atomic.Mutex = .unlocked;
var registry: std.AutoHashMapUnmanaged(u32, *ParsedDiff) = .empty;
var next_id: u32 = 1;

/// The registry is touched for microseconds at a time, so contention is resolved by spinning.
fn lockRegistry() void {
    while (!registry_mutex.tryLock()) std.atomic.spinLoopHint();
}

fn unlockRegistry() void {
    registry_mutex.unlock();
}

/// Parse `output` and keep the result until `closeDiff`. Safe to call through the thread pool.
pub fn openDiff(allocator: Allocator, output: []const u8, truncated: bool, maxRows: u32) !DiffSummary {
    const parsed = try quickgui.allocator.create(ParsedDiff);
    errdefer quickgui.allocator.destroy(parsed);
    parsed.* = .{ .arena = std.heap.ArenaAllocator.init(quickgui.allocator), .files = &.{}, .rows = &.{}, .added = 0, .removed = 0 };
    errdefer parsed.arena.deinit();
    const retained = parsed.arena.allocator();
    const copy = try retained.dupe(u8, output);
    parsed.files = try parseDiff(retained, copy, truncated);

    var rows: std.ArrayList(RowRef) = .empty;
    var summaries = try allocator.alloc(DiffFileSummary, parsed.files.len);
    for (parsed.files, 0..) |file, fileIndex| {
        var added: u32 = 0;
        var removed: u32 = 0;
        var lineCount: u32 = 0;
        const index: u32 = @intCast(fileIndex);
        if (parsed.files.len > 1) try rows.append(retained, .{ .kind = .file, .fileIndex = index, .hunkIndex = 0, .lineIndex = 0 });
        if (file.binary) {
            try rows.append(retained, .{ .kind = .notice, .fileIndex = index, .hunkIndex = 0, .lineIndex = @intFromEnum(NoticeKind.binary) });
        } else {
            for (file.hunks, 0..) |hunk, hunkIndex| {
                try rows.append(retained, .{ .kind = .hunk, .fileIndex = index, .hunkIndex = @intCast(hunkIndex), .lineIndex = 0 });
                for (hunk.lines, 0..) |line, lineIndex| {
                    lineCount += 1;
                    switch (line.kind) {
                        .added => added += 1,
                        .removed => removed += 1,
                        .context => {},
                    }
                    if (rows.items.len >= maxRows) continue;
                    try rows.append(retained, .{ .kind = .line, .fileIndex = index, .hunkIndex = @intCast(hunkIndex), .lineIndex = @intCast(lineIndex) });
                }
            }
            if (file.truncated) {
                try rows.append(retained, .{ .kind = .notice, .fileIndex = index, .hunkIndex = 0, .lineIndex = @intFromEnum(NoticeKind.truncatedFile) });
            }
        }
        parsed.added += added;
        parsed.removed += removed;
        summaries[fileIndex] = .{
            .kind = file.kind,
            .oldPath = file.oldPath,
            .newPath = file.newPath,
            .path = file.path,
            .binary = file.binary,
            .oldMode = file.oldMode,
            .newMode = file.newMode,
            .similarity = file.similarity,
            .hunkCount = @intCast(file.hunks.len),
            .lineCount = lineCount,
            .added = added,
            .removed = removed,
            .truncated = file.truncated,
        };
    }
    if (rows.items.len >= maxRows) {
        try rows.append(retained, .{ .kind = .notice, .fileIndex = 0, .hunkIndex = 0, .lineIndex = @intFromEnum(NoticeKind.tooManyRows) });
    }
    parsed.rows = rows.items;

    lockRegistry();
    defer unlockRegistry();
    const id = next_id;
    try registry.put(quickgui.allocator, id, parsed);
    next_id += 1;
    return .{
        .id = id,
        .files = summaries,
        .rowCount = @intCast(parsed.rows.len),
        .added = parsed.added,
        .removed = parsed.removed,
    };
}

/// Release a diff. Returns false when the handle is unknown (already closed).
pub fn closeDiff(id: u32) bool {
    lockRegistry();
    const entry = registry.fetchRemove(id);
    unlockRegistry();
    const parsed = (entry orelse return false).value;
    parsed.arena.deinit();
    quickgui.allocator.destroy(parsed);
    return true;
}

/// Number of diffs currently held, for tests.
pub fn openDiffCount() u32 {
    lockRegistry();
    defer unlockRegistry();
    return registry.count();
}

fn lookup(id: u32) !*ParsedDiff {
    lockRegistry();
    defer unlockRegistry();
    return registry.get(id) orelse error.UnknownDiff;
}

/// The table rows in `[start, end)`, clipped to the diff.
pub fn diffRows(allocator: Allocator, id: u32, start: u32, end: u32) ![]DiffRow {
    const parsed = try lookup(id);
    const last = @min(end, parsed.rows.len);
    if (start >= last) return &.{};
    const rows = try allocator.alloc(DiffRow, last - start);
    for (parsed.rows[start..last], 0..) |ref, offset| {
        const file = parsed.files[ref.fileIndex];
        rows[offset] = switch (ref.kind) {
            .file => .{ .kind = .file, .fileIndex = ref.fileIndex, .hunkIndex = 0, .lineIndex = 0, .lineKind = .context, .text = file.path, .oldLineNumber = null, .newLineNumber = null, .noNewline = false },
            .hunk => blk: {
                const hunk = file.hunks[ref.hunkIndex];
                const text = if (hunk.heading.len > 0)
                    try std.fmt.allocPrint(allocator, "@@ -{d},{d} +{d},{d} @@ {s}", .{ hunk.oldStart, hunk.oldLines, hunk.newStart, hunk.newLines, hunk.heading })
                else
                    try std.fmt.allocPrint(allocator, "@@ -{d},{d} +{d},{d} @@", .{ hunk.oldStart, hunk.oldLines, hunk.newStart, hunk.newLines });
                break :blk .{ .kind = .hunk, .fileIndex = ref.fileIndex, .hunkIndex = ref.hunkIndex, .lineIndex = 0, .lineKind = .context, .text = text, .oldLineNumber = null, .newLineNumber = null, .noNewline = false };
            },
            .line => blk: {
                const line = file.hunks[ref.hunkIndex].lines[ref.lineIndex];
                break :blk .{ .kind = .line, .fileIndex = ref.fileIndex, .hunkIndex = ref.hunkIndex, .lineIndex = ref.lineIndex, .lineKind = line.kind, .text = clipUtf8(line.text, max_row_text_bytes), .oldLineNumber = line.oldLineNumber, .newLineNumber = line.newLineNumber, .noNewline = line.noNewline };
            },
            .notice => .{ .kind = .notice, .fileIndex = ref.fileIndex, .hunkIndex = 0, .lineIndex = 0, .lineKind = .context, .text = switch (@as(NoticeKind, @enumFromInt(ref.lineIndex))) {
                .binary => "Binary file",
                .truncatedFile => "Diff truncated: the file is too large to show in full.",
                .tooManyRows => "Diff truncated: too many lines to show.",
            }, .oldLineNumber = null, .newLineNumber = null, .noNewline = false },
        };
    }
    return rows;
}

/// One file with all its hunks, for building a patch.
pub fn diffFile(id: u32, fileIndex: u32) !DiffFile {
    const parsed = try lookup(id);
    if (fileIndex >= parsed.files.len) return error.UnknownFile;
    return parsed.files[fileIndex];
}

/// The changed lines inside the table's selected row ranges (inclusive), grouped by hunk in the
/// order they were first seen.
pub fn diffSelectedLines(allocator: Allocator, id: u32, ranges: []const [2]u32) ![]DiffLineGroup {
    const parsed = try lookup(id);
    const Group = struct { fileIndex: u32, hunkIndex: u32, lines: std.AutoArrayHashMapUnmanaged(u32, void) };
    var groups: std.AutoArrayHashMapUnmanaged(u64, Group) = .empty;
    for (ranges) |range| {
        var index: u64 = range[0];
        while (index <= range[1] and index < parsed.rows.len) : (index += 1) {
            const ref = parsed.rows[index];
            if (ref.kind != .line) continue;
            const line = parsed.files[ref.fileIndex].hunks[ref.hunkIndex].lines[ref.lineIndex];
            if (line.kind == .context) continue;
            const key = (@as(u64, ref.fileIndex) << 32) | ref.hunkIndex;
            const entry = try groups.getOrPut(allocator, key);
            if (!entry.found_existing) entry.value_ptr.* = .{ .fileIndex = ref.fileIndex, .hunkIndex = ref.hunkIndex, .lines = .empty };
            try entry.value_ptr.lines.put(allocator, ref.lineIndex, {});
        }
    }
    const out = try allocator.alloc(DiffLineGroup, groups.count());
    for (groups.values(), 0..) |group, offset| {
        out[offset] = .{ .fileIndex = group.fileIndex, .hunkIndex = group.hunkIndex, .lines = group.lines.keys() };
    }
    return out;
}

/// The diff as text for a prompt: one header per file, hunk headers, and marked lines.
pub fn renderDiff(allocator: Allocator, id: u32) ![]const u8 {
    const parsed = try lookup(id);
    var out = std.Io.Writer.Allocating.init(allocator);
    for (parsed.files, 0..) |file, fileIndex| {
        if (fileIndex > 0) try out.writer.writeByte('\n');
        try out.writer.print("diff --git a/{s} b/{s}", .{ file.oldPath orelse file.path, file.newPath orelse file.path });
        if (file.kind == .added) {
            try out.writer.print("\nnew file mode {s}\n--- /dev/null\n+++ b/{s}", .{ file.newMode orelse "100644", file.path });
        }
        for (file.hunks) |hunk| {
            try out.writer.print("\n@@ -{d},{d} +{d},{d} @@ {s}", .{ hunk.oldStart, hunk.oldLines, hunk.newStart, hunk.newLines, hunk.heading });
            for (hunk.lines) |line| {
                const marker: u8 = switch (line.kind) {
                    .added => '+',
                    .removed => '-',
                    .context => ' ',
                };
                try out.writer.writeByte('\n');
                try out.writer.writeByte(marker);
                try out.writer.writeAll(line.text);
            }
        }
    }
    return out.written();
}

// --- helpers -------------------------------------------------------------------------------------

/// The lines of `text` the way `text.split("\n")` with a dropped empty tail sees them.
fn splitLines(allocator: Allocator, text: []const u8) ![][]const u8 {
    var lines: std.ArrayList([]const u8) = .empty;
    if (text.len == 0) return lines.items;
    const body = if (text[text.len - 1] == '\n') text[0 .. text.len - 1] else text;
    var iterator = std.mem.splitScalar(u8, body, '\n');
    while (iterator.next()) |line| try lines.append(allocator, line);
    return lines.items;
}

/// Make `bytes` valid UTF-8 the way `TextDecoder` does, replacing invalid sequences with U+FFFD.
/// Git output is passed to the module as raw bytes; a valid input is returned as is.
fn sanitizeUtf8(allocator: Allocator, bytes: []const u8) ![]const u8 {
    if (std.unicode.utf8ValidateSlice(bytes)) return bytes;
    var out = try std.ArrayList(u8).initCapacity(allocator, bytes.len + 16);
    var index: usize = 0;
    while (index < bytes.len) {
        const length = std.unicode.utf8ByteSequenceLength(bytes[index]) catch {
            try out.appendSlice(allocator, "\u{FFFD}");
            index += 1;
            continue;
        };
        if (index + length <= bytes.len and std.unicode.utf8ValidateSlice(bytes[index .. index + length])) {
            try out.appendSlice(allocator, bytes[index .. index + length]);
            index += length;
        } else {
            try out.appendSlice(allocator, "\u{FFFD}");
            index += 1;
        }
    }
    return out.items;
}

/// The first at most `limit` bytes of valid UTF-8 `text`, never splitting a code point.
fn clipUtf8(text: []const u8, limit: usize) []const u8 {
    if (text.len <= limit) return text;
    var end = limit;
    while (end > 0 and (text[end] & 0b1100_0000) == 0b1000_0000) end -= 1;
    return text[0..end];
}

fn cutPrefix(text: []const u8, prefix: []const u8) ?[]const u8 {
    if (!std.mem.startsWith(u8, text, prefix)) return null;
    return text[prefix.len..];
}

fn trim(text: []const u8) []const u8 {
    return std.mem.trim(u8, text, &std.ascii.whitespace);
}

/// Consume the leading decimal digits of `rest`.
fn takeInteger(rest: *[]const u8) ?u64 {
    var end: usize = 0;
    while (end < rest.len and std.ascii.isDigit(rest.*[end])) end += 1;
    if (end == 0) return null;
    const value = std.fmt.parseInt(u64, rest.*[0..end], 10) catch return null;
    rest.* = rest.*[end..];
    return value;
}

/// `Number.parseInt`: skip whitespace, read digits, ignore the rest (`90%` is 90).
fn leadingInteger(comptime T: type, text: []const u8) ?T {
    var rest = std.mem.trimStart(u8, text, &std.ascii.whitespace);
    const value = takeInteger(&rest) orelse return null;
    return std.math.cast(T, value);
}
