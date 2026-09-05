//! The subset of Node-API a QuickGUI native module needs.
//!
//! Every symbol is resolved by the host process (Bun or Node) when the addon is loaded, so the
//! module is linked with undefined symbols allowed and never sees a Node-API header.

pub const Env = *opaque {};
pub const Value = *opaque {};
pub const CallbackInfo = *opaque {};
pub const Deferred = *opaque {};
pub const AsyncWork = *opaque {};

pub const Status = enum(c_int) {
    ok = 0,
    invalid_arg = 1,
    object_expected = 2,
    string_expected = 3,
    name_expected = 4,
    function_expected = 5,
    number_expected = 6,
    boolean_expected = 7,
    array_expected = 8,
    generic_failure = 9,
    pending_exception = 10,
    cancelled = 11,
    escape_called_twice = 12,
    handle_scope_mismatch = 13,
    callback_scope_mismatch = 14,
    queue_full = 15,
    closing = 16,
    bigint_expected = 17,
    date_expected = 18,
    arraybuffer_expected = 19,
    detachable_arraybuffer_expected = 20,
    would_deadlock = 21,
    _,
};

pub const TypedArrayType = enum(c_int) {
    int8 = 0,
    uint8 = 1,
    uint8_clamped = 2,
    int16 = 3,
    uint16 = 4,
    int32 = 5,
    uint32 = 6,
    float32 = 7,
    float64 = 8,
    bigint64 = 9,
    biguint64 = 10,
    _,
};

pub const Callback = *const fn (Env, CallbackInfo) callconv(.c) ?Value;
pub const ExecuteCallback = *const fn (Env, ?*anyopaque) callconv(.c) void;
pub const CompleteCallback = *const fn (Env, Status, ?*anyopaque) callconv(.c) void;

pub extern fn napi_create_function(env: Env, name: [*]const u8, length: usize, callback: Callback, data: ?*anyopaque, result: *Value) Status;
pub extern fn napi_set_named_property(env: Env, object: Value, name: [*:0]const u8, value: Value) Status;
pub extern fn napi_get_cb_info(env: Env, info: CallbackInfo, argc: *usize, argv: [*]Value, this: ?*Value, data: ?*?*anyopaque) Status;
pub extern fn napi_get_typedarray_info(env: Env, value: Value, kind: *TypedArrayType, length: *usize, data: *?*anyopaque, arraybuffer: ?*Value, byte_offset: ?*usize) Status;
pub extern fn napi_create_buffer_copy(env: Env, length: usize, data: [*]const u8, result_data: ?*?*anyopaque, result: *Value) Status;
pub extern fn napi_create_string_utf8(env: Env, string: [*]const u8, length: usize, result: *Value) Status;
pub extern fn napi_get_value_uint32(env: Env, value: Value, result: *u32) Status;
pub extern fn napi_throw_error(env: Env, code: ?[*:0]const u8, message: [*:0]const u8) Status;
pub extern fn napi_throw_type_error(env: Env, code: ?[*:0]const u8, message: [*:0]const u8) Status;
pub extern fn napi_create_error(env: Env, code: ?Value, message: Value, result: *Value) Status;
pub extern fn napi_create_promise(env: Env, deferred: *Deferred, promise: *Value) Status;
pub extern fn napi_resolve_deferred(env: Env, deferred: Deferred, value: Value) Status;
pub extern fn napi_reject_deferred(env: Env, deferred: Deferred, value: Value) Status;
pub extern fn napi_create_async_work(env: Env, resource: ?Value, name: Value, execute: ExecuteCallback, complete: CompleteCallback, data: ?*anyopaque, result: *AsyncWork) Status;
pub extern fn napi_queue_async_work(env: Env, work: AsyncWork) Status;
pub extern fn napi_delete_async_work(env: Env, work: AsyncWork) Status;
