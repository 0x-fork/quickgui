///|
fn layout_fraction(fraction : Float) -> String {
  let fraction = if !fraction.is_nan() && !fraction.is_inf() && fraction >= 0 {
    fraction
  } else {
    0
  }
  "\{fraction * 100}%"
}

///|
fn layout_grid_tracks(
  count : Int,
  minimum : String,
  maximum : String,
) -> String {
  if count <= 0 {
    return "none"
  }
  let count = if count > 1024 { 1024 } else { count }
  "repeat(\{count}, minmax(\{minimum}, \{maximum}))"
}
