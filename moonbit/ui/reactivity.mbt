///|
/// Create component state with a getter and setter.
pub fn[T : Eq] create_signal(value : T) -> (() -> T, (T) -> Unit) {
  @reactive.create_signal(value)
}

///|
/// Cache a derived value and return its read-only getter.
pub fn[T : Eq] create_memo(read : () -> T) -> () -> T {
  @reactive.create_memo(read)
}

///|
pub fn create_effect(run : () -> Unit) -> Unit {
  @reactive.create_effect(run)
}

///|
pub fn[T] batch(run : () -> T raise?) -> T raise? {
  @reactive.batch(run)
}

///|
pub fn[T] untrack(read : () -> T raise?) -> T raise? {
  @reactive.untrack(read)
}

///|
pub fn on_cleanup(cleanup : () -> Unit) -> Unit {
  @reactive.on_cleanup(cleanup)
}
