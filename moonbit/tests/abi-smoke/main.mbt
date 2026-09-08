///|
fn main {
  match @native.call("protocol-version", {}) {
    Ok(Number(version, ..)) =>
      if version != @protocol.VERSION.to_double() {
        abort("Wrong native protocol")
      }
    _ => abort("Native C ABI call failed")
  }
  match @native.call("quickgui-unknown-test-operation", {}) {
    Err(_) => ()
    Ok(_) => abort("An unknown native call must return an error")
  }
  println("MoonBit C ABI smoke passed")
}
