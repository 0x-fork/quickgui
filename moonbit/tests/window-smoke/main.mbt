///|
fn main {
  let closed = @ref.Ref(false)
  let cleaned = @ref.Ref(false)
  let updated = @ref.Ref(false)
  match
    @native.run(
      () => {
        let count = @reactive.signal(0)
        let window = @ui.window(
          () => {
            @reactive.on_cleanup(() => cleaned.val = true)
            @ui.view([
              @ui.view([]).bind_content(() => "Native value: \{count.get()}"),
              @ui.button("Test")
              .bg(@ui.rgb8(45, 105, 180))
              .hover(s => s.opacity(0.5)),
            ])
            .size_full()
            .flex_col()
            .child_if(() => count.get() > 0, () => @ui.text("Mounted"))
            .child(@ui.checkbox([]).bind_checked(() => count.get() > 0))
            .child(@ui.switch([]).checked(true).width(40).height(24))
            .child(
              @ui.select([])
              .items([{ "value": "moonbit", "label": "MoonBit" }])
              .active_value("moonbit"),
            )
            .child(
              {
                let tabs = @ui.tabs([]).active_value("one")
                tabs.children([
                  tabs
                  .slot(@ui.List)
                  .child(tabs.slot(@ui.Tab, value="one").label("One")),
                  tabs.slot(@ui.Panel, value="one").child(@ui.text("Tab panel")),
                ])
              },
            )
          },
          options={
            "title": "QuickGUI MoonBit smoke",
            "show": false,
            "width": 320,
            "height": 200,
          },
        )
        @native.on_event(
          "close",
          _ => {
            if @native.current_window().id != window.id {
              abort("Lost callback window context")
            }
            closed.val = true
            @native.quit()
          },
          window=window.id,
        )
        @native.on_event(
          "window-ready-to-show",
          _ => {
            @native.command(
              {
                "method": "window-action",
                "window": window.id,
                "action": "set-title",
                "value": "Native ready",
              },
              result => {
                match result {
                  Err(error) => abort(error)
                  Ok(_) => {
                    count.set(1)
                    @native.flush()
                    @native.command(
                      {
                        "method": "window-action",
                        "window": window.id,
                        "action": "set-title",
                        "value": "Mutation applied",
                      },
                      result => {
                        match result {
                          Err(error) => abort(error)
                          Ok(_) => ()
                        }
                        updated.val = true
                        window.close()
                      },
                    )
                  }
                }
              },
            )
          },
          window=window.id,
        )
      },
      options={
        "name": "MoonBit smoke",
        "version": "0.1.0",
        "identifier": "dev.quickgui.moonbit-smoke",
        "quitMode": "explicit",
      },
    ) {
    Err(error) => abort(error)
    Ok(_) => ()
  }
  if !closed.val || !cleaned.val || !updated.val {
    abort("Window lifecycle did not complete")
  }
  println("MoonBit native window lifecycle passed")
}
