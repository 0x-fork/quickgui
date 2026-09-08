use super::*;

#[derive(Default)]
struct Capture {
    events: Mutex<Vec<(u32, Vec<u8>)>>,
    releases: AtomicUsize,
}

unsafe extern "C" fn capture(context: *mut c_void, kind: u32, bytes: Bytes) {
    let state = &*(context as *const Arc<Capture>);
    assert!(bytes.len <= abi::MAX_PAYLOAD);
    state
        .events
        .lock()
        .unwrap()
        .push((kind, slice::from_raw_parts(bytes.data, bytes.len).to_vec()));
}

unsafe extern "C" fn release(context: *mut c_void) {
    let state = Box::from_raw(context as *mut Arc<Capture>);
    state.releases.fetch_add(1, Ordering::SeqCst);
}

fn sink() -> (Arc<Capture>, Arc<Sink>) {
    let state = Arc::new(Capture::default());
    let sink = Arc::new(Sink(ServiceSink {
        context: Box::into_raw(Box::new(state.clone())) as *mut c_void,
        emit: capture,
        release,
    }));
    (state, sink)
}

impl Capture {
    fn bytes(&self, stream: &str) -> Vec<u8> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(kind, data)| {
                if *kind != 2 {
                    return None;
                }
                let event: Value = serde_json::from_slice(data).unwrap();
                (event["stream"] == stream).then(|| STANDARD.decode(text(&event, "bytes")).unwrap())
            })
            .flatten()
            .collect()
    }
}

#[cfg(unix)]
fn shell(
    script: &str,
    input: Value,
    stop: Arc<AtomicBool>,
) -> (Result<Value, String>, Arc<Capture>) {
    let (capture, sink) = sink();
    let mut options = json!({"program": "/bin/sh", "args": ["-c", script], "timeoutMs": 5000});
    options
        .as_object_mut()
        .unwrap()
        .extend(input.as_object().unwrap().clone());
    let result = run(options, 1, stop, sink);
    assert_eq!(capture.releases.load(Ordering::SeqCst), 1);
    (result, capture)
}

#[test]
#[cfg(unix)]
fn preserves_separate_byte_streams_stdin_and_exit_code() {
    let (result, capture) = shell(
        "cat; printf '\\377error' >&2; exit 7",
        json!({"input": "MoonBit 月\n"}),
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(result.unwrap()["code"], 7);
    assert_eq!(capture.bytes("stdout"), "MoonBit 月\n".as_bytes());
    assert_eq!(capture.bytes("stderr"), b"\xfferror");
}

#[test]
#[cfg(unix)]
fn output_limit_stops_the_process_without_exceeding_the_byte_budget() {
    let (result, capture) = shell(
        "while :; do printf 0123456789abcdef; done",
        json!({"maxOutput": 97}),
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(result.unwrap()["truncated"], true);
    assert_eq!(capture.bytes("stdout").len(), 97);
}

#[test]
#[cfg(unix)]
fn timeout_retires_descendants_holding_pipes() {
    let before = Instant::now();
    let (result, _) = shell(
        "sleep 30 & wait",
        json!({"timeoutMs": 20}),
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(result.unwrap()["timedOut"], true);
    assert!(before.elapsed() < Duration::from_secs(3));
}

#[test]
#[cfg(unix)]
fn cancellation_retires_descendants_holding_pipes() {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let cancel = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        flag.store(true, Ordering::Release);
    });
    let before = Instant::now();
    let (result, _) = shell("sleep 30 & wait", json!({}), stop);
    cancel.join().unwrap();
    assert_eq!(result.unwrap()["cancelled"], true);
    assert!(before.elapsed() < Duration::from_secs(3));
}

#[test]
#[cfg(unix)]
fn exited_parent_cannot_strand_a_pipe_reader() {
    let before = Instant::now();
    let (result, capture) = shell(
        "sleep 30 & printf done",
        json!({}),
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(result.unwrap()["code"], 0);
    assert_eq!(capture.bytes("stdout"), b"done");
    assert!(before.elapsed() < Duration::from_secs(3));
}

#[test]
#[cfg(unix)]
fn missing_input_file_is_reported_even_if_child_succeeds() {
    let temp = temporary();
    let (result, _) = shell(
        "cat",
        json!({"inputFile": temp.0.join("missing")}),
        Arc::new(AtomicBool::new(false)),
    );
    assert!(result.is_err());
}

#[test]
fn cancellation_before_spawn_does_not_start_a_program() {
    let (capture, sink) = sink();
    let result = run(
        json!({"program": "quickgui-nonexistent-test-program"}),
        1,
        Arc::new(AtomicBool::new(true)),
        sink,
    )
    .unwrap();
    assert_eq!(result["cancelled"], true);
    assert!(capture.events.lock().unwrap().is_empty());
    assert_eq!(capture.releases.load(Ordering::SeqCst), 1);
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = file_operation("remove-temp", &json!({"path": self.0}));
    }
}
fn temporary() -> Temporary {
    Temporary(PathBuf::from(
        file_operation("temp", &json!({}))
            .unwrap()
            .as_str()
            .unwrap(),
    ))
}

#[test]
fn file_chunks_and_atomic_preferences_round_trip() {
    let temporary = temporary();
    let path = temporary.0.join("bytes");
    let data: Vec<u8> = (0..40000).map(|i| (i % 256) as u8).collect();
    file_operation(
        "write",
        &json!({"path": path, "bytes": STANDARD.encode(&data[..32768])}),
    )
    .unwrap();
    file_operation(
        "write",
        &json!({"path": path, "bytes": STANDARD.encode(&data[32768..]), "append": true}),
    )
    .unwrap();
    let first = file_operation("read", &json!({"path": path})).unwrap();
    let second = file_operation("read", &json!({"path": path, "offset": 32768})).unwrap();
    assert_eq!(
        STANDARD.decode(text(&first, "bytes")).unwrap(),
        data[..32768]
    );
    assert_eq!(
        STANDARD.decode(text(&second, "bytes")).unwrap(),
        data[32768..]
    );
    let state = temporary.0.join("settings/state.json");
    for number in [1, 2] {
        file_operation(
            "save-state",
            &json!({"path": state, "state": {"draft": number}}),
        )
        .unwrap();
        assert_eq!(
            file_operation("load-state", &json!({"path": state})).unwrap(),
            json!({"draft": number})
        );
    }
    assert_eq!(fs::read_dir(state.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn temporary_cleanup_rejects_unowned_paths() {
    let temporary = temporary();
    let child = temporary.0.join("unowned");
    fs::create_dir(&child).unwrap();
    assert!(file_operation("remove-temp", &json!({"path": child})).is_err());
    assert!(child.exists());
    file_operation("remove-temp", &json!({"path": temporary.0})).unwrap();
    assert!(!temporary.0.exists());
}

#[test]
fn large_settings_use_a_staged_file_and_atomic_replace() {
    let temp = temporary();
    let input = temp.0.join("input");
    let output = temp.0.join("settings");
    let state = json!({"draft": "月".repeat(30000)}).to_string();
    fs::write(&input, &state).unwrap();
    file_operation("save-state", &json!({"path": output, "inputFile": input})).unwrap();
    assert_eq!(fs::read_to_string(output).unwrap(), state);
}
