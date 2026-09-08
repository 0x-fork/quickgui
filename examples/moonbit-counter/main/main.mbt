///|
fn open_details_window() -> Unit {
  ignore(
    @ui.window(
      () => {
        let window = @native.current_window()
        @ui.div([
          @ui.text("Created while the app is running")
          .font_size(22)
          .font_weight(700),
          @ui.text(
            "This window has its own retained tree and native lifecycle.",
          )
          .text_color(@ui.rgb8(148, 163, 184))
          .line_height(21),
          @ui.button("Close window")
          .on_click(() => window.close())
          .height(40)
          .flex_row()
          .items_center()
          .justify_center()
          .bg(@ui.rgb8(51, 65, 85))
          .border_radius(9)
          .cursor("default"),
        ])
        .size_full()
        .flex_col()
        .justify_center()
        .gap(16)
        .padding(28)
        .bg(@ui.rgb8(17, 24, 39))
        .text_color(@ui.rgb8(226, 232, 240))
      },
      options={
        "title": "Dynamic QuickGUI window",
        "width": 420,
        "height": 260,
        "minimumWidth": 320,
        "minimumHeight": 200,
        "background": @ui.rgb8(17, 24, 39),
      },
    ),
  )
}

///|
fn counter() -> @ui.Element {
  let (count, set_count) = @ui.create_signal(0)
  @ui.div([
    @ui.div([@ui.text("QuickGUI · MoonBit").font_weight(600)])
    .flex_row()
    .height(52)
    .flex_shrink(0)
    .items_center()
    .justify_center()
    .app_region("drag")
    .border_color(@ui.rgb8(30, 41, 59))
    .border_width(1),
    @ui.div([
      @ui.div(
        [
          @ui.text("Fine-grained native UI")
          .font_size(28)
          .line_height(36)
          .font_weight(700),
          @ui.text(
            "Signals update only the changed text node. The application is ordinary MoonBit, " +
            "and QuickGUI retains layout, sleeps while clean, and redraws once per mutation batch.",
          )
          .text_color(@ui.rgb8(148, 163, 184))
          .font_size(14)
          .line_height(21),
          @ui.text("Count: \{count()}")
          .font_size(20)
          .font_weight(600)
          .text_color(
            if count() >= 5 {
              @ui.rgb8(251, 191, 36)
            } else {
              @ui.rgb8(191, 219, 254)
            },
          ),
          ..if count() >= 5 {
            [
              @ui.text(
                "Five or more clicks: the row above was created on demand.",
              )
              .text_color(@ui.rgb8(251, 191, 36))
              .font_size(14),
            ]
          } else {
            []
          },
          @ui.button("Increment")
          .on_click(() => set_count(count() + 1))
          .height(44)
          .flex_row()
          .items_center()
          .justify_center()
          .bg(@ui.rgb8(37, 99, 235))
          .text_color(@ui.rgb8(255, 255, 255))
          .border_radius(9)
          .cursor("default")
          .app_region("no-drag")
          .user_select("none")
          .hover(s => s.bg(@ui.rgb8(59, 130, 246))),
          @ui.button("Open window")
          .on_click(open_details_window)
          .height(44)
          .flex_row()
          .items_center()
          .justify_center()
          .bg(@ui.rgb8(51, 65, 85))
          .text_color(@ui.rgb8(255, 255, 255))
          .border_radius(9)
          .cursor("default")
          .app_region("no-drag")
          .user_select("none"),
        ],
      )
      .flex_col()
      .width(420)
      .gap(18)
      .padding(28)
      .bg(@ui.rgb8(17, 24, 39))
      .border_color(@ui.rgb8(51, 65, 85))
      .border_width(1)
      .border_radius(16),
    ])
    .flex_1()
    .min_height(0)
    .flex_row()
    .items_center()
    .justify_center()
    .padding(32),
  ])
  .size_full()
  .flex_col()
  .bg(@ui.rgb8(9, 13, 22))
  .text_color(@ui.rgb8(226, 232, 240))
}

///|
fn open_main_window() -> Unit {
  ignore(
    @ui.window(counter, options={
      "title": "QuickGUI Counter",
      "width": 760,
      "height": 520,
      "minimumWidth": 520,
      "minimumHeight": 360,
      "background": @ui.rgb8(9, 13, 22),
      "titleBarStyle": "hiddenInset",
      "trafficLightX": 16,
      "trafficLightY": 13,
    }),
  )
}

///|
fn main {
  match @native.run(open_main_window) {
    Ok(_) => ()
    Err(error) => abort(error)
  }
}
