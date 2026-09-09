///|
fn counter() -> @ui.Element {
  let (count, set_count) = @ui.create_signal(0)
  @ui.view(
    [
      @ui.text("QuickGUI · MoonBit").font_size(28).font_weight(700),
      @ui.text("Count: \{count()}").font_size(22),
      @ui.button("Increment")
      .on_click(() => set_count(count() + 1))
      .px(20)
      .py(10)
      .border_radius(8)
      .bg(@ui.rgb8(45, 105, 180))
      .opacity(0.9)
      .hover(s => s.bg(@ui.rgb8(56, 122, 204)).opacity(1.0)),
      ..if count() >= 5 {
        [
          @ui.text("Five clicks! This child was mounted on demand.").text_color(
            @ui.rgb8(251, 191, 36),
          ),
        ]
      } else {
        []
      },
      @ui.button("Open window")
      .on_click(() => {
        ignore(
          @ui.window(
            () => {
              let window = @native.current_window()
              @ui.view([
                @ui.text("An independent MoonBit window"),
                @ui.button("Close window").on_click(() => window.close()),
              ])
              .size_full()
              .flex_col()
              .items_center()
              .justify_center()
              .gap(16)
            },
            options={ "title": "Details", "width": 420, "height": 240 },
          ),
        )
      })
      .px(20)
      .py(10)
      .border_radius(8)
      .bg(@ui.rgb8(51, 65, 85)),
    ],
  )
  .size_full()
  .flex_col()
  .items_center()
  .justify_center()
  .gap(18)
  .bg(@ui.rgb8(18, 18, 20))
  .text_color(@ui.rgb8(240, 241, 244))
}

///|
fn main {
  match
    @native.run(() => {
      ignore(
        @ui.window(counter, options={
            "title": {{APP_NAME}},
          "width": 620,
          "height": 480,
        }),
      )
    }) {
    Ok(_) => ()
    Err(error) => abort(error)
  }
}
