///|
fn main {
  @native.require_extension("acme-echo", "1.0.0")
  let received = @ref.Ref(false)
  match
    @native.run(
      () => {
        @native.invoke(
          "extension/acme-echo/echo",
          "MoonBit → native → MoonBit",
          result => {
            match result {
              Ok(String(value)) => {
                if value != "MoonBit → native → MoonBit" {
                  abort("Incorrect extension response")
                }
                received.val = true
              }
              _ => abort("Native extension invocation failed")
            }
            @native.quit()
          },
        )
      },
      options={ "quitMode": "explicit" },
    ) {
    Err(error) => abort(error)
    Ok(_) => ()
  }
  if !received.val {
    abort("Missing extension callback")
  }
  println("MoonBit independent native extension passed")
}
