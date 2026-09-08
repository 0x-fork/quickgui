///|
pub fn label(value : String, quantity~ : Int) -> @ui.Element {
  @ui.div([@ui.text(value), @ui.text("\{quantity}")])
}
