///|
// Implement your service here. Inputs and successful replies are raw UTF-8 JSON;
// returning the bytes unchanged preserves nulls and large numbers exactly.
fn handle(
  _request : UInt,
  operation : Bytes,
  params : Bytes,
) -> Result[Bytes, String] {
  if operation == b"echo" {
    Ok(params)
  } else {
    Err("Unknown method")
  }
}

///|
// Release service-owned state here. Do not wait for workers or call the UI.
fn shutdown() -> Unit {
  ()
}
