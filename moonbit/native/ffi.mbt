///|
extern "C" fn ffi_load(protocol : Int) -> Int = "qgm_load"

///|
extern "C" fn ffi_error() -> Bytes = "qgm_error"

///|
extern "C" fn ffi_run(entry : FuncRef[() -> Unit]) -> Int = "qgm_run"

///|
extern "C" fn ffi_next_event(blocking : Bool) -> Int = "qgm_next_event"

///|
extern "C" fn ffi_event_number(field : Int) -> Int = "qgm_event_number"

///|
extern "C" fn ffi_event_bytes(field : Int) -> Bytes = "qgm_event_bytes"

///|
#borrow(json)
extern "C" fn ffi_create_app(json : Bytes) -> Int = "qgm_create_app"

///|
extern "C" fn ffi_prepare_app(app : Int) -> Int = "qgm_prepare_app"

///|
extern "C" fn ffi_allocate_window() -> Int = "qgm_allocate_window"

///|
#borrow(options, batch)
extern "C" fn ffi_create_window(
  app : Int,
  window : Int,
  options : Bytes,
  batch : Bytes,
) -> Int = "qgm_create_window"

///|
#borrow(options, batch)
extern "C" fn ffi_create_child(
  app : Int,
  window : Int,
  parent : Int,
  anchor : Int,
  mode : Int,
  options : Bytes,
  batch : Bytes,
) -> Int = "qgm_create_child"

///|
#borrow(batch)
extern "C" fn ffi_apply_batch(app : Int, window : Int, batch : Bytes) -> Int = "qgm_apply_batch"

///|
extern "C" fn ffi_close_window(app : Int, window : Int) -> Int = "qgm_close_window"

///|
extern "C" fn ffi_focus_node(app : Int, window : Int, node : Int) -> Int = "qgm_focus_node"

///|
#borrow(json)
extern "C" fn ffi_command(app : Int, request : Int, json : Bytes) -> Int = "qgm_command"

///|
#borrow(operation, params)
extern "C" fn ffi_invoke(
  request : Int,
  operation : Bytes,
  params : Bytes,
) -> Int = "qgm_invoke"

///|
#borrow(json)
extern "C" fn ffi_dialog(
  app : Int,
  window : Int,
  request : Int,
  kind : Int,
  json : Bytes,
) -> Int = "qgm_dialog"

///|
#borrow(operation, params)
extern "C" fn ffi_call(operation : Bytes, params : Bytes) -> Bytes = "qgm_call"

///|
#borrow(message)
extern "C" fn ffi_abort(message : Bytes) -> Unit = "qgm_abort"

///|
#borrow(name, version)
extern "C" fn ffi_load_extension(name : Bytes, version : Bytes) -> Int = "qgm_load_extension"

///|
extern "C" fn ffi_metadata() -> Bytes = "qgm_metadata"

///|
extern "C" fn ffi_resource_directory() -> Bytes = "qgm_resource_directory"
