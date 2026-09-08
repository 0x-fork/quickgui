///|
/// Render every demo through the real native host in both appearances. Waiting
/// for FirstPresented catches layout/identity errors that test_tree cannot.
pub fn smoke() -> Unit {
  let presented = @ref.Ref(0)
  let cleaned = @ref.Ref(0)
  let total = demos.length() * 2
  fn next() -> Unit {
    if presented.val == total {
      @native.quit()
      return
    }
    let index = presented.val
    let demo = demos[index % demos.length()]
    let appearance = if index < demos.length() { "light" } else { "dark" }
    let saw_toasts = @ref.Ref(false)
    let window = @ui.window(
      () => {
        @ui.on_cleanup(() => cleaned.val += 1)
        let view = gallery(
          { appearance: @reactive.signal(appearance), },
          @reactive.signal(demo.id),
          @reactive.signal("1080×780"),
        )
        if demo.id == "toast" {
          // Exercise dynamic toast parts too: an empty viewport does not expose
          // duplicate close listeners or retained toast identity mistakes.
          for kind in ["info", "success", "error"] {
            smoke_button(view.native_node(), kind).emit("click", {})
          }
        }
        view
      },
      options=window_options(show=false),
    )
    @native.on_event(
      "window-ready-to-show",
      _ => {
        println("Presented \{demo.id} (\{appearance})")
        presented.val += 1
        if demo.id != "toast" {
          window.close()
        }
      },
      window=window.id,
    )
    if demo.id == "toast" {
      @native.on_event(
        "componentchange",
        event => {
          if get(event.json(), "toasts") is Array(entries) {
            if !entries.is_empty() {
              saw_toasts.val = true
            } else if saw_toasts.val {
              println("Toast deadline dismissed all entries (\{appearance})")
              window.close()
            }
          }
        },
        window=window.id,
      )
    }
    @native.on_event("close", _ => next(), window=window.id)
  }
  match
    @native.run(next, options={
      "name": "MoonBit gallery rendering check",
      "version": "0.1.0",
      "identifier": "dev.quickgui.components-moonbit.smoke",
      "quitMode": "explicit",
    }) {
    Err(error) => abort(error)
    Ok(_) => ()
  }
  if presented.val != total || cleaned.val != total {
    abort("Gallery rendering or owner cleanup did not complete")
  }
  println("Presented and disposed all \{total} gallery windows")
}

///|
fn smoke_button(node : @native.Node, label : String) -> @native.Node {
  fn find(node : @native.Node) -> @native.Node? {
    if node.property(@protocol.CLICK_LISTENER) == @protocol.Boolean(true) &&
      node.children().iter().any(child => child.text_content() == label) {
      return Some(node)
    }
    for child in node.children() {
      if find(child) is Some(button) {
        return Some(button)
      }
    }
    None
  }
  find(node).unwrap()
}
