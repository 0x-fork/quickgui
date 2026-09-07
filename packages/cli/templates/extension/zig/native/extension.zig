const std = @import("std");
const c = @cImport({
    @cInclude("quickgui_extension.h");
});
const build_info = @import("build_info.zig");

fn bytes(value: []const u8) c.QuickGuiBytes {
    return .{ .data = value.ptr, .len = value.len };
}

fn invoke(_: u32, method: c.QuickGuiBytes, params: c.QuickGuiBytes, sink: c.QuickGuiServiceSink) callconv(.c) void {
    // This bounded CPU-only example replies immediately without allocating. Copy
    // borrowed input and retain the sink when queuing asynchronous work instead.
    defer sink.release.?(sink.context);
    if (method.len == 4 and method.data != null and
        std.mem.eql(u8, method.data[0..method.len], "echo") and
        params.len <= c.QUICKGUI_EXTENSION_MAX_PAYLOAD)
    {
        sink.emit.?(sink.context, c.QUICKGUI_EXTENSION_REPLY, params);
    } else {
        sink.emit.?(sink.context, c.QUICKGUI_EXTENSION_ERROR, bytes("Unknown method or oversized payload"));
    }
}

fn shutdown() callconv(.c) void {
    // No sessions or workers in this example. Cancel yours here without waiting.
}

const api: c.QuickGuiServiceApi = .{ .invoke = invoke, .shutdown = shutdown };
const descriptor: c.QuickGuiExtension = .{
    .abi_version = c.QUICKGUI_EXTENSION_ABI_V1,
    .descriptor_size = @sizeOf(c.QuickGuiExtension),
    .kind = c.QUICKGUI_EXTENSION_SERVICE,
    .api_size = @sizeOf(c.QuickGuiServiceApi),
    .name = bytes(build_info.name),
    .version = bytes(build_info.version),
    .api = &api,
};

export fn quickgui_extension_v1() callconv(.c) *const c.QuickGuiExtension {
    return &descriptor;
}
