///|
test "echo preserves JSON bytes" {
  let payload = b"{ \"keep\": null, \"integer\": 900719925474099312345 }"
  assert_eq(handle(1, b"echo", payload), Ok(payload))
}

///|
test "unknown operations return an error" {
  assert_eq(handle(2, b"missing", b"null"), Err("Unknown method"))
}
