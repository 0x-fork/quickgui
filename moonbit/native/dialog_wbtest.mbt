///|
test "repository picker fills required native fields without changing its input" {
  let options : Json = {
    "title": "Open Repository",
    "files": false,
    "directories": true,
  }
  let prepared = dialog_options(1, options)
  assert_eq(field(prepared, "files"), Some(false.to_json()))
  assert_eq(field(prepared, "directories"), Some(true.to_json()))
  assert_eq(field(prepared, "multiple"), Some(false.to_json()))
  assert_eq(field(prepared, "filters"), Some(Json::array([])))
  assert_eq(field(prepared, "showsHiddenFiles"), Some(false.to_json()))
  assert_eq(field(options, "filters"), None)
}

///|
test "save dialog defaults preserve explicit filters and hidden-file settings" {
  let filters : Json = [{ "name": "Patch", "extensions": ["patch", "diff"] }]
  let prepared = dialog_options(2, {
    "filters": filters,
    "showsHiddenFiles": true,
  })
  assert_eq(field(prepared, "directory"), Some("".to_json()))
  assert_eq(field(prepared, "filters"), Some(filters))
  assert_eq(field(prepared, "showsHiddenFiles"), Some(true.to_json()))
}
