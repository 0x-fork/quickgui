//! Deferred C event delivery for runtimes whose callbacks must run on their own thread.
//! Only the zero-argument notification crosses threads. Borrowed event spans are copied here
//! and delivered by `quickgui_drain_events` on the caller's thread, without idle polling.

use super::*;

const MAX_EVENT_BYTES: usize = 32 * 1024 * 1024;

struct QueuedEvent {
    kind: Vec<u8>,
    window: u32,
    target: u32,
    flags: u32,
    value: Vec<u8>,
    extra: Vec<u8>,
    data: Vec<u8>,
}

impl QueuedEvent {
    fn bytes(&self) -> usize {
        self.kind.len() + self.value.len() + self.extra.len() + self.data.len()
    }
}

#[derive(Default)]
struct Queue {
    events: VecDeque<QueuedEvent>,
    bytes: usize,
    overflow: bool,
    notify: Option<unsafe extern "C" fn()>,
}

impl Queue {
    fn push(&mut self, event: QueuedEvent) -> bool {
        let wake = self.events.is_empty() && !self.overflow;
        if self.events.len() >= MAX_QUEUED_EVENTS || event.bytes() > MAX_EVENT_BYTES - self.bytes {
            self.overflow = true;
        } else if !self.overflow {
            self.bytes += event.bytes();
            self.events.push_back(event);
        }
        wake
    }

    fn pop(&mut self) -> Option<QueuedEvent> {
        let event = self.events.pop_front()?;
        self.bytes -= event.bytes();
        Some(event)
    }
}

static QUEUE: LazyLock<Mutex<Queue>> = LazyLock::new(|| Mutex::new(Queue::default()));

unsafe extern "C" fn receive(
    kind: *const u8,
    kind_len: usize,
    window: u32,
    target: u32,
    flags: u32,
    value: *const u8,
    value_len: usize,
    extra: *const u8,
    extra_len: usize,
    data: *const u8,
    data_len: usize,
    _context: *mut c_void,
) {
    unsafe fn copy(pointer: *const u8, length: usize) -> Vec<u8> {
        if length == 0 {
            Vec::new()
        } else {
            // SAFETY: EventSink lends these spans until this callback returns.
            unsafe { std::slice::from_raw_parts(pointer, length) }.to_vec()
        }
    }
    {
        let mut queue = lock(&QUEUE);
        let Some(notify) = queue.notify else {
            return;
        };
        // Check before copying, so even a rejected payload cannot allocate beyond the limit.
        let length = kind_len
            .saturating_add(value_len)
            .saturating_add(extra_len)
            .saturating_add(data_len);
        let wake = if length > MAX_EVENT_BYTES - queue.bytes {
            let wake = queue.events.is_empty() && !queue.overflow;
            queue.overflow = true;
            wake
        } else {
            queue.push(QueuedEvent {
                kind: unsafe { copy(kind, kind_len) },
                window,
                target,
                flags,
                value: unsafe { copy(value, value_len) },
                extra: unsafe { copy(extra, extra_len) },
                data: unsafe { copy(data, data_len) },
            })
        };
        // The notifier only schedules work. Hold the lock across scheduling so clearing it
        // is a barrier: no native thread can invoke a stale Bun trampoline after unregister.
        if wake {
            unsafe { notify() };
        }
    }
}

/// Select deferred event delivery. Register once before creating the application.
///
/// # Safety
/// `notify` must be nonblocking and callable from any native thread until unregistered.
/// It schedules a call to `quickgui_drain_events`; it must not drain synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_set_event_notifier(notify: unsafe extern "C" fn()) {
    lock(&QUEUE).notify = Some(notify);
    HOST.set_sink(Some(EventSink {
        callback: receive,
        context: 0,
    }));
}

/// Stop scheduling notifications before destroying the frontend runtime. Already-scheduled
/// callbacks remain owned by that runtime; it must retain them until its own queue is stopped.
#[unsafe(no_mangle)]
pub extern "C" fn quickgui_clear_event_notifier() {
    {
        let mut queue = lock(&QUEUE);
        queue.notify = None;
        queue.events.clear();
        queue.bytes = 0;
        queue.overflow = false;
    }
    HOST.set_sink(None);
}

/// Deliver up to 256 queued events synchronously on the calling thread. Repeat until 0.
/// Returns -1 on queue overflow; the frontend must abort instead of losing events silently.
///
/// # Safety
/// `callback` must copy borrowed spans before returning and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_drain_events(
    callback: EventCallback,
    context: *mut c_void,
) -> i32 {
    let mut count = 0;
    while count < 256 {
        let event = {
            let mut queue = lock(&QUEUE);
            if queue.overflow {
                return -1;
            }
            queue.pop()
        };
        let Some(event) = event else {
            break;
        };
        unsafe {
            callback(
                event.kind.as_ptr(),
                event.kind.len(),
                event.window,
                event.target,
                event.flags,
                event.value.as_ptr(),
                event.value.len(),
                event.extra.as_ptr(),
                event.extra.len(),
                event.data.as_ptr(),
                event.data.len(),
                context,
            );
        }
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(value: u8) -> QueuedEvent {
        QueuedEvent {
            kind: b"input".to_vec(),
            window: 1,
            target: 2,
            flags: 1,
            value: vec![value],
            extra: Vec::new(),
            data: Vec::new(),
        }
    }

    #[test]
    fn deferred_events_coalesce_wakes_preserve_order_and_release_bytes() {
        let mut queue = Queue::default();
        assert!(queue.push(event(1)));
        assert!(!queue.push(event(2)));
        assert_eq!(queue.pop().unwrap().value, [1]);
        assert_eq!(queue.pop().unwrap().value, [2]);
        assert_eq!(queue.bytes, 0);
        assert!(queue.push(event(3)));
    }

    #[test]
    fn deferred_events_reject_overflow_without_growing_the_queue() {
        let mut queue = Queue::default();
        for _ in 0..MAX_QUEUED_EVENTS {
            queue.push(event(0));
        }
        queue.push(event(1));
        assert!(queue.overflow);
        assert_eq!(queue.events.len(), MAX_QUEUED_EVENTS);
        let mut queue = Queue {
            bytes: MAX_EVENT_BYTES,
            ..Queue::default()
        };
        assert!(queue.push(event(1)));
        assert!(queue.overflow);
        assert!(queue.events.is_empty());
    }
}
