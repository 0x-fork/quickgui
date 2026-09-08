//! Optional, language-neutral process/file service. Git parsing and UI live in MoonBit.
//! Only copied JSON and bounded byte chunks cross the extension boundary.
mod abi;

use abi::{Bytes, Extension, ServiceApi, ServiceSink};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    ffi::c_void,
    fs::{self, OpenOptions},
    io::{Read, Write},
    mem::size_of,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    slice,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, LazyLock, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_OUTPUT: usize = 32 * 1024 * 1024;
const MAX_JOBS: usize = 4;
static JOBS: LazyLock<Mutex<HashMap<u64, Arc<AtomicBool>>>> = LazyLock::new(Default::default);
static FILE_JOBS: AtomicUsize = AtomicUsize::new(0);
static SERIAL: AtomicU64 = AtomicU64::new(1);

// The ABI explicitly permits emit/release on workers. Drop runs only after all
// reader threads have returned; the sink is never copied or released twice.
struct Sink(ServiceSink);
unsafe impl Send for Sink {}
unsafe impl Sync for Sink {}
impl Sink {
    fn json(&self, kind: u32, value: Value) {
        let bytes = serde_json::to_vec(&value).unwrap();
        if bytes.len() <= abi::MAX_PAYLOAD {
            unsafe { (self.0.emit)(self.0.context, kind, Bytes::new(&bytes)) }
        } else {
            self.error("Service response exceeds the 64 KiB boundary");
        }
    }
    fn error(&self, error: &str) {
        unsafe { (self.0.emit)(self.0.context, abi::ERROR, Bytes::new(error.as_bytes())) }
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) }
    }
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
fn io_error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn executable(program: &str) -> PathBuf {
    if Path::new(program).components().count() > 1 {
        return PathBuf::from(program);
    }
    let mut dirs: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    if let Some(home) = std::env::var_os("HOME") {
        for suffix in [
            ".local/bin",
            ".bun/bin",
            ".cargo/bin",
            ".local/share/mise/shims",
        ] {
            dirs.push(PathBuf::from(&home).join(suffix));
        }
    }
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].map(PathBuf::from));
    for dir in dirs {
        let path = dir.join(program);
        if is_executable(&path) {
            return path;
        }
        #[cfg(windows)]
        for ext in ["exe", "cmd", "bat"] {
            let path = path.with_extension(ext);
            if path.is_file() {
                return path;
            }
        }
    }
    PathBuf::from(program)
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn stream<R: Read + Send + 'static>(
    mut reader: R,
    sink: Arc<Sink>,
    id: u64,
    name: &'static str,
    size: Arc<AtomicUsize>,
    limit: usize,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            let previous = size.fetch_add(n, Ordering::Relaxed);
            let keep = n.min(limit.saturating_sub(previous));
            if keep > 0 {
                sink.json(
                    2,
                    json!({"job": id, "stream": name, "bytes": STANDARD.encode(&chunk[..keep])}),
                );
            }
            if previous + n > limit {
                stop.store(true, Ordering::Release);
            }
        }
    })
}

fn terminate(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}

fn run(v: Value, id: u64, stop: Arc<AtomicBool>, sink: Arc<Sink>) -> Result<Value, String> {
    let program = text(&v, "program");
    if program.is_empty() {
        return Err("A program is required".into());
    }
    let mut cmd = Command::new(executable(program));
    if let Some(args) = v["args"].as_array() {
        for arg in args {
            cmd.arg(arg.as_str().ok_or("Arguments must be strings")?);
        }
    }
    if !text(&v, "cwd").is_empty() {
        cmd.current_dir(text(&v, "cwd"));
    }
    if let Some(env) = v["env"].as_object() {
        for (key, value) in env {
            cmd.env(
                key,
                value.as_str().ok_or("Environment values must be strings")?,
            );
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stop.load(Ordering::Acquire) {
        return Ok(json!({"code": -1, "cancelled": true}));
    }
    let mut child = cmd.spawn().map_err(io_error)?;
    let size = Arc::new(AtomicUsize::new(0));
    let limit = v["maxOutput"]
        .as_u64()
        .unwrap_or(MAX_OUTPUT as u64)
        .clamp(1, MAX_OUTPUT as u64) as usize;
    let out = stream(
        child.stdout.take().unwrap(),
        sink.clone(),
        id,
        "stdout",
        size.clone(),
        limit,
        stop.clone(),
    );
    let err = stream(
        child.stderr.take().unwrap(),
        sink,
        id,
        "stderr",
        size.clone(),
        limit,
        stop.clone(),
    );
    let mut stdin = child.stdin.take().unwrap();
    let input = text(&v, "input").to_owned();
    let input_file = text(&v, "inputFile").to_owned();
    let writer = thread::spawn(move || -> std::io::Result<()> {
        if input_file.is_empty() {
            stdin.write_all(input.as_bytes())
        } else {
            std::io::copy(
                &mut fs::File::open(input_file)?.take(MAX_OUTPUT as u64),
                &mut stdin,
            )
            .map(|_| ())
        }
    });
    let deadline = Instant::now()
        + Duration::from_millis(v["timeoutMs"].as_u64().unwrap_or(60_000).clamp(1, 600_000));
    let mut timed_out = false;
    let status = loop {
        if stop.load(Ordering::Acquire) || Instant::now() >= deadline {
            timed_out = Instant::now() >= deadline;
            terminate(&mut child);
            break child.wait().map_err(io_error);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(15)),
            Err(error) => {
                terminate(&mut child);
                let _ = child.wait();
                break Err(io_error(error));
            }
        }
    };
    // A child may exit while descendants still hold its pipes. Retire its process
    // group before joining readers so cancellation/deadlines remain bounded.
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let written = writer
        .join()
        .map_err(|_| "Process input worker failed".to_owned())?;
    let _ = out.join();
    let _ = err.join();
    let status = status?;
    if status.success() {
        written.map_err(io_error)?;
    }
    let truncated = size.load(Ordering::Relaxed) > limit;
    Ok(
        json!({"code": status.code().unwrap_or(-1), "timedOut": timed_out,
        "truncated": truncated, "cancelled": stop.load(Ordering::Acquire) && !truncated}),
    )
}

fn file_operation(method: &str, v: &Value) -> Result<Value, String> {
    let path = Path::new(text(v, "path"));
    match method {
        "environment" => Ok(
            json!({"open": std::env::var("QUICK_GIT_OPEN").unwrap_or_default(),
            "now": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            "codex": is_executable(&executable("codex")), "claude": is_executable(&executable("claude"))}),
        ),
        "canonical" => Ok(json!(fs::canonicalize(path)
            .map_err(io_error)?
            .to_string_lossy())),
        "read" => {
            use std::io::{Seek, SeekFrom};
            let mut file = fs::File::open(path).map_err(io_error)?;
            file.seek(SeekFrom::Start(v["offset"].as_u64().unwrap_or(0)))
                .map_err(io_error)?;
            let mut bytes = vec![0; 32 * 1024];
            let length = file.read(&mut bytes).map_err(io_error)?;
            bytes.truncate(length);
            Ok(json!({"bytes": STANDARD.encode(bytes), "length": length}))
        }
        "write" => {
            let data = STANDARD.decode(text(v, "bytes")).map_err(io_error)?;
            let append = v["append"].as_bool().unwrap_or(false);
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(append)
                .truncate(!append)
                .open(path)
                .map_err(io_error)?;
            file.write_all(&data).map_err(io_error)?;
            Ok(Value::Null)
        }
        "save-state" => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            let tmp = path.with_extension(format!(
                "tmp-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            let bytes = if text(v, "inputFile").is_empty() {
                v["state"].to_string().into_bytes()
            } else {
                let mut bytes = Vec::new();
                fs::File::open(text(v, "inputFile"))
                    .map_err(io_error)?
                    .take(4 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(io_error)?;
                if bytes.len() > 4 * 1024 * 1024 {
                    return Err("Settings exceed 4 MiB".into());
                }
                bytes
            };
            fs::write(&tmp, bytes).map_err(io_error)?;
            let result = fs::rename(&tmp, path).map_err(io_error);
            let _ = fs::remove_file(tmp);
            result.map(|_| Value::Null)
        }
        "load-state" => {
            if !path.exists() {
                return Ok(json!({}));
            }
            let mut bytes = Vec::new();
            fs::File::open(path)
                .map_err(io_error)?
                .take(60 * 1024)
                .read_to_end(&mut bytes)
                .map_err(io_error)?;
            serde_json::from_slice(&bytes).map_err(io_error)
        }
        "temp" => {
            let dir = std::env::temp_dir().join(format!(
                "quick-git-moonbit-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&dir).map_err(io_error)?;
            Ok(json!(dir.to_string_lossy()))
        }
        "remove-temp" => {
            if path.parent() != Some(std::env::temp_dir().as_path())
                || !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with(&format!("quick-git-moonbit-{}-", std::process::id()))
            {
                return Err("Only this application's temporary directories may be removed".into());
            }
            fs::remove_dir_all(path).map_err(io_error)?;
            Ok(Value::Null)
        }
        _ => Err(format!("Unknown I/O method: {method}")),
    }
}

unsafe extern "C" fn invoke(_: u32, method: Bytes, params: Bytes, raw_sink: ServiceSink) {
    let sink = Arc::new(Sink(raw_sink));
    if method.data.is_null()
        || params.data.is_null()
        || method.len > abi::MAX_PAYLOAD
        || params.len > abi::MAX_PAYLOAD
    {
        sink.error("Invalid request");
        return;
    }
    let method =
        String::from_utf8_lossy(slice::from_raw_parts(method.data, method.len)).into_owned();
    let value: Value = match serde_json::from_slice(slice::from_raw_parts(params.data, params.len))
    {
        Ok(value) => value,
        Err(error) => {
            sink.error(&error.to_string());
            return;
        }
    };
    let id = value["id"].as_u64().unwrap_or(0);
    if method == "cancel" {
        if let Some(stop) = JOBS.lock().unwrap().get(&id) {
            stop.store(true, Ordering::Release);
        }
        sink.json(abi::REPLY, Value::Null);
    } else if method == "run" {
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut jobs = JOBS.lock().unwrap();
            if id == 0 || jobs.len() >= MAX_JOBS || jobs.contains_key(&id) {
                sink.error("Process concurrency limit reached or duplicate job");
                return;
            }
            jobs.insert(id, stop.clone());
        }
        thread::spawn(move || {
            let result = run(value, id, stop, sink.clone());
            JOBS.lock().unwrap().remove(&id);
            match result {
                Ok(value) => sink.json(abi::REPLY, value),
                Err(error) => sink.error(&error),
            }
        });
    } else {
        if FILE_JOBS.fetch_add(1, Ordering::AcqRel) >= 16 {
            FILE_JOBS.fetch_sub(1, Ordering::AcqRel);
            sink.error("File I/O concurrency limit reached");
            return;
        }
        thread::spawn(move || {
            let result = file_operation(&method, &value);
            FILE_JOBS.fetch_sub(1, Ordering::AcqRel);
            match result {
                Ok(value) => sink.json(abi::REPLY, value),
                Err(error) => sink.error(&error),
            }
        });
    }
}

extern "C" fn shutdown() {
    for stop in JOBS.lock().unwrap().values() {
        stop.store(true, Ordering::Release);
    }
}
static API: ServiceApi = ServiceApi { invoke, shutdown };
static DESCRIPTOR: Extension = Extension {
    abi_version: abi::ABI_VERSION,
    descriptor_size: size_of::<Extension>() as u32,
    kind: abi::SERVICE,
    api_size: size_of::<ServiceApi>() as u32,
    name: Bytes::new(b"quick-git-io"),
    version: Bytes::new(b"1.0.0"),
    api: &API as *const ServiceApi as *const c_void,
};
#[no_mangle]
pub extern "C" fn quickgui_extension_v1() -> *const Extension {
    &DESCRIPTOR
}

#[cfg(test)]
mod tests;
