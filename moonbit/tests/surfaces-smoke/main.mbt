///|
fn main {
  let surface_ready = @ref.Ref(false)
  let embedded_ready = @ref.Ref(false)
  let parent_id = @ref.Ref(0)
  let popup_id = @ref.Ref(0)
  let parent_window : @ref.Ref[@native.Window?] = @ref.Ref(None)
  let anchor : @ref.Ref[@native.Node?] = @ref.Ref(None)
  let embedded_cleaned = @ref.Ref(false)
  let result = @native.run(
    () => {
      @native.on_event("window-ready-to-show", event => {
        if event.window == parent_id.val {
          let popup = @native.new_system_popover(
            anchor.val.unwrap(),
            () => @ui.text("Native popover").padding(16).native_node(),
            options={
              "width": 200,
              "height": 100,
              "show": false,
              "popoverGrab": false,
            },
          )
          popup_id.val = popup.id
        } else if event.window == popup_id.val {
          surface_ready.val = true
          @native.Window::{ id: event.window, }.close()
        } else {
          embedded_ready.val = true
        }
        if surface_ready.val && embedded_ready.val {
          parent_window.val.unwrap().close()
        }
      })
      @native.on_event("close", event => {
        if event.window == parent_id.val {
          @native.quit()
        }
      })
      let parent = @ui.window(
        () => {
          let button = @ui.button("Anchor").width(120).height(32)
          anchor.val = Some(button.native_node())
          @ui.view([
            button,
            @ui.swift_ui_host([
              @ui.swift_ui_quickgui_host_view(() => {
                @reactive.on_cleanup(() => embedded_cleaned.val = true)
                @ui.text("Embedded QuickGUI").padding(12)
              })
              .width(240)
              .height(120),
            ])
            .width(240)
            .height(120),
          ])
          .size_full()
          .flex_col()
        },
        options={
          "title": "MoonBit native surfaces smoke",
          "width": 400,
          "height": 240,
          "show": false,
        },
      )
      parent_id.val = parent.id
      parent_window.val = Some(parent)
    },
    options={
      "name": "MoonBit surfaces smoke",
      "version": "0.1.0",
      "identifier": "dev.quickgui.moonbit-surfaces-smoke",
      "quitMode": "explicit",
    },
  )
  if result is Err(error) {
    abort(error)
  }
  if !surface_ready.val || !embedded_ready.val || !embedded_cleaned.val {
    abort("Native child surface lifecycle did not complete")
  }
  println("MoonBit native popover and SwiftUI reverse-host lifecycle passed")
}
