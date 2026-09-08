///|
/// Register before native.run. Package the separate terminal extension through
/// quickgui.toml; importing this package does not add a terminal to the core.
pub fn register(version? : String = "0.1.3") -> Unit {
  @native.require_extension("terminal", version)
}

///|
pub fn view(
  program : String,
  args? : Array[String] = [],
  directory? : String = "",
  environment? : Map[String, String] = Map([]),
) -> @ui.Element {
  @ui.element(@protocol.TAG_TERMINAL)
  .property(@protocol.TERMINAL_PROGRAM, @protocol.Text(program))
  .property(
    @protocol.TERMINAL_ARGUMENTS,
    @protocol.Text(args.to_json().stringify()),
  )
  .property(@protocol.TERMINAL_WORKING_DIRECTORY, @protocol.Text(directory))
  .property(
    @protocol.TERMINAL_ENVIRONMENT,
    @protocol.Text(environment.to_json().stringify()),
  )
}
