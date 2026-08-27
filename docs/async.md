# Asynchronous work

[Documentation index](README.md)

Non-blocking application futures use a window-owned foreground executor. The future starts on the
next event-loop turn; dropping its `Task` handle cancels it, while `detach()` keeps it alive until
completion or window close. View access after an `await` is explicitly fallible and deferred until
the current future poll has released executor state:

```rust
let task = cx.spawn(|task_cx: AsyncViewContext<MyView>| async move {
    task_cx.sleep(Duration::from_millis(500)).await?;
    task_cx.update(|view, cx| {
        view.ready = true;
        cx.invalidate();
    }).await
})?;
```

`sleep` contributes one exact `ControlFlow::WaitUntil` deadline; it does not create a timer thread,
polling frame, or idle wakeup. The runtime caps live tasks at 1,024 per window and 4,096 per
application, polls at most 256 ready futures per event-loop turn, and independently bounds queued
updates and timers. See `cargo run --release --example foreground_tasks`.

Blocking or CPU-heavy application work uses the separate bounded background pool. Completion
returns to the owning window once through the event loop, so a clean UI does not poll:

```rust
cx.spawn_background(load_data, |this, result, cx| {
    this.data = result.ok();
    cx.invalidate();
})?;
```

For a one-off paint transition such as hiding an overlay, `cx.request_repaint_at(deadline)` adds
one exact event-loop deadline; it does not enable display-rate rendering.
