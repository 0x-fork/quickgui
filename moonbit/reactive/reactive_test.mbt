///|
test "signals update their subscribers only, batch, and dispose" {
  let baseline = @reactive.active_effect_count()
  let root = @reactive.create_scope()
  let a = @reactive.signal(0)
  let b = @reactive.signal(0)
  let runs = @ref.Ref(0)
  @reactive.with_owner(root, () => {
    @reactive.create_effect(() => {
      ignore(a.get())
      runs.val += 1
    })
  })
  b.set(1)
  a.set(0)
  assert_eq(runs.val, 1)
  @reactive.batch(() => {
    a.set(1)
    a.set(2)
  })
  assert_eq(runs.val, 2)
  @reactive.dispose(root)
  a.set(3)
  assert_eq(runs.val, 2)
  assert_eq(@reactive.active_effect_count(), baseline)
}

///|
test "conditional dependencies unsubscribe and memo diamonds settle before effects" {
  let root = @reactive.create_scope()
  let a = @reactive.signal(1)
  let b = @reactive.signal(2)
  let choose = @reactive.signal(true)
  let seen : Array[Int] = []
  @reactive.with_owner(root, () => {
    let value = @reactive.memo(() => {
      if choose.get() {
        a.get()
      } else {
        b.get()
      }
    })
    let doubled = @reactive.memo(() => value.get() * 2)
    @reactive.create_effect(() => seen.push(value.get() + doubled.get()))
  })
  choose.set(false)
  a.set(9)
  b.set(4)
  assert_eq(seen, [3, 6, 12])
  @reactive.dispose(root)
}

///|
test "rerunning effects retire child scopes and cleanups" {
  let baseline = @reactive.active_scope_count()
  let root = @reactive.create_scope()
  let value = @reactive.signal(0)
  let cleaned = @ref.Ref(0)
  @reactive.with_owner(root, () => {
    @reactive.create_effect(() => {
      ignore(value.get())
      @reactive.on_cleanup(() => cleaned.val += 1)
      @reactive.create_effect(() => ())
    })
  })
  value.set(1)
  assert_eq(cleaned.val, 1)
  @reactive.dispose(root)
  assert_eq(cleaned.val, 2)
  assert_eq(@reactive.active_scope_count(), baseline)
}

///|
test "initial dependent writes settle immediately and respect explicit batches" {
  let root = @reactive.create_scope()
  let value = @reactive.signal(0)
  let seen : Array[Int] = []
  @reactive.with_owner(root, () => {
    @reactive.create_effect(() => seen.push(value.get()))
    @reactive.create_effect(() => value.set(7))
  })
  assert_eq(seen, [0, 7])
  @reactive.batch(fn() raise {
    @reactive.with_owner(root, () => @reactive.create_effect(() => value.set(8)))
    assert_eq(seen, [0, 7])
  })
  assert_eq(seen, [0, 7, 8])
  @reactive.dispose(root)
}
