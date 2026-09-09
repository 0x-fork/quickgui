///|
pub fn label(value : String, quantity~ : Int) -> @ui.Element {
  @ui.view([@ui.text(value), @ui.text("\{quantity}")])
}
