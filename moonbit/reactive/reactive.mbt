// Only the application UI thread accesses these objects. IDs rather than object
// back-references keep subscriptions and scopes free of reference-count cycles.

///|
priv struct Scope {
  parent : Int
  children : Map[Int, Unit]
  effects : Map[Int, Unit]
  cleanups : Map[Int, () -> Unit]
}

///|
priv struct Effect {
  run : () -> Unit
  scope : Int
  subscriptions : Array[() -> Unit]
  memo : Bool
  mut queued : Bool
}

///|
let scopes : Map[Int, Scope] = Map([])

///|
let effects : Map[Int, Effect] = Map([])

///|
let next_id : @ref.Ref[Int] = @ref.Ref(0)

///|
let current_scope : @ref.Ref[Int] = @ref.Ref(0)

///|
let current_effect : @ref.Ref[Int] = @ref.Ref(0)

///|
let batch_depth : @ref.Ref[Int] = @ref.Ref(0)

///|
let flushing : @ref.Ref[Bool] = @ref.Ref(false)

///|
let work : Array[Int] = []

///|
let memo_work : Array[Int] = []

///|
fn allocate_id() -> Int {
  if next_id.val == 0x7fffffff {
    abort("QuickGUI reactive ID space exhausted")
  }
  next_id.val += 1
  next_id.val
}

///|
pub fn owner() -> Int {
  current_scope.val
}

///|
pub fn parent_owner(scope : Int) -> Int {
  match scopes.get(scope) {
    Some(scope) => scope.parent
    None => 0
  }
}

///|
pub fn owner_alive(scope : Int) -> Bool {
  scopes.contains(scope)
}

///|
pub fn create_scope(parent? : Int = current_scope.val) -> Int {
  let id = allocate_id()
  let scope = {
    parent,
    children: Map([]),
    effects: Map([]),
    cleanups: Map([]),
  }
  scopes[id] = scope
  if parent != 0 {
    match scopes.get(parent) {
      Some(owner) => owner.children[id] = ()
      None => abort("Cannot create a scope under a disposed owner")
    }
  }
  id
}

///|
pub fn[T] with_owner(scope : Int, run : () -> T raise?) -> T raise? {
  if scope != 0 && !scopes.contains(scope) {
    abort("QuickGUI reactive owner is disposed")
  }
  current_scope.protect(scope, run)
}

///|
pub fn[T] untrack(run : () -> T raise?) -> T raise? {
  current_effect.protect(0, run)
}

///|
pub fn on_cleanup(cleanup : () -> Unit) -> Unit {
  ignore(on_cleanup_cancelable(cleanup))
}

///|
pub fn on_cleanup_cancelable(cleanup : () -> Unit) -> () -> Unit {
  let owner = current_scope.val
  let id = allocate_id()
  match scopes.get(current_scope.val) {
    Some(scope) => scope.cleanups[id] = cleanup
    None => abort("on_cleanup requires a reactive owner")
  }
  () => if scopes.get(owner) is Some(scope) { scope.cleanups.remove(id) }
}

///|
fn reset_scope(scope : Scope) -> Unit {
  for id in scope.children.keys().to_array() {
    dispose(id)
  }
  for id in scope.effects.keys().to_array() {
    dispose_effect(id)
  }
  scope.children.clear()
  scope.effects.clear()
  let previous = scope.cleanups.copy()
  scope.cleanups.clear()
  let order = previous.keys().to_array()
  order.sort()
  for i = order.length() - 1; i >= 0; i = i - 1 {
    previous.get(order[i]).unwrap()()
  }
}

///|
pub fn dispose(id : Int) -> Unit {
  match scopes.get(id) {
    None => ()
    Some(scope) => {
      // Remove first so cleanup cannot create new work in a retired owner.
      scopes.remove(id)
      untrack(() => reset_scope(scope))
      if scopes.get(scope.parent) is Some(parent) {
        parent.children.remove(id)
      }
    }
  }
}

///|
fn dispose_effect(id : Int) -> Unit {
  if effects.get(id) is Some(effect) {
    effects.remove(id)
    for cleanup in effect.subscriptions {
      cleanup()
    }
    effect.subscriptions.clear()
    dispose(effect.scope)
  }
}

///|
fn run_effect(id : Int) -> Unit {
  guard effects.get(id) is Some(effect) else { return }
  guard scopes.get(effect.scope) is Some(scope) else { return }
  effect.queued = false
  for cleanup in effect.subscriptions {
    cleanup()
  }
  effect.subscriptions.clear()
  untrack(() => reset_scope(scope))
  current_effect.protect(id, () => with_owner(effect.scope, effect.run))
}

///|
fn new_effect(run : () -> Unit, memo : Bool) -> Int {
  if current_scope.val == 0 {
    abort("Effects require a scope; create a root scope first")
  }
  let parent = current_scope.val
  let id = allocate_id()
  let scope = create_scope()
  let effect = { run, scope, memo, subscriptions: [], queued: false, }
  effects[id] = effect
  scopes.get(parent).unwrap().effects[id] = ()
  run_effect(id)
  // Initial effects can write derived/component state. Settle those subscribers
  // before returning to the caller, while preserving explicit and nested batches.
  if current_effect.val == 0 && (!work.is_empty() || !memo_work.is_empty()) {
    flush()
  }
  id
}

///|
pub fn create_effect(run : () -> Unit) -> Unit {
  ignore(new_effect(run, false))
}

///|
fn schedule(id : Int) -> Unit {
  if effects.get(id) is Some(effect) {
    if !effect.queued {
      effect.queued = true
      if effect.memo {
        memo_work.push(id)
      } else {
        work.push(id)
      }
    }
  }
}

///|
pub fn flush() -> Unit {
  if flushing.val || batch_depth.val > 0 {
    return
  }
  flushing.val = true
  defer {
    flushing.val = false
  }
  let mut cursor = 0
  let mut memo_cursor = 0
  let mut count = 0
  while memo_cursor < memo_work.length() || cursor < work.length() {
    count += 1
    if count > 10000 {
      abort("QuickGUI reactive update cycle exceeded 10000 effects")
    }
    if memo_cursor < memo_work.length() {
      let id = memo_work[memo_cursor]
      memo_cursor += 1
      run_effect(id)
    } else {
      let id = work[cursor]
      cursor += 1
      run_effect(id)
    }
  }
  work.clear()
  memo_work.clear()
}

///|
pub fn[T] batch(run : () -> T raise?) -> T raise? {
  batch_depth.val += 1
  defer {
    batch_depth.val -= 1
    if batch_depth.val == 0 {
      flush()
    }
  }
  run()
}

///|
pub struct Signal[T] {
  mut value : T
  subscribers : Map[Int, Unit]
}

///|
pub fn[T] signal(value : T) -> Signal[T] {
  { value, subscribers: Map([]), }
}

///|
/// The getter tracks reads in a view binding, memo, or effect. The setter skips
/// equal values and participates in the same batching and ownership as Signal.
pub fn[T : Eq] create_signal(value : T) -> (() -> T, (T) -> Unit) {
  let state = signal(value)
  (() => state.get(), next => state.set(next))
}

///|
pub fn[T] Signal::get(self : Signal[T]) -> T {
  let id = current_effect.val
  if id != 0 && !self.subscribers.contains(id) {
    if effects.get(id) is Some(effect) {
      self.subscribers[id] = ()
      effect.subscriptions.push(() => self.subscribers.remove(id))
    }
  }
  self.value
}

///|
pub fn[T] Signal::peek(self : Signal[T]) -> T {
  self.value
}

///|
pub fn[T : Eq] Signal::set(self : Signal[T], value : T) -> Unit {
  if self.value == value {
    return
  }
  self.value = value
  for id in self.subscribers.keys().to_array() {
    schedule(id)
  }
  if current_effect.val == 0 {
    flush()
  }
}

///|
pub fn[T : Eq] Signal::update(self : Signal[T], update : (T) -> T) -> Unit {
  self.set(update(self.value))
}

///|
/// A cached, read-only getter. Reuses the same equality, batching, and ownership
/// as signals without exposing a setter for derived state.
pub fn[T : Eq] create_memo(read : () -> T) -> () -> T {
  let value = signal(untrack(read))
  ignore(new_effect(() => value.set(read()), true))
  () => value.get()
}

// Useful for ownership regressions without requiring a window or native library.

///|
pub fn active_effect_count() -> Int {
  effects.length()
}

///|
pub fn active_scope_count() -> Int {
  scopes.length()
}

///|
pub fn active_cleanup_count() -> Int {
  let mut count = 0
  for _, scope in scopes {
    count += scope.cleanups.length()
  }
  count
}
