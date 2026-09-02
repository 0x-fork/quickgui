//! Bounded crash reporting: panic reports, native fatal-signal reports, retention, and upload.
//!
//! Everything the reporter writes is bounded before it reaches the filesystem: report bodies,
//! extra parameters, retained report counts, and the number of reports one upload attempts.
//! The native fatal-signal path never allocates, never locks, and never formats through `std`;
//! it writes a pre-rendered byte template plus a handful of digits into a file descriptor that
//! was opened while the process was still healthy.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    panic::PanicHookInfo,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{AppInfo, Result, SystemIntegrationError, invalid};

/// Schema version stamped into every report QuickGUI writes.
pub const CRASH_REPORT_SCHEMA_VERSION: u32 = 1;
/// Hard ceiling for [`CrashReporterOptions::max_reports`].
pub const MAX_CRASH_REPORTS: usize = 128;
/// Retention used when the caller does not choose one.
pub const DEFAULT_MAX_CRASH_REPORTS: usize = 16;
/// Hard ceiling for [`CrashReporterOptions::max_report_bytes`].
pub const MAX_CRASH_REPORT_BYTES: usize = 1024 * 1024;
/// Report size limit used when the caller does not choose one.
pub const DEFAULT_MAX_CRASH_REPORT_BYTES: usize = 64 * 1024;
/// Maximum number of retained extra parameters.
pub const MAX_CRASH_EXTRA_PARAMETERS: usize = 32;
/// Maximum UTF-8 length of one extra-parameter key.
pub const MAX_CRASH_PARAMETER_KEY_BYTES: usize = 128;
/// Maximum UTF-8 length of one extra-parameter value.
pub const MAX_CRASH_PARAMETER_VALUE_BYTES: usize = 4 * 1024;
/// Maximum retained panic/hang message length; longer messages are truncated.
pub const MAX_CRASH_MESSAGE_BYTES: usize = 8 * 1024;
/// Maximum retained backtrace length; longer backtraces are truncated.
pub const MAX_CRASH_BACKTRACE_BYTES: usize = 32 * 1024;
/// Maximum number of reports one [`CrashReporter::upload_pending`] call sends.
pub const MAX_CRASH_UPLOAD_REPORTS: usize = 32;
/// Shortest accepted watchdog poll interval.
pub const MIN_WATCHDOG_INTERVAL: Duration = Duration::from_millis(10);
/// Longest accepted watchdog poll interval.
pub const MAX_WATCHDOG_INTERVAL: Duration = Duration::from_secs(300);
/// Watchdog poll interval used when the caller does not choose one.
pub const DEFAULT_WATCHDOG_INTERVAL: Duration = Duration::from_secs(5);
/// Hang threshold used when the caller does not choose one.
pub const DEFAULT_WATCHDOG_HANG_THRESHOLD: Duration = Duration::from_secs(15);

const REPORT_SUFFIX: &str = ".crash.json";
const PARTIAL_SUFFIX: &str = ".crash.partial";

/// How a report was produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrashKind {
    /// A Rust panic observed by the installed panic hook.
    Panic,
    /// A native fatal signal or structured exception.
    Signal,
    /// A main-thread heartbeat that stopped within the configured threshold.
    Hang,
}

impl CrashKind {
    /// The stable wire name used in report JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Panic => "panic",
            Self::Signal => "signal",
            Self::Hang => "hang",
        }
    }
}

/// Whether a panic report captures a `std::backtrace::Backtrace`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BacktracePolicy {
    /// Never capture a backtrace.
    Disabled,
    /// Capture only when `RUST_BACKTRACE` enables it (the default).
    #[default]
    Environment,
    /// Always capture, regardless of the environment.
    Always,
}

/// Source position recorded for a panic report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CrashLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// One bounded crash report, as written to and parsed back from disk.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub schema_version: u32,
    pub id: String,
    pub kind: CrashKind,
    pub timestamp: String,
    pub app_name: String,
    pub app_version: String,
    pub app_identifier: String,
    pub operating_system: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operating_system_version: Option<String>,
    pub architecture: String,
    pub process_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<CrashLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault_address: Option<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
}

/// Result of one bounded [`CrashReporter::upload_pending`] pass.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UploadSummary {
    /// Reports the pass tried to send, at most [`MAX_CRASH_UPLOAD_REPORTS`].
    pub attempted: usize,
    /// Reports accepted by the endpoint and then deleted from disk.
    pub uploaded: usize,
    /// Reports the endpoint rejected or that could not be sent; these stay on disk.
    pub failed: usize,
}

/// Bounded configuration for [`CrashReporter::install`].
#[derive(Clone, Debug)]
pub struct CrashReporterOptions {
    app: AppInfo,
    directory: Option<PathBuf>,
    max_reports: usize,
    max_report_bytes: usize,
    parameters: BTreeMap<String, String>,
    upload_endpoint: Option<String>,
    backtrace: BacktracePolicy,
    capture_signals: bool,
}

impl CrashReporterOptions {
    /// Defaults: `<AppPaths::log_dir>/crashes`, 16 retained reports, 64 KiB per report,
    /// environment-controlled backtraces, and native fatal-signal capture enabled.
    pub fn new(app: AppInfo) -> Self {
        Self {
            app,
            directory: None,
            max_reports: DEFAULT_MAX_CRASH_REPORTS,
            max_report_bytes: DEFAULT_MAX_CRASH_REPORT_BYTES,
            parameters: BTreeMap::new(),
            upload_endpoint: None,
            backtrace: BacktracePolicy::default(),
            capture_signals: true,
        }
    }

    /// Override the report directory. The path must be absolute.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Retain at most `maximum` reports, capped at [`MAX_CRASH_REPORTS`].
    pub fn max_reports(mut self, maximum: usize) -> Result<Self> {
        if maximum == 0 || maximum > MAX_CRASH_REPORTS {
            return Err(invalid(format!(
                "the crash report retention must be between 1 and {MAX_CRASH_REPORTS}"
            )));
        }
        self.max_reports = maximum;
        Ok(self)
    }

    /// Write at most `maximum` bytes per report, capped at [`MAX_CRASH_REPORT_BYTES`].
    pub fn max_report_bytes(mut self, maximum: usize) -> Result<Self> {
        if !(1024..=MAX_CRASH_REPORT_BYTES).contains(&maximum) {
            return Err(invalid(format!(
                "the crash report size limit must be between 1024 and {MAX_CRASH_REPORT_BYTES} bytes"
            )));
        }
        self.max_report_bytes = maximum;
        Ok(self)
    }

    /// Add one bounded extra parameter recorded in every report.
    pub fn extra_parameter(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self> {
        let (key, value) = validate_parameter(key.into(), value.into())?;
        if self.parameters.len() >= MAX_CRASH_EXTRA_PARAMETERS
            && !self.parameters.contains_key(&key)
        {
            return Err(invalid(format!(
                "at most {MAX_CRASH_EXTRA_PARAMETERS} crash-report parameters are retained"
            )));
        }
        self.parameters.insert(key, value);
        Ok(self)
    }

    /// Remember a default HTTPS upload endpoint for [`CrashReporter::upload_pending`].
    pub fn upload_endpoint(mut self, endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();
        validate_https_url(&endpoint, "the crash upload endpoint")?;
        self.upload_endpoint = Some(endpoint);
        Ok(self)
    }

    /// Choose whether panic reports carry a backtrace.
    pub fn backtrace(mut self, policy: BacktracePolicy) -> Self {
        self.backtrace = policy;
        self
    }

    /// Disable the native fatal-signal handler and keep only the Rust panic hook.
    pub fn capture_signals(mut self, capture: bool) -> Self {
        self.capture_signals = capture;
        self
    }
}

struct ReporterState {
    directory: PathBuf,
    app_name: String,
    app_version: String,
    app_identifier: String,
    max_reports: usize,
    max_report_bytes: usize,
    parameters: Mutex<BTreeMap<String, String>>,
    upload_endpoint: Option<String>,
    backtrace: BacktracePolicy,
    sequence: AtomicUsize,
}

/// Cheap cloneable handle to the installed crash reporter.
#[derive(Clone)]
pub struct CrashReporter(Arc<ReporterState>);

impl std::fmt::Debug for CrashReporter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CrashReporter")
            .field("directory", &self.0.directory)
            .field("max_reports", &self.0.max_reports)
            .field("max_report_bytes", &self.0.max_report_bytes)
            .finish()
    }
}

static INSTALLED: OnceLock<CrashReporter> = OnceLock::new();

impl CrashReporter {
    /// Install the process-wide panic hook and, unless disabled, the native fatal-signal handler.
    ///
    /// Installing twice is an error; use [`CrashReporter::current`] to reach the existing handle.
    pub fn install(options: CrashReporterOptions) -> Result<Self> {
        if INSTALLED.get().is_some() {
            return Err(invalid("the crash reporter is already installed"));
        }
        let capture_signals = options.capture_signals;
        let reporter = Self::detached(options)?;
        INSTALLED
            .set(reporter.clone())
            .map_err(|_| invalid("the crash reporter is already installed"))?;
        install_panic_hook();
        if capture_signals {
            install_signal_handler(&reporter);
        }
        Ok(reporter)
    }

    /// Build a reporter that owns its directory but installs no process-wide hooks.
    ///
    /// Deterministic tests and secondary tools use this to exercise writing, retention, parsing,
    /// and upload without changing global process state.
    pub fn detached(options: CrashReporterOptions) -> Result<Self> {
        let directory = match options.directory {
            Some(directory) => directory,
            None => default_directory(&options.app)?,
        };
        validate_directory(&directory)?;
        fs::create_dir_all(&directory).map_err(io_error)?;
        let reporter = Self(Arc::new(ReporterState {
            directory,
            app_name: options.app.name().to_owned(),
            app_version: options.app.version().to_owned(),
            app_identifier: options.app.identifier().to_owned(),
            max_reports: options.max_reports,
            max_report_bytes: options.max_report_bytes,
            parameters: Mutex::new(options.parameters),
            upload_endpoint: options.upload_endpoint,
            backtrace: options.backtrace,
            sequence: AtomicUsize::new(0),
        }));
        promote_partial_reports(reporter.directory())?;
        prune_reports(reporter.directory(), reporter.0.max_reports)?;
        Ok(reporter)
    }

    /// The installed reporter, when [`CrashReporter::install`] has already succeeded.
    pub fn current() -> Option<Self> {
        INSTALLED.get().cloned()
    }

    /// Directory holding retained reports.
    pub fn directory(&self) -> &Path {
        &self.0.directory
    }

    /// Configured retention count.
    pub fn max_reports(&self) -> usize {
        self.0.max_reports
    }

    /// Configured per-report byte limit.
    pub fn max_report_bytes(&self) -> usize {
        self.0.max_report_bytes
    }

    /// Add or replace one bounded extra parameter recorded in later reports.
    pub fn add_extra_parameter(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        let (key, value) = validate_parameter(key.into(), value.into())?;
        let mut parameters = self.parameters();
        if parameters.len() >= MAX_CRASH_EXTRA_PARAMETERS && !parameters.contains_key(&key) {
            return Err(invalid(format!(
                "at most {MAX_CRASH_EXTRA_PARAMETERS} crash-report parameters are retained"
            )));
        }
        parameters.insert(key, value);
        Ok(())
    }

    /// Remove one extra parameter, reporting whether it was present.
    pub fn remove_extra_parameter(&self, key: &str) -> bool {
        self.parameters().remove(key).is_some()
    }

    /// A snapshot of the retained extra parameters.
    pub fn extra_parameters(&self) -> BTreeMap<String, String> {
        self.parameters().clone()
    }

    /// Write one caller-built report atomically and apply retention.
    pub fn capture_report(&self, report: &CrashReport) -> Result<PathBuf> {
        let path = write_report_atomically(&self.0.directory, report, self.0.max_report_bytes)?;
        prune_reports(&self.0.directory, self.0.max_reports)?;
        Ok(path)
    }

    /// The newest retained report, parsed from disk.
    pub fn last_crash_report(&self) -> Result<Option<CrashReport>> {
        Ok(self.pending_reports()?.pop())
    }

    /// Every retained report in oldest-to-newest order, bounded by the configured retention.
    pub fn pending_reports(&self) -> Result<Vec<CrashReport>> {
        read_reports(
            &self.0.directory,
            self.0.max_reports,
            self.0.max_report_bytes,
        )
    }

    /// Delete one retained report by identifier, reporting whether it existed.
    pub fn delete_report(&self, id: &str) -> Result<bool> {
        validate_report_id(id)?;
        let path = self.0.directory.join(format!("{id}{REPORT_SUFFIX}"));
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(io_error(error)),
        }
    }

    /// The endpoint configured through [`CrashReporterOptions::upload_endpoint`].
    pub fn upload_endpoint(&self) -> Option<&str> {
        self.0.upload_endpoint.as_deref()
    }

    /// Build the exact JSON body one upload sends for a report.
    pub fn upload_body(report: &CrashReport) -> Result<Vec<u8>> {
        serde_json::to_vec(report).map_err(io_error)
    }

    /// POST every retained report to `endpoint`, deleting the ones the endpoint accepts.
    ///
    /// At most [`MAX_CRASH_UPLOAD_REPORTS`] reports are sent per call. Reports the endpoint
    /// rejects stay on disk for the next attempt.
    pub fn upload_pending(&self, endpoint: &str) -> Result<UploadSummary> {
        validate_https_url(endpoint, "the crash upload endpoint")?;
        let reports = self.pending_reports()?;
        let mut summary = UploadSummary::default();
        let agent = https_agent();
        for report in reports.into_iter().take(MAX_CRASH_UPLOAD_REPORTS) {
            summary.attempted += 1;
            let body = Self::upload_body(&report)?;
            match agent
                .post(endpoint)
                .header("content-type", "application/json")
                .send(&body[..])
            {
                Ok(response) if (200..300).contains(&response.status().as_u16()) => {
                    self.delete_report(&report.id)?;
                    summary.uploaded += 1;
                }
                _ => summary.failed += 1,
            }
        }
        Ok(summary)
    }

    /// Build a report for the current process using the reporter's identity and parameters.
    pub fn build_report(&self, kind: CrashKind, message: impl Into<String>) -> CrashReport {
        let system = crate::SystemInfo::current();
        CrashReport {
            schema_version: CRASH_REPORT_SCHEMA_VERSION,
            id: self.next_report_id(),
            kind,
            timestamp: format_rfc3339(unix_seconds()),
            app_name: self.0.app_name.clone(),
            app_version: self.0.app_version.clone(),
            app_identifier: self.0.app_identifier.clone(),
            operating_system: system.name().to_owned(),
            operating_system_version: system.version().map(str::to_owned),
            architecture: system.architecture().to_owned(),
            process_id: std::process::id(),
            thread: std::thread::current()
                .name()
                .map(|name| truncate_utf8(name, MAX_CRASH_PARAMETER_VALUE_BYTES).to_owned()),
            message: truncate_utf8(&message.into(), MAX_CRASH_MESSAGE_BYTES).to_owned(),
            location: None,
            backtrace: None,
            signal: None,
            signal_name: None,
            fault_address: None,
            parameters: self.extra_parameters(),
        }
    }

    fn record_panic(&self, info: &PanicHookInfo<'_>) -> Result<PathBuf> {
        let message = panic_message(info);
        let mut report = self.build_report(CrashKind::Panic, message);
        report.location = info.location().map(|location| CrashLocation {
            file: truncate_utf8(location.file(), MAX_CRASH_PARAMETER_VALUE_BYTES).to_owned(),
            line: location.line(),
            column: location.column(),
        });
        report.backtrace = capture_backtrace(self.0.backtrace);
        self.capture_report(&report)
    }

    fn next_report_id(&self) -> String {
        let sequence = self.0.sequence.fetch_add(1, Ordering::Relaxed);
        report_id(unix_millis(), std::process::id(), sequence)
    }

    fn parameters(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.0
            .parameters
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Opt-in main-thread hang detector.
///
/// The watchdog is **disabled by default** and costs one sleeping thread that wakes every
/// [`WatchdogOptions::interval`]. At the default five-second interval this is 12 wakeups per
/// minute and no allocation; enable it only when hang telemetry is worth that idle cost.
pub struct Watchdog {
    shared: Arc<WatchdogShared>,
    handle: Option<JoinHandle<()>>,
}

/// Bounded watchdog configuration.
#[derive(Clone, Copy, Debug)]
pub struct WatchdogOptions {
    interval: Duration,
    hang_threshold: Duration,
}

impl Default for WatchdogOptions {
    fn default() -> Self {
        Self {
            interval: DEFAULT_WATCHDOG_INTERVAL,
            hang_threshold: DEFAULT_WATCHDOG_HANG_THRESHOLD,
        }
    }
}

impl WatchdogOptions {
    /// Poll interval, clamped to [`MIN_WATCHDOG_INTERVAL`]..=[`MAX_WATCHDOG_INTERVAL`].
    pub fn interval(mut self, interval: Duration) -> Result<Self> {
        if interval < MIN_WATCHDOG_INTERVAL || interval > MAX_WATCHDOG_INTERVAL {
            return Err(invalid(
                "the watchdog interval must be between 10 milliseconds and 300 seconds",
            ));
        }
        self.interval = interval;
        Ok(self)
    }

    /// Report a hang once the main thread has not beaten for this long.
    pub fn hang_threshold(mut self, threshold: Duration) -> Result<Self> {
        if threshold < MIN_WATCHDOG_INTERVAL || threshold > MAX_WATCHDOG_INTERVAL {
            return Err(invalid(
                "the watchdog hang threshold must be between 10 milliseconds and 300 seconds",
            ));
        }
        self.hang_threshold = threshold;
        Ok(self)
    }
}

struct WatchdogShared {
    origin: Instant,
    last_beat_millis: AtomicU64,
    reported: AtomicBool,
    stopped: AtomicBool,
    wake: Mutex<bool>,
    condition: Condvar,
}

impl Watchdog {
    /// Start the monitor thread. The caller must call [`Watchdog::heartbeat`] from the thread
    /// whose responsiveness is being observed.
    pub fn start(reporter: &CrashReporter, options: WatchdogOptions) -> Result<Self> {
        let shared = Arc::new(WatchdogShared {
            origin: Instant::now(),
            last_beat_millis: AtomicU64::new(0),
            reported: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            wake: Mutex::new(false),
            condition: Condvar::new(),
        });
        let monitor = Arc::clone(&shared);
        let reporter = reporter.clone();
        let interval = options.interval;
        let threshold = options.hang_threshold;
        let handle = std::thread::Builder::new()
            .name("quickgui-watchdog".to_owned())
            .spawn(move || {
                loop {
                    let mut wake = monitor
                        .wake
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    while !*wake {
                        let (guard, timeout) = monitor
                            .condition
                            .wait_timeout(wake, interval)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        wake = guard;
                        if timeout.timed_out() {
                            break;
                        }
                    }
                    drop(wake);
                    if monitor.stopped.load(Ordering::Acquire) {
                        return;
                    }
                    let elapsed = monitor
                        .origin
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    let last = monitor.last_beat_millis.load(Ordering::Acquire);
                    if watchdog_should_report(
                        elapsed,
                        last,
                        threshold,
                        monitor.reported.load(Ordering::Acquire),
                    ) {
                        monitor.reported.store(true, Ordering::Release);
                        let report = reporter.build_report(
                            CrashKind::Hang,
                            format!(
                                "the observed thread did not beat for at least {} ms",
                                threshold.as_millis()
                            ),
                        );
                        let _ = reporter.capture_report(&report);
                    }
                }
            })
            .map_err(io_error)?;
        Ok(Self {
            shared,
            handle: Some(handle),
        })
    }

    /// Record that the observed thread is still running its loop.
    pub fn heartbeat(&self) {
        let elapsed = self
            .shared
            .origin
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        self.shared
            .last_beat_millis
            .store(elapsed, Ordering::Release);
        self.shared.reported.store(false, Ordering::Release);
    }

    /// Whether the watchdog has already written a hang report for the current stall.
    pub fn reported_hang(&self) -> bool {
        self.shared.reported.load(Ordering::Acquire)
    }

    /// Stop the monitor thread and wait for it to exit.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.shared.stopped.store(true, Ordering::Release);
        {
            let mut wake = self
                .shared
                .wake
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *wake = true;
        }
        self.shared.condition.notify_all();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn watchdog_should_report(
    elapsed_millis: u64,
    last_beat_millis: u64,
    threshold: Duration,
    already_reported: bool,
) -> bool {
    if already_reported {
        return false;
    }
    let threshold = threshold.as_millis().min(u128::from(u64::MAX)) as u64;
    elapsed_millis.saturating_sub(last_beat_millis) >= threshold
}

// ---------------------------------------------------------------------------
// Directory helpers, shared by the reporter and by deterministic tests.
// ---------------------------------------------------------------------------

fn default_directory(app: &AppInfo) -> Result<PathBuf> {
    let paths = app.paths()?;
    let base = paths
        .log_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths.temp_dir().to_path_buf());
    Ok(base.join("crashes"))
}

fn validate_directory(directory: &Path) -> Result<()> {
    let bytes = directory.as_os_str().as_encoded_bytes();
    if !directory.is_absolute() || bytes.is_empty() || bytes.contains(&0) || bytes.len() > 32 * 1024
    {
        return Err(invalid(
            "the crash report directory must be absolute, NUL-free, and at most 32768 encoded bytes",
        ));
    }
    Ok(())
}

fn report_id(millis: u64, process_id: u32, sequence: usize) -> String {
    format!("{millis:013}-{process_id}-{sequence:04}")
}

fn validate_report_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(invalid(
            "a crash report identifier must be nonempty ASCII alphanumerics or hyphens",
        ));
    }
    Ok(())
}

/// Serialize, size-check, and atomically publish one report.
fn write_report_atomically(
    directory: &Path,
    report: &CrashReport,
    max_report_bytes: usize,
) -> Result<PathBuf> {
    validate_report_id(&report.id)?;
    fs::create_dir_all(directory).map_err(io_error)?;
    let mut bytes = serde_json::to_vec(report).map_err(io_error)?;
    if bytes.len() > max_report_bytes {
        let mut trimmed = report.clone();
        trimmed.backtrace = None;
        trimmed.message = truncate_utf8(&trimmed.message, 1024).to_owned();
        trimmed.parameters.clear();
        bytes = serde_json::to_vec(&trimmed).map_err(io_error)?;
        if bytes.len() > max_report_bytes {
            return Err(invalid(
                "the crash report exceeds the configured report size limit",
            ));
        }
    }
    let final_path = directory.join(format!("{}{REPORT_SUFFIX}", report.id));
    let temporary = directory.join(format!(".{}.crash.tmp", report.id));
    {
        let mut file = File::create(&temporary).map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
    }
    fs::rename(&temporary, &final_path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        io_error(error)
    })?;
    Ok(final_path)
}

/// Delete the oldest reports until at most `max_reports` remain.
fn prune_reports(directory: &Path, max_reports: usize) -> Result<usize> {
    let mut names = report_file_names(directory)?;
    if names.len() <= max_reports {
        return Ok(0);
    }
    names.sort();
    let excess = names.len() - max_reports;
    let mut removed = 0;
    for name in names.into_iter().take(excess) {
        if fs::remove_file(directory.join(name)).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Parse retained reports oldest-first, skipping unreadable or oversized files.
fn read_reports(
    directory: &Path,
    max_reports: usize,
    max_report_bytes: usize,
) -> Result<Vec<CrashReport>> {
    let mut names = report_file_names(directory)?;
    names.sort();
    if names.len() > max_reports {
        names.drain(..names.len() - max_reports);
    }
    let mut reports = Vec::with_capacity(names.len());
    for name in names {
        let path = directory.join(&name);
        let Ok(file) = File::open(&path) else {
            continue;
        };
        let mut bytes = Vec::new();
        if file
            .take(max_report_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes.len() > max_report_bytes
        {
            continue;
        }
        if let Ok(report) = serde_json::from_slice::<CrashReport>(&bytes) {
            reports.push(report);
        }
    }
    Ok(reports)
}

fn report_file_names(directory: &Path) -> Result<Vec<String>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.ends_with(REPORT_SUFFIX) || names.len() >= MAX_CRASH_REPORTS * 4 {
            continue;
        }
        names.push(name);
    }
    Ok(names)
}

/// Promote non-empty signal-handler scratch files written by an earlier process run.
///
/// A crashed process leaves a populated `*.crash.partial`; a healthy one leaves an empty file.
/// Empty files belonging to a process that is no longer alive are removed.
fn promote_partial_reports(directory: &Path) -> Result<usize> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(io_error(error)),
    };
    let mut promoted = 0;
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some(stem) = name.strip_suffix(PARTIAL_SUFFIX) else {
            continue;
        };
        let path = directory.join(&name);
        let empty = entry
            .metadata()
            .map(|metadata| metadata.len() == 0)
            .unwrap_or(true);
        if empty {
            if !process_is_alive(partial_process_id(stem)) {
                let _ = fs::remove_file(&path);
            }
            continue;
        }
        if fs::rename(&path, directory.join(format!("{stem}{REPORT_SUFFIX}"))).is_ok() {
            promoted += 1;
        }
    }
    Ok(promoted)
}

fn partial_process_id(stem: &str) -> Option<u32> {
    stem.split('-').nth(1).and_then(|value| value.parse().ok())
}

#[cfg(unix)]
fn process_is_alive(process_id: Option<u32>) -> bool {
    let Some(process_id) = process_id else {
        return false;
    };
    if process_id == std::process::id() {
        return true;
    }
    // SAFETY: `kill` with signal 0 performs an existence and permission check only.
    unsafe { libc::kill(process_id as libc::pid_t, 0) == 0 }
}

#[cfg(not(unix))]
fn process_is_alive(process_id: Option<u32>) -> bool {
    process_id == Some(std::process::id())
}

// ---------------------------------------------------------------------------
// Panic hook
// ---------------------------------------------------------------------------

fn install_panic_hook() {
    static HOOK: OnceLock<()> = OnceLock::new();
    HOOK.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(reporter) = CrashReporter::current() {
                let _ = reporter.record_panic(info);
            }
            previous(info);
        }));
    });
}

fn panic_message(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "the process panicked with a non-string payload".to_owned()
    }
}

fn capture_backtrace(policy: BacktracePolicy) -> Option<String> {
    use std::backtrace::{Backtrace, BacktraceStatus};

    let backtrace = match policy {
        BacktracePolicy::Disabled => return None,
        BacktracePolicy::Environment => Backtrace::capture(),
        BacktracePolicy::Always => Backtrace::force_capture(),
    };
    if backtrace.status() != BacktraceStatus::Captured {
        return None;
    }
    let text = backtrace.to_string();
    Some(truncate_utf8(&text, MAX_CRASH_BACKTRACE_BYTES).to_owned())
}

// ---------------------------------------------------------------------------
// Native fatal-signal path
// ---------------------------------------------------------------------------

/// Pre-rendered pieces of a native fatal-fault report.
///
/// The handler writes `head`, the timestamp digits, `signal_prefix`, the signal number,
/// `name_prefix`, a static signal name, `address_prefix`, hexadecimal digits, `thread_prefix`,
/// hexadecimal digits, and finally `tail`. No allocation, locking, or formatting machinery is
/// involved, so every operation is async-signal-safe.
struct SignalTemplate {
    head: Box<[u8]>,
    signal_prefix: &'static [u8],
    name_prefix: &'static [u8],
    address_prefix: &'static [u8],
    thread_prefix: &'static [u8],
    tail: &'static [u8],
}

impl SignalTemplate {
    fn new(reporter: &CrashReporter, id: &str) -> Self {
        let system = crate::SystemInfo::current();
        let parameters = reporter.extra_parameters();
        let mut head = String::with_capacity(512);
        head.push_str("{\"schemaVersion\":");
        head.push_str(&CRASH_REPORT_SCHEMA_VERSION.to_string());
        push_field(&mut head, "id", id);
        push_field(&mut head, "kind", CrashKind::Signal.as_str());
        push_field(&mut head, "appName", &reporter.0.app_name);
        push_field(&mut head, "appVersion", &reporter.0.app_version);
        push_field(&mut head, "appIdentifier", &reporter.0.app_identifier);
        push_field(&mut head, "operatingSystem", system.name());
        if let Some(version) = system.version() {
            push_field(&mut head, "operatingSystemVersion", version);
        }
        push_field(&mut head, "architecture", system.architecture());
        head.push_str(",\"processId\":");
        head.push_str(&std::process::id().to_string());
        push_field(
            &mut head,
            "message",
            "the process terminated on a fatal native fault",
        );
        head.push_str(",\"parameters\":{");
        for (index, (key, value)) in parameters.iter().enumerate() {
            if index > 0 {
                head.push(',');
            }
            head.push('"');
            head.push_str(&json_escape(key));
            head.push_str("\":\"");
            head.push_str(&json_escape(value));
            head.push('"');
        }
        head.push_str("},\"timestamp\":\"");
        Self {
            head: head.into_bytes().into_boxed_slice(),
            signal_prefix: b"\",\"signal\":",
            name_prefix: b",\"signalName\":\"",
            address_prefix: b"\",\"faultAddress\":\"0x",
            thread_prefix: b"\",\"thread\":\"0x",
            tail: b"\"}",
        }
    }
}

fn push_field(buffer: &mut String, key: &str, value: &str) {
    buffer.push_str(",\"");
    buffer.push_str(key);
    buffer.push_str("\":\"");
    buffer.push_str(&json_escape(value));
    buffer.push('"');
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(unix)]
mod native {
    use super::{CrashReporter, PARTIAL_SUFFIX, SignalTemplate, report_id, unix_millis};
    use std::{
        os::fd::{IntoRawFd, RawFd},
        sync::atomic::{AtomicBool, AtomicPtr, Ordering},
    };

    /// Fatal faults QuickGUI reports. `SIGABRT` is included so aborts from the runtime and from
    /// double panics are recorded like hardware faults.
    pub(super) const FATAL_SIGNALS: [libc::c_int; 5] = [
        libc::SIGSEGV,
        libc::SIGBUS,
        libc::SIGILL,
        libc::SIGFPE,
        libc::SIGABRT,
    ];

    pub(super) fn signal_name(signal: libc::c_int) -> &'static str {
        match signal {
            libc::SIGSEGV => "SIGSEGV",
            libc::SIGBUS => "SIGBUS",
            libc::SIGILL => "SIGILL",
            libc::SIGFPE => "SIGFPE",
            libc::SIGABRT => "SIGABRT",
            _ => "SIGNAL",
        }
    }

    struct SignalState {
        descriptor: RawFd,
        template: SignalTemplate,
        written: AtomicBool,
    }

    static STATE: AtomicPtr<SignalState> = AtomicPtr::new(std::ptr::null_mut());

    pub(super) fn install(reporter: &CrashReporter) {
        if !STATE.load(Ordering::Acquire).is_null() {
            return;
        }
        let id = report_id(unix_millis(), std::process::id(), 0);
        let path = reporter.directory().join(format!("{id}{PARTIAL_SUFFIX}"));
        let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
        else {
            return;
        };
        let state = Box::into_raw(Box::new(SignalState {
            descriptor: file.into_raw_fd(),
            template: SignalTemplate::new(reporter, &id),
            written: AtomicBool::new(false),
        }));
        if STATE
            .compare_exchange(
                std::ptr::null_mut(),
                state,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            // SAFETY: `state` was just created here and never published.
            drop(unsafe { Box::from_raw(state) });
            return;
        }
        // SAFETY: `action` is fully initialized before `sigaction` reads it, and the handler only
        // performs async-signal-safe work. `SA_RESETHAND` restores the default disposition so the
        // re-raise below terminates the process with the original fault.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = handle as *const () as usize;
            libc::sigemptyset(&raw mut action.sa_mask);
            action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK | libc::SA_RESETHAND;
            for signal in FATAL_SIGNALS {
                libc::sigaction(signal, &raw const action, std::ptr::null_mut());
            }
        }
    }

    extern "C" fn handle(
        signal: libc::c_int,
        info: *mut libc::siginfo_t,
        _context: *mut libc::c_void,
    ) {
        let state = STATE.load(Ordering::Acquire);
        if !state.is_null() {
            // SAFETY: the pointer was published once by `install` and is never freed or replaced.
            let state = unsafe { &*state };
            if !state.written.swap(true, Ordering::AcqRel) {
                let address = if info.is_null() {
                    0
                } else {
                    // SAFETY: the kernel passes a valid `siginfo_t` for `SA_SIGINFO` handlers.
                    unsafe { (*info).si_addr() as usize }
                };
                // SAFETY: the thread identifier is read as an opaque integer only.
                let thread = unsafe { libc::pthread_self() as usize };
                write_report(state, signal, address, thread);
            }
        }
        // SAFETY: `SA_RESETHAND` already restored the default disposition for this signal.
        unsafe {
            libc::raise(signal);
        }
    }

    fn write_report(state: &SignalState, signal: libc::c_int, address: usize, thread: usize) {
        let mut scratch = [0_u8; 32];
        write_bytes(state.descriptor, &state.template.head);
        // SAFETY: `time` is on the POSIX async-signal-safe list.
        let seconds = unsafe { libc::time(std::ptr::null_mut()) } as i64;
        let length = super::format_rfc3339_into(seconds, &mut scratch);
        write_bytes(state.descriptor, &scratch[..length]);
        write_bytes(state.descriptor, state.template.signal_prefix);
        let length = format_decimal(signal as i64, &mut scratch);
        write_bytes(state.descriptor, &scratch[..length]);
        write_bytes(state.descriptor, state.template.name_prefix);
        write_bytes(state.descriptor, signal_name(signal).as_bytes());
        write_bytes(state.descriptor, state.template.address_prefix);
        let length = format_hex(address, &mut scratch);
        write_bytes(state.descriptor, &scratch[..length]);
        write_bytes(state.descriptor, state.template.thread_prefix);
        let length = format_hex(thread, &mut scratch);
        write_bytes(state.descriptor, &scratch[..length]);
        write_bytes(state.descriptor, state.template.tail);
        // SAFETY: `fsync` is a plain syscall wrapper.
        unsafe {
            libc::fsync(state.descriptor);
        }
    }

    fn write_bytes(descriptor: RawFd, bytes: &[u8]) {
        let mut offset = 0;
        while offset < bytes.len() {
            // SAFETY: the slice range is in bounds and `write` is async-signal-safe.
            let written = unsafe {
                libc::write(
                    descriptor,
                    bytes[offset..].as_ptr().cast(),
                    bytes.len() - offset,
                )
            };
            if written <= 0 {
                return;
            }
            offset += written as usize;
        }
    }

    fn format_decimal(value: i64, out: &mut [u8; 32]) -> usize {
        let negative = value < 0;
        let mut magnitude = value.unsigned_abs();
        let mut digits = [0_u8; 20];
        let mut count = 0;
        loop {
            digits[count] = b'0' + (magnitude % 10) as u8;
            magnitude /= 10;
            count += 1;
            if magnitude == 0 {
                break;
            }
        }
        let mut length = 0;
        if negative {
            out[0] = b'-';
            length = 1;
        }
        for index in (0..count).rev() {
            out[length] = digits[index];
            length += 1;
        }
        length
    }

    fn format_hex(value: usize, out: &mut [u8; 32]) -> usize {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut buffer = [0_u8; 16];
        let mut magnitude = value;
        let mut count = 0;
        loop {
            buffer[count] = DIGITS[magnitude % 16];
            magnitude /= 16;
            count += 1;
            if magnitude == 0 {
                break;
            }
        }
        for index in 0..count {
            out[index] = buffer[count - 1 - index];
        }
        count
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn formats_decimal_and_hexadecimal_without_allocating() {
            let mut scratch = [0_u8; 32];
            let length = format_decimal(-11, &mut scratch);
            assert_eq!(&scratch[..length], b"-11");
            let length = format_decimal(0, &mut scratch);
            assert_eq!(&scratch[..length], b"0");
            let length = format_hex(0x7f_ab_cd, &mut scratch);
            assert_eq!(&scratch[..length], b"7fabcd");
            let length = format_hex(0, &mut scratch);
            assert_eq!(&scratch[..length], b"0");
        }

        #[test]
        fn names_every_reported_fatal_signal() {
            for signal in FATAL_SIGNALS {
                assert_ne!(signal_name(signal), "SIGNAL");
            }
        }
    }
}

#[cfg(windows)]
mod native {
    use super::{CrashReporter, PARTIAL_SUFFIX, SignalTemplate, report_id, unix_millis};
    use std::{
        fs::File,
        io::Write,
        sync::atomic::{AtomicBool, AtomicPtr, Ordering},
    };
    use windows_sys::Win32::{
        Foundation::EXCEPTION_ACCESS_VIOLATION,
        System::Diagnostics::Debug::{EXCEPTION_POINTERS, SetUnhandledExceptionFilter},
    };

    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

    struct SignalState {
        file: std::sync::Mutex<File>,
        template: SignalTemplate,
        written: AtomicBool,
    }

    static STATE: AtomicPtr<SignalState> = AtomicPtr::new(std::ptr::null_mut());

    pub(super) fn install(reporter: &CrashReporter) {
        if !STATE.load(Ordering::Acquire).is_null() {
            return;
        }
        let id = report_id(unix_millis(), std::process::id(), 0);
        let path = reporter.directory().join(format!("{id}{PARTIAL_SUFFIX}"));
        let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
        else {
            return;
        };
        let state = Box::into_raw(Box::new(SignalState {
            file: std::sync::Mutex::new(file),
            template: SignalTemplate::new(reporter, &id),
            written: AtomicBool::new(false),
        }));
        if STATE
            .compare_exchange(
                std::ptr::null_mut(),
                state,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            // SAFETY: `state` was just created here and never published.
            drop(unsafe { Box::from_raw(state) });
            return;
        }
        // SAFETY: the filter pointer stays valid for the life of the process.
        unsafe {
            SetUnhandledExceptionFilter(Some(handle));
        }
    }

    unsafe extern "system" fn handle(exception: *const EXCEPTION_POINTERS) -> i32 {
        let state = STATE.load(Ordering::Acquire);
        if !state.is_null() {
            // SAFETY: the pointer was published once by `install` and is never freed.
            let state = unsafe { &*state };
            if !state.written.swap(true, Ordering::AcqRel) {
                let (code, address) = if exception.is_null() {
                    (0_u32, 0_usize)
                } else {
                    // SAFETY: Windows passes a valid record to an unhandled-exception filter.
                    unsafe {
                        let record = (*exception).ExceptionRecord;
                        if record.is_null() {
                            (0, 0)
                        } else {
                            (
                                (*record).ExceptionCode as u32,
                                (*record).ExceptionAddress as usize,
                            )
                        }
                    }
                };
                write_report(state, code, address);
            }
        }
        EXCEPTION_CONTINUE_SEARCH
    }

    fn write_report(state: &SignalState, code: u32, address: usize) {
        let name = if code == EXCEPTION_ACCESS_VIOLATION as u32 {
            "EXCEPTION_ACCESS_VIOLATION"
        } else {
            "EXCEPTION"
        };
        let Ok(mut file) = state.file.lock() else {
            return;
        };
        let mut bytes = Vec::with_capacity(state.template.head.len() + 128);
        bytes.extend_from_slice(&state.template.head);
        bytes.extend_from_slice(super::format_rfc3339(super::unix_seconds()).as_bytes());
        bytes.extend_from_slice(state.template.signal_prefix);
        bytes.extend_from_slice(format!("{}", code as i64).as_bytes());
        bytes.extend_from_slice(state.template.name_prefix);
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(state.template.address_prefix);
        bytes.extend_from_slice(format!("{address:x}").as_bytes());
        bytes.extend_from_slice(state.template.thread_prefix);
        bytes.extend_from_slice(format!("{:x}", std::process::id()).as_bytes());
        bytes.extend_from_slice(state.template.tail);
        let _ = file.write_all(&bytes);
        let _ = file.flush();
    }
}

#[cfg(any(unix, windows))]
fn install_signal_handler(reporter: &CrashReporter) {
    native::install(reporter);
}

#[cfg(not(any(unix, windows)))]
fn install_signal_handler(_reporter: &CrashReporter) {}

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

fn validate_parameter(key: String, value: String) -> Result<(String, String)> {
    if key.is_empty() || key.len() > MAX_CRASH_PARAMETER_KEY_BYTES || key.contains('\0') {
        return Err(invalid(format!(
            "a crash parameter key must be nonempty, NUL-free, and at most {MAX_CRASH_PARAMETER_KEY_BYTES} UTF-8 bytes"
        )));
    }
    if value.len() > MAX_CRASH_PARAMETER_VALUE_BYTES || value.contains('\0') {
        return Err(invalid(format!(
            "a crash parameter value must be NUL-free and at most {MAX_CRASH_PARAMETER_VALUE_BYTES} UTF-8 bytes"
        )));
    }
    Ok((key, value))
}

fn validate_https_url(value: &str, name: &str) -> Result<()> {
    if value.len() > 16 * 1024 || value.contains('\0') || !value.starts_with("https://") {
        return Err(invalid(format!("{name} must be a bounded HTTPS URL")));
    }
    Ok(())
}

fn https_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder().https_only(true).build();
    ureq::Agent::new_with_config(config)
}

fn truncate_utf8(value: &str, maximum: usize) -> &str {
    if value.len() <= maximum {
        return value;
    }
    let mut end = maximum;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix timestamp, using only integer arithmetic.
fn format_rfc3339(seconds: i64) -> String {
    let mut buffer = [0_u8; 32];
    let length = format_rfc3339_into(seconds, &mut buffer);
    String::from_utf8_lossy(&buffer[..length]).into_owned()
}

/// Allocation-free RFC 3339 rendering used by both the panic and the fatal-fault paths.
fn format_rfc3339_into(seconds: i64, out: &mut [u8; 32]) -> usize {
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;

    let mut index = 0;
    write_padded(out, &mut index, year.clamp(0, 9_999) as u64, 4);
    out[index] = b'-';
    index += 1;
    write_padded(out, &mut index, u64::from(month), 2);
    out[index] = b'-';
    index += 1;
    write_padded(out, &mut index, u64::from(day), 2);
    out[index] = b'T';
    index += 1;
    write_padded(out, &mut index, hour as u64, 2);
    out[index] = b':';
    index += 1;
    write_padded(out, &mut index, minute as u64, 2);
    out[index] = b':';
    index += 1;
    write_padded(out, &mut index, second as u64, 2);
    out[index] = b'Z';
    index += 1;
    index
}

fn write_padded(out: &mut [u8; 32], index: &mut usize, value: u64, width: usize) {
    let mut digits = [0_u8; 20];
    let mut count = 0;
    let mut magnitude = value;
    loop {
        digits[count] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        count += 1;
        if magnitude == 0 {
            break;
        }
    }
    for _ in count..width {
        out[*index] = b'0';
        *index += 1;
    }
    for position in (0..count).rev() {
        out[*index] = digits[position];
        *index += 1;
    }
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to a proleptic Gregorian date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

fn io_error(error: impl ToString) -> SystemIntegrationError {
    SystemIntegrationError::Platform(Arc::from(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "quickgui-crash-{label}-{}-{}",
            std::process::id(),
            unix_millis()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn reporter(directory: &Path) -> CrashReporter {
        let app = AppInfo::new("Crash Fixture", "1.2.3", "com.example.crash").unwrap();
        CrashReporter::detached(
            CrashReporterOptions::new(app)
                .directory(directory)
                .capture_signals(false)
                .max_reports(3)
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn formats_rfc3339_timestamps_without_a_date_library() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(format_rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn writes_and_parses_a_bounded_report() {
        let directory = temporary_directory("roundtrip");
        let reporter = reporter(&directory);
        reporter.add_extra_parameter("channel", "beta").unwrap();
        let mut report = reporter.build_report(CrashKind::Panic, "boom");
        report.location = Some(CrashLocation {
            file: "src/app.rs".to_owned(),
            line: 12,
            column: 3,
        });
        let path = reporter.capture_report(&report).unwrap();
        assert!(path.exists());

        let parsed = reporter.last_crash_report().unwrap().unwrap();
        assert_eq!(parsed.kind, CrashKind::Panic);
        assert_eq!(parsed.message, "boom");
        assert_eq!(parsed.app_name, "Crash Fixture");
        assert_eq!(parsed.app_version, "1.2.3");
        assert_eq!(parsed.app_identifier, "com.example.crash");
        assert_eq!(
            parsed.parameters.get("channel").map(String::as_str),
            Some("beta")
        );
        assert_eq!(parsed.location.unwrap().line, 12);
        assert_eq!(parsed.schema_version, CRASH_REPORT_SCHEMA_VERSION);
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn retains_only_the_newest_reports() {
        let directory = temporary_directory("retention");
        let reporter = reporter(&directory);
        for index in 0..8 {
            let report = reporter.build_report(CrashKind::Panic, format!("crash {index}"));
            reporter.capture_report(&report).unwrap();
        }
        let reports = reporter.pending_reports().unwrap();
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].message, "crash 5");
        assert_eq!(reports[2].message, "crash 7");
        assert_eq!(report_file_names(&directory).unwrap().len(), 3);
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn deletes_reports_by_identifier() {
        let directory = temporary_directory("delete");
        let reporter = reporter(&directory);
        let report = reporter.build_report(CrashKind::Panic, "boom");
        reporter.capture_report(&report).unwrap();
        assert!(reporter.delete_report(&report.id).unwrap());
        assert!(!reporter.delete_report(&report.id).unwrap());
        assert!(reporter.pending_reports().unwrap().is_empty());
        assert!(reporter.delete_report("../escape").is_err());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn drops_oversized_report_bodies_before_writing() {
        let directory = temporary_directory("oversize");
        let app = AppInfo::new("Crash Fixture", "1.0.0", "com.example.crash").unwrap();
        let reporter = CrashReporter::detached(
            CrashReporterOptions::new(app)
                .directory(&directory)
                .capture_signals(false)
                .max_report_bytes(2048)
                .unwrap(),
        )
        .unwrap();
        let mut report = reporter.build_report(CrashKind::Panic, "x".repeat(4096));
        report.backtrace = Some("frame\n".repeat(1024));
        reporter.capture_report(&report).unwrap();
        let stored = reporter.last_crash_report().unwrap().unwrap();
        assert!(stored.backtrace.is_none());
        assert_eq!(stored.message.len(), 1024);
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn bounds_extra_parameters() {
        let directory = temporary_directory("parameters");
        let reporter = reporter(&directory);
        for index in 0..MAX_CRASH_EXTRA_PARAMETERS {
            reporter
                .add_extra_parameter(format!("key{index}"), "value")
                .unwrap();
        }
        assert!(reporter.add_extra_parameter("overflow", "value").is_err());
        assert!(reporter.add_extra_parameter("key0", "replacement").is_ok());
        assert!(reporter.add_extra_parameter("", "value").is_err());
        assert!(
            reporter
                .add_extra_parameter("key", "v".repeat(MAX_CRASH_PARAMETER_VALUE_BYTES + 1))
                .is_err()
        );
        assert!(reporter.remove_extra_parameter("key0"));
        assert!(!reporter.remove_extra_parameter("key0"));
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn promotes_populated_partial_reports_and_removes_empty_ones() {
        let directory = temporary_directory("partial");
        let app = AppInfo::new("Crash Fixture", "1.0.0", "com.example.crash").unwrap();
        let template_reporter = CrashReporter::detached(
            CrashReporterOptions::new(app.clone())
                .directory(&directory)
                .capture_signals(false),
        )
        .unwrap();
        let id = report_id(1_700_000_000_000, 999_999, 0);
        let template = SignalTemplate::new(&template_reporter, &id);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&template.head);
        bytes.extend_from_slice(format_rfc3339(1_700_000_000).as_bytes());
        bytes.extend_from_slice(template.signal_prefix);
        bytes.extend_from_slice(b"11");
        bytes.extend_from_slice(template.name_prefix);
        bytes.extend_from_slice(b"SIGSEGV");
        bytes.extend_from_slice(template.address_prefix);
        bytes.extend_from_slice(b"deadbeef");
        bytes.extend_from_slice(template.thread_prefix);
        bytes.extend_from_slice(b"1f");
        bytes.extend_from_slice(template.tail);
        fs::write(directory.join(format!("{id}{PARTIAL_SUFFIX}")), &bytes).unwrap();
        let empty = report_id(1_700_000_000_001, 999_998, 0);
        fs::write(directory.join(format!("{empty}{PARTIAL_SUFFIX}")), b"").unwrap();

        assert_eq!(promote_partial_reports(&directory).unwrap(), 1);
        let reports = read_reports(&directory, 16, DEFAULT_MAX_CRASH_REPORT_BYTES).unwrap();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert_eq!(report.kind, CrashKind::Signal);
        assert_eq!(report.signal, Some(11));
        assert_eq!(report.signal_name.as_deref(), Some("SIGSEGV"));
        assert_eq!(report.fault_address.as_deref(), Some("0xdeadbeef"));
        assert_eq!(report.thread.as_deref(), Some("0x1f"));
        assert_eq!(report.timestamp, "2023-11-14T22:13:20Z");
        assert_eq!(report.app_identifier, "com.example.crash");
        assert!(!directory.join(format!("{empty}{PARTIAL_SUFFIX}")).exists());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn escapes_reserved_json_characters_in_the_signal_template() {
        let directory = temporary_directory("escape");
        let app = AppInfo::new("Quote \" App", "1.0.0", "com.example.crash").unwrap();
        let reporter = CrashReporter::detached(
            CrashReporterOptions::new(app)
                .directory(&directory)
                .capture_signals(false)
                .extra_parameter("note", "line\nbreak")
                .unwrap(),
        )
        .unwrap();
        let template = SignalTemplate::new(&reporter, "1-2-3");
        let head = String::from_utf8(template.head.to_vec()).unwrap();
        assert!(head.contains("Quote \\\" App"));
        assert!(head.contains("line\\nbreak"));
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn rejects_plain_http_upload_endpoints_before_any_request() {
        let directory = temporary_directory("upload");
        let reporter = reporter(&directory);
        assert!(reporter.upload_pending("http://example.com/crash").is_err());
        assert_eq!(
            reporter
                .upload_pending("https://example.invalid/crash")
                .unwrap(),
            UploadSummary::default(),
        );
        let report = reporter.build_report(CrashKind::Panic, "boom");
        let body = CrashReporter::upload_body(&report).unwrap();
        let decoded: CrashReport = serde_json::from_slice(&body).unwrap();
        assert_eq!(decoded, report);
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn watchdog_reports_one_hang_and_resets_after_a_heartbeat() {
        assert!(!watchdog_should_report(
            100,
            90,
            Duration::from_millis(50),
            false
        ));
        assert!(watchdog_should_report(
            200,
            90,
            Duration::from_millis(50),
            false
        ));
        assert!(!watchdog_should_report(
            200,
            90,
            Duration::from_millis(50),
            true
        ));

        let directory = temporary_directory("watchdog");
        let reporter = reporter(&directory);
        let watchdog = Watchdog::start(
            &reporter,
            WatchdogOptions::default()
                .interval(Duration::from_millis(10))
                .unwrap()
                .hang_threshold(Duration::from_millis(20))
                .unwrap(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && reporter.pending_reports().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(10));
        }
        watchdog.stop();
        let reports = reporter.pending_reports().unwrap();
        assert_eq!(reports.len(), 1, "the watchdog reports a hang exactly once");
        assert_eq!(reports[0].kind, CrashKind::Hang);
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn rejects_out_of_range_options() {
        let app = AppInfo::new("Crash Fixture", "1.0.0", "com.example.crash").unwrap();
        assert!(
            CrashReporterOptions::new(app.clone())
                .max_reports(0)
                .is_err()
        );
        assert!(
            CrashReporterOptions::new(app.clone())
                .max_reports(MAX_CRASH_REPORTS + 1)
                .is_err()
        );
        assert!(
            CrashReporterOptions::new(app.clone())
                .max_report_bytes(16)
                .is_err()
        );
        assert!(
            CrashReporterOptions::new(app.clone())
                .upload_endpoint("http://example.com")
                .is_err()
        );
        assert!(
            WatchdogOptions::default()
                .interval(Duration::from_millis(1))
                .is_err()
        );
        assert!(
            WatchdogOptions::default()
                .hang_threshold(Duration::from_secs(600))
                .is_err()
        );
        assert!(
            CrashReporter::detached(CrashReporterOptions::new(app).directory("relative/crashes"),)
                .is_err()
        );
    }
}
