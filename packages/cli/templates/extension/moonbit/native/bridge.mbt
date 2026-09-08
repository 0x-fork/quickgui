///|
// Borrowed for qge_invoke only; never retain this pointer in MoonBit state.
#external
pub type Request

///|
extern "C" fn request_id(context : Request) -> UInt = "qge_request_id"

///|
extern "C" fn request_bytes(context : Request, field : Int) -> Bytes = "qge_request_bytes"

///|
#borrow(payload)
extern "C" fn reply(context : Request, kind : Int, payload : Bytes) -> Unit = "qge_reply"

///|
// scripts/provider.ts exports these two entry points to the private C adapter.
// Only an opaque pointer crosses this boundary; MoonBit owns all managed values.
pub fn qge_invoke(context : Request) -> Unit {
  let result = handle(
    request_id(context),
    request_bytes(context, 0),
    request_bytes(context, 1),
  )
  match result {
    Ok(bytes) => reply(context, 0, bytes)
    Err(message) => reply(context, 1, @utf8.encode(message))
  }
}

///|
pub fn qge_shutdown() -> Unit {
  shutdown()
}
