//! Background system integrations and CPU-only services behind the C ABI.
//!
//! [`invoke`] runs off the application thread and reports through one `invoke` event; [`call`]
//! answers synchronously because nothing here touches the application runtime. Both speak JSON so
//! the compiled application decodes typed records without a second marshalling layer.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use quickgui::{
    AutoStart, AutoStartMode, AutoStartOptions, BacktracePolicy, CpuUsage, CpuUsageSampler,
    CrashReport, CrashReporter, CrashReporterOptions, PermissionManager,
    PowerMonitor as CorePowerMonitor, ProcessMetrics, ProtocolRegistration,
    ProtocolRegistrationOptions, SecureStorage, SystemIntegrationError, SystemMemory,
    UploadSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::router::{NativeRouteDefinition, NativeRouter};
use crate::system::{
    NativePowerAssertion, NativePowerState, parse_permission_kind, permission_status_name,
};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAutoStartOptions {
    pub app_name: String,
    pub executable: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub mode: Option<String>,
    pub bundle_identifier: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProtocolRegistrationOptions {
    pub scheme: String,
    pub app_name: String,
    pub app_id: String,
    pub executable: Option<String>,
    pub arguments: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Crash reporting
// ---------------------------------------------------------------------------

#[derive(Clone, Deserialize, Serialize)]
pub struct NativeCrashParameter {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrashReporterOptions {
    pub app_name: String,
    pub app_version: String,
    pub app_identifier: String,
    pub directory: Option<String>,
    pub max_reports: Option<u32>,
    pub max_report_bytes: Option<u32>,
    pub parameters: Option<Vec<NativeCrashParameter>>,
    pub upload_endpoint: Option<String>,
    pub backtrace: Option<String>,
    pub capture_signals: Option<bool>,
}

#[derive(Clone, Serialize)]
pub struct NativeCrashLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrashReport {
    pub schema_version: u32,
    pub id: String,
    pub kind: String,
    pub timestamp: String,
    pub app_name: String,
    pub app_version: String,
    pub app_identifier: String,
    pub operating_system: String,
    pub operating_system_version: Option<String>,
    pub architecture: String,
    pub process_id: u32,
    pub thread: Option<String>,
    pub message: String,
    pub location: Option<NativeCrashLocation>,
    pub backtrace: Option<String>,
    pub signal: Option<i32>,
    pub signal_name: Option<String>,
    pub fault_address: Option<String>,
    pub parameters: Vec<NativeCrashParameter>,
}

#[derive(Clone, Serialize)]
pub struct NativeCrashUploadSummary {
    pub attempted: u32,
    pub uploaded: u32,
    pub failed: u32,
}

impl From<CrashReport> for NativeCrashReport {
    fn from(report: CrashReport) -> Self {
        Self {
            schema_version: report.schema_version,
            id: report.id,
            kind: report.kind.as_str().to_owned(),
            timestamp: report.timestamp,
            app_name: report.app_name,
            app_version: report.app_version,
            app_identifier: report.app_identifier,
            operating_system: report.operating_system,
            operating_system_version: report.operating_system_version,
            architecture: report.architecture,
            process_id: report.process_id,
            thread: report.thread,
            message: report.message,
            location: report.location.map(|location| NativeCrashLocation {
                file: location.file,
                line: location.line,
                column: location.column,
            }),
            backtrace: report.backtrace,
            signal: report.signal,
            signal_name: report.signal_name,
            fault_address: report.fault_address,
            parameters: report
                .parameters
                .into_iter()
                .map(|(key, value)| NativeCrashParameter { key, value })
                .collect(),
        }
    }
}

impl From<UploadSummary> for NativeCrashUploadSummary {
    fn from(summary: UploadSummary) -> Self {
        Self {
            attempted: summary.attempted as u32,
            uploaded: summary.uploaded as u32,
            failed: summary.failed as u32,
        }
    }
}

fn crash_reporter_options(options: NativeCrashReporterOptions) -> Result<CrashReporterOptions> {
    let app = quickgui::AppInfo::new(
        options.app_name,
        options.app_version,
        options.app_identifier,
    )
    .map_err(system_error)?;
    let mut native = CrashReporterOptions::new(app);
    if let Some(directory) = options.directory {
        native = native.directory(PathBuf::from(directory));
    }
    if let Some(maximum) = options.max_reports {
        native = native.max_reports(maximum as usize).map_err(system_error)?;
    }
    if let Some(maximum) = options.max_report_bytes {
        native = native
            .max_report_bytes(maximum as usize)
            .map_err(system_error)?;
    }
    for parameter in options.parameters.unwrap_or_default() {
        native = native
            .extra_parameter(parameter.key, parameter.value)
            .map_err(system_error)?;
    }
    if let Some(endpoint) = options.upload_endpoint {
        native = native.upload_endpoint(endpoint).map_err(system_error)?;
    }
    if let Some(policy) = options.backtrace {
        native = native.backtrace(crash_backtrace_policy(&policy)?);
    }
    if let Some(capture) = options.capture_signals {
        native = native.capture_signals(capture);
    }
    Ok(native)
}

fn crash_backtrace_policy(policy: &str) -> Result<BacktracePolicy> {
    match policy {
        "disabled" => Ok(BacktracePolicy::Disabled),
        "environment" => Ok(BacktracePolicy::Environment),
        "always" => Ok(BacktracePolicy::Always),
        value => Err(format!("unknown crash backtrace policy `{value}`")),
    }
}

fn installed_reporter() -> Result<CrashReporter> {
    CrashReporter::current().ok_or_else(|| {
        "the crash reporter has not been started; call CrashReporter.start first".to_owned()
    })
}

// ---------------------------------------------------------------------------
// Process and system metrics
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProcessMetrics {
    pub cpu_user_seconds: f64,
    pub cpu_system_seconds: f64,
    pub resident_bytes: f64,
    pub footprint_bytes: Option<f64>,
    pub virtual_bytes: f64,
    pub thread_count: Option<u32>,
    pub uptime_seconds: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSystemMemory {
    pub total_bytes: f64,
    pub available_bytes: f64,
    pub free_bytes: f64,
    pub used_bytes: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCpuUsage {
    pub percent: Option<f64>,
    pub interval_seconds: f64,
    pub cpu_seconds: f64,
    pub total_cpu_seconds: f64,
}

impl From<ProcessMetrics> for NativeProcessMetrics {
    fn from(metrics: ProcessMetrics) -> Self {
        Self {
            cpu_user_seconds: metrics.cpu_user.as_secs_f64(),
            cpu_system_seconds: metrics.cpu_system.as_secs_f64(),
            resident_bytes: metrics.resident_bytes as f64,
            footprint_bytes: metrics.footprint_bytes.map(|bytes| bytes as f64),
            virtual_bytes: metrics.virtual_bytes as f64,
            thread_count: metrics.thread_count,
            uptime_seconds: metrics.uptime.as_secs_f64(),
        }
    }
}

impl From<SystemMemory> for NativeSystemMemory {
    fn from(memory: SystemMemory) -> Self {
        Self {
            total_bytes: memory.total_bytes as f64,
            available_bytes: memory.available_bytes as f64,
            free_bytes: memory.free_bytes as f64,
            used_bytes: memory.used_bytes as f64,
        }
    }
}

impl From<CpuUsage> for NativeCpuUsage {
    fn from(usage: CpuUsage) -> Self {
        Self {
            percent: usage.percent,
            interval_seconds: usage.interval.as_secs_f64(),
            cpu_seconds: usage.cpu_time.as_secs_f64(),
            total_cpu_seconds: usage.total_cpu_time.as_secs_f64(),
        }
    }
}

fn auto_start(options: NativeAutoStartOptions) -> Result<AutoStart> {
    let mut native = AutoStartOptions::new(options.app_name).map_err(system_error)?;
    if let Some(executable) = options.executable {
        native.executable = PathBuf::from(executable);
    }
    native.arguments = options.arguments.unwrap_or_default();
    native.mode = auto_start_mode(options.mode.as_deref())?;
    native.bundle_identifier = options.bundle_identifier;
    AutoStart::new(native).map_err(system_error)
}

fn auto_start_mode(mode: Option<&str>) -> Result<AutoStartMode> {
    match mode.unwrap_or("native") {
        "native" => Ok(AutoStartMode::Native),
        "macos-launch-agent" => Ok(AutoStartMode::MacOsLaunchAgent),
        "macos-apple-script" => Ok(AutoStartMode::MacOsAppleScript),
        "linux-systemd" => Ok(AutoStartMode::LinuxSystemd),
        "windows-system" => Ok(AutoStartMode::WindowsSystem),
        mode => Err(format!("unknown native autostart mode `{mode}`")),
    }
}

fn protocol_registration(
    options: NativeProtocolRegistrationOptions,
) -> Result<ProtocolRegistration> {
    let mut native =
        ProtocolRegistrationOptions::new(options.app_name, options.app_id).map_err(system_error)?;
    if let Some(executable) = options.executable {
        native.executable = PathBuf::from(executable);
    }
    native.arguments = options.arguments.unwrap_or_default();
    ProtocolRegistration::new(options.scheme, native).map_err(system_error)
}

fn system_error(error: SystemIntegrationError) -> String {
    error.to_string()
}

fn params<T: for<'de> Deserialize<'de>>(json: &str, method: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|error| format!("invalid `{method}` parameters: {error}"))
}

fn to_json<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

#[derive(Deserialize)]
struct EndpointParams {
    endpoint: Option<String>,
}

#[derive(Deserialize)]
struct SecureStorageParams {
    service: String,
    account: String,
    #[serde(default, with = "crate::base64_bytes")]
    value: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct CrashParameterParams {
    key: String,
    value: Option<String>,
}

#[derive(Deserialize)]
struct IdParams {
    id: String,
}

#[derive(Deserialize)]
struct KindParams {
    kind: String,
}

#[derive(Deserialize)]
struct WatchFilesParams {
    id: u32,
    paths: Vec<PathBuf>,
}
static FILE_WATCHERS: std::sync::LazyLock<Mutex<HashMap<u32, quickgui::FileWatcher>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn clear_file_watchers() {
    FILE_WATCHERS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
}

/// Run one background integration. The result is one JSON value or an error message.
///
pub(crate) fn invoke(method: &str, json: &str, _request: u32) -> Result<Value> {
    match method {
        "watch-files" => {
            let WatchFilesParams { id, paths } = params(json, method)?;
            let mut watchers = FILE_WATCHERS
                .lock()
                .map_err(|_| "file watcher registry is unavailable")?;
            if watchers.len() >= 128 || watchers.contains_key(&id) {
                return Err("file watcher registration limit reached".into());
            }
            let watcher = quickgui::FileWatcher::new(&paths, move |event| {
                crate::publish_reply(
                    "file-watch",
                    id,
                    Ok(
                        json!({ "paths": event.paths, "rescan": event.rescan, "error": event.error }),
                    ),
                );
            })?;
            watchers.insert(id, watcher);
            Ok(Value::Null)
        }
        "unwatch-files" => {
            let HandleParams { id } = params(json, method)?;
            FILE_WATCHERS
                .lock()
                .map_err(|_| "file watcher registry is unavailable")?
                .remove(&id);
            Ok(Value::Null)
        }

        "enable-auto-start" => auto_start(params(json, method)?)?
            .enable()
            .map(|()| Value::Null)
            .map_err(system_error),
        "disable-auto-start" => auto_start(params(json, method)?)?
            .disable()
            .map(|()| Value::Null)
            .map_err(system_error),
        "is-auto-start-enabled" => auto_start(params(json, method)?)?
            .is_enabled()
            .map(Value::Bool)
            .map_err(system_error),
        "register-protocol" => protocol_registration(params(json, method)?)?
            .register()
            .map(|()| Value::Bool(true))
            .map_err(system_error),
        "unregister-protocol" => protocol_registration(params(json, method)?)?
            .unregister()
            .map(Value::Bool)
            .map_err(system_error),
        "is-protocol-registered" => protocol_registration(params(json, method)?)?
            .is_registered()
            .map(Value::Bool)
            .map_err(system_error),
        "set-secure-storage" => {
            let SecureStorageParams {
                service,
                account,
                value,
            } = params(json, method)?;
            let secret = value.ok_or_else(|| "set-secure-storage requires a value".to_owned())?;
            SecureStorage::set(&service, &account, &secret)
                .map(|()| Value::Bool(true))
                .map_err(system_error)
        }
        "get-secure-storage" => {
            let SecureStorageParams {
                service, account, ..
            } = params(json, method)?;
            SecureStorage::get(&service, &account)
                .map(|value| match value {
                    Some(bytes) => Value::String(STANDARD.encode(bytes)),
                    None => Value::Null,
                })
                .map_err(system_error)
        }
        "delete-secure-storage" => {
            let SecureStorageParams {
                service, account, ..
            } = params(json, method)?;
            SecureStorage::delete(&service, &account)
                .map(Value::Bool)
                .map_err(system_error)
        }
        "start-crash-reporter" => {
            let options: NativeCrashReporterOptions = params(json, method)?;
            if let Some(existing) = CrashReporter::current() {
                return Ok(Value::String(
                    existing.directory().to_string_lossy().into_owned(),
                ));
            }
            let reporter =
                CrashReporter::install(crash_reporter_options(options)?).map_err(system_error)?;
            Ok(Value::String(
                reporter.directory().to_string_lossy().into_owned(),
            ))
        }
        "get-last-crash-report" => {
            let reports: Vec<NativeCrashReport> = installed_reporter()?
                .last_crash_report()
                .map_err(system_error)?
                .into_iter()
                .map(Into::into)
                .collect();
            to_json(reports)
        }
        "get-pending-crash-reports" => {
            let reports: Vec<NativeCrashReport> = installed_reporter()?
                .pending_reports()
                .map_err(system_error)?
                .into_iter()
                .map(Into::into)
                .collect();
            to_json(reports)
        }
        "add-crash-extra-parameter" => {
            let CrashParameterParams { key, value } = params(json, method)?;
            let value =
                value.ok_or_else(|| "add-crash-extra-parameter requires a value".to_owned())?;
            installed_reporter()?
                .add_extra_parameter(key, value)
                .map(|()| Value::Bool(true))
                .map_err(system_error)
        }
        "remove-crash-extra-parameter" => {
            let CrashParameterParams { key, .. } = params(json, method)?;
            Ok(Value::Bool(
                installed_reporter()?.remove_extra_parameter(&key),
            ))
        }
        "delete-crash-report" => {
            let IdParams { id } = params(json, method)?;
            installed_reporter()?
                .delete_report(&id)
                .map(Value::Bool)
                .map_err(system_error)
        }
        "upload-pending-crash-reports" => {
            let EndpointParams { endpoint } = params(json, method)?;
            let reporter = installed_reporter()?;
            let endpoint = match endpoint {
                Some(endpoint) => endpoint,
                None => reporter
                    .upload_endpoint()
                    .map(str::to_owned)
                    .ok_or_else(|| "no crash upload endpoint is configured".to_owned())?,
            };
            reporter
                .upload_pending(&endpoint)
                .map_err(system_error)
                .and_then(|summary| to_json(NativeCrashUploadSummary::from(summary)))
        }
        "get-process-metrics" => ProcessMetrics::current()
            .map_err(system_error)
            .and_then(|metrics| to_json(NativeProcessMetrics::from(metrics))),
        "get-system-memory" => SystemMemory::current()
            .map_err(system_error)
            .and_then(|memory| to_json(NativeSystemMemory::from(memory))),
        "request-permission" => {
            let KindParams { kind } = params(json, method)?;
            let kind = parse_permission_kind(&kind)?;
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            PermissionManager::request(kind, move |result| {
                let _ = sender.send(result);
            })
            .map_err(|error| error.to_string())?;
            let status = receiver
                .recv()
                .map_err(|_| "the native permission request was cancelled".to_owned())?
                .map_err(|error| error.to_string())?;
            Ok(Value::String(permission_status_name(status)))
        }
        method => Err(format!("unknown native integration `{method}`")),
    }
}

// ---------------------------------------------------------------------------
// Synchronous, CPU-only services owned by the application thread
// ---------------------------------------------------------------------------

thread_local! {
    static ROUTERS: RefCell<HashMap<u32, NativeRouter>> = RefCell::new(HashMap::new());
    static POWER_ASSERTIONS: RefCell<HashMap<u32, NativePowerAssertion>> = RefCell::new(HashMap::new());
    static CPU_SAMPLERS: RefCell<HashMap<u32, Arc<Mutex<CpuUsageSampler>>>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: RefCell<u32> = const { RefCell::new(1) };
}

/// Most routers, power assertions, and samplers one application thread retains at once.
const MAX_HANDLES: usize = 4_096;

fn allocate_handle() -> Result<u32> {
    NEXT_HANDLE.with(|next| {
        let mut next = next.borrow_mut();
        let handle = *next;
        *next = next
            .checked_add(1)
            .ok_or_else(|| "native handle space exhausted".to_owned())?;
        Ok(handle)
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouterCreateParams {
    routes: Vec<NativeRouteDefinition>,
    initial_destination: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouterParams {
    id: u32,
    destination: Option<String>,
    end: Option<bool>,
    delta: Option<i32>,
}

#[derive(Deserialize)]
struct HandleParams {
    id: u32,
}

#[derive(Deserialize)]
struct PowerAssertionParams {
    kind: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdleStateParams {
    threshold_seconds: f64,
}

fn with_router<T>(id: u32, callback: impl FnOnce(&mut NativeRouter) -> Result<T>) -> Result<T> {
    ROUTERS.with(|routers| {
        let mut routers = routers.borrow_mut();
        let router = routers
            .get_mut(&id)
            .ok_or_else(|| format!("unknown native router {id}"))?;
        callback(router)
    })
}

/// Answer one synchronous service. Nothing here reaches the application runtime.
pub(crate) fn call(method: &str, json: &str) -> Result<Value> {
    match method {
        "protocol-version" => Ok(Value::from(crate::PROTOCOL_VERSION)),
        "is-application-packaged" => Ok(Value::Bool(quickgui::is_application_packaged())),
        "is-auto-start-supported" => Ok(Value::Bool(AutoStart::is_supported())),
        "supports-dynamic-protocol-registration" => Ok(Value::Bool(
            ProtocolRegistration::supports_dynamic_registration(),
        )),
        "is-secure-storage-supported" => Ok(Value::Bool(SecureStorage::is_supported())),
        "is-crash-reporter-started" => Ok(Value::Bool(CrashReporter::current().is_some())),
        "get-power-state" => CorePowerMonitor::snapshot()
            .map_err(|error| error.to_string())
            .and_then(|state| to_json(NativePowerState::from(state))),
        "get-system-idle-time" => CorePowerMonitor::system_idle_time()
            .map(|duration| Value::from(duration.as_secs_f64()))
            .map_err(|error| error.to_string()),
        "get-session-state" => CorePowerMonitor::session_state()
            .map(|state| {
                Value::String(
                    match state {
                        quickgui::SessionState::Active => "active",
                        quickgui::SessionState::Inactive => "inactive",
                        quickgui::SessionState::Locked => "locked",
                        quickgui::SessionState::Unknown => "unknown",
                    }
                    .to_owned(),
                )
            })
            .map_err(|error| error.to_string()),
        "get-system-idle-state" => {
            let IdleStateParams { threshold_seconds } = params(json, method)?;
            if !threshold_seconds.is_finite()
                || threshold_seconds <= 0.0
                || threshold_seconds > 366.0 * 24.0 * 60.0 * 60.0
            {
                return Err(
                    "the idle threshold must be a positive finite number of seconds".to_owned(),
                );
            }
            CorePowerMonitor::system_idle_state(std::time::Duration::from_secs_f64(
                threshold_seconds,
            ))
            .map(|state| {
                Value::String(
                    match state {
                        quickgui::IdleState::Active => "active",
                        quickgui::IdleState::Idle => "idle",
                        quickgui::IdleState::Locked => "locked",
                        quickgui::IdleState::Unknown => "unknown",
                    }
                    .to_owned(),
                )
            })
            .map_err(|error| error.to_string())
        }
        "get-permission-status" => {
            let KindParams { kind } = params(json, method)?;
            PermissionManager::status(parse_permission_kind(&kind)?)
                .map(|status| Value::String(permission_status_name(status)))
                .map_err(|error| error.to_string())
        }
        "power-assertion-acquire" => {
            let PowerAssertionParams { kind, reason } = params(json, method)?;
            let assertion = NativePowerAssertion::new(&kind, reason)?;
            POWER_ASSERTIONS.with(|assertions| {
                let mut assertions = assertions.borrow_mut();
                if assertions.len() >= MAX_HANDLES {
                    return Err("too many native power assertions are retained".to_owned());
                }
                let id = allocate_handle()?;
                assertions.insert(id, assertion);
                Ok(Value::from(id))
            })
        }
        "power-assertion-info" => {
            let HandleParams { id } = params(json, method)?;
            POWER_ASSERTIONS.with(|assertions| {
                let assertions = assertions.borrow();
                let assertion = assertions
                    .get(&id)
                    .ok_or_else(|| format!("unknown native power assertion {id}"))?;
                Ok(json!({
                    "kind": assertion.kind(),
                    "reason": assertion.reason(),
                    "active": assertion.active(),
                }))
            })
        }
        "power-assertion-release" => {
            let HandleParams { id } = params(json, method)?;
            POWER_ASSERTIONS.with(|assertions| {
                let mut assertions = assertions.borrow_mut();
                let assertion = assertions
                    .get_mut(&id)
                    .ok_or_else(|| format!("unknown native power assertion {id}"))?;
                let released = assertion.release()?;
                assertions.remove(&id);
                Ok(Value::Bool(released))
            })
        }
        "cpu-sampler-create" => CPU_SAMPLERS.with(|samplers| {
            let mut samplers = samplers.borrow_mut();
            if samplers.len() >= MAX_HANDLES {
                return Err("too many native CPU samplers are retained".to_owned());
            }
            let id = allocate_handle()?;
            samplers.insert(id, Arc::new(Mutex::new(CpuUsageSampler::new())));
            Ok(Value::from(id))
        }),
        "cpu-sampler-sample" => {
            let HandleParams { id } = params(json, method)?;
            let sampler = CPU_SAMPLERS.with(|samplers| {
                samplers
                    .borrow()
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| format!("unknown native CPU sampler {id}"))
            })?;
            let mut sampler = sampler
                .lock()
                .map_err(|_| "the CPU usage sampler is poisoned".to_owned())?;
            sampler
                .sample()
                .map_err(system_error)
                .and_then(|usage| to_json(NativeCpuUsage::from(usage)))
        }
        "cpu-sampler-release" => {
            let HandleParams { id } = params(json, method)?;
            Ok(Value::Bool(CPU_SAMPLERS.with(|samplers| {
                samplers.borrow_mut().remove(&id).is_some()
            })))
        }
        "router-create" => {
            let RouterCreateParams {
                routes,
                initial_destination,
            } = params(json, method)?;
            let router = NativeRouter::new(routes, initial_destination)?;
            ROUTERS.with(|routers| {
                let mut routers = routers.borrow_mut();
                if routers.len() >= MAX_HANDLES {
                    return Err("too many native routers are retained".to_owned());
                }
                let id = allocate_handle()?;
                routers.insert(id, router);
                Ok(Value::from(id))
            })
        }
        "router-release" => {
            let HandleParams { id } = params(json, method)?;
            Ok(Value::Bool(ROUTERS.with(|routers| {
                routers.borrow_mut().remove(&id).is_some()
            })))
        }
        "router-state" => {
            let RouterParams { id, .. } = params(json, method)?;
            with_router(id, |router| to_json(router.state()))
        }
        "router-resolve" => {
            let RouterParams {
                id, destination, ..
            } = params(json, method)?;
            let destination =
                destination.ok_or_else(|| "router-resolve requires a destination".to_owned())?;
            with_router(id, |router| router.resolve(&destination).and_then(to_json))
        }
        "router-is-active" => {
            let RouterParams {
                id,
                destination,
                end,
                ..
            } = params(json, method)?;
            let destination =
                destination.ok_or_else(|| "router-is-active requires a destination".to_owned())?;
            with_router(id, |router| {
                router
                    .is_active(&destination, end.unwrap_or(false))
                    .map(Value::Bool)
            })
        }
        "router-push" => {
            let RouterParams {
                id, destination, ..
            } = params(json, method)?;
            let destination =
                destination.ok_or_else(|| "router-push requires a destination".to_owned())?;
            with_router(id, |router| router.push(&destination).and_then(to_json))
        }
        "router-replace" => {
            let RouterParams {
                id, destination, ..
            } = params(json, method)?;
            let destination =
                destination.ok_or_else(|| "router-replace requires a destination".to_owned())?;
            with_router(id, |router| router.replace(&destination).and_then(to_json))
        }
        "router-go" => {
            let RouterParams { id, delta, .. } = params(json, method)?;
            with_router(id, |router| to_json(router.go(delta.unwrap_or(0))))
        }
        "router-back" => {
            let RouterParams { id, .. } = params(json, method)?;
            with_router(id, |router| to_json(router.back()))
        }
        "router-forward" => {
            let RouterParams { id, .. } = params(json, method)?;
            with_router(id, |router| to_json(router.forward()))
        }
        method => Err(format!("unknown native service `{method}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_autostart_modes() {
        assert_eq!(
            auto_start_mode(Some("linux-systemd")).unwrap(),
            AutoStartMode::LinuxSystemd
        );
        assert!(auto_start_mode(Some("unknown")).is_err());
    }

    #[test]
    fn parses_crash_backtrace_policies() {
        assert_eq!(
            crash_backtrace_policy("always").unwrap(),
            BacktracePolicy::Always
        );
        assert_eq!(
            crash_backtrace_policy("disabled").unwrap(),
            BacktracePolicy::Disabled
        );
        assert_eq!(
            crash_backtrace_policy("environment").unwrap(),
            BacktracePolicy::Environment
        );
        assert!(crash_backtrace_policy("verbose").is_err());
    }

    #[test]
    fn crash_reporter_options_bound_every_application_input() {
        let base = NativeCrashReporterOptions {
            app_name: "Demo".to_owned(),
            app_version: "1.0.0".to_owned(),
            app_identifier: "com.example.demo".to_owned(),
            directory: Some("/tmp/quickgui-crash-binding".to_owned()),
            max_reports: Some(4),
            max_report_bytes: Some(8192),
            parameters: Some(vec![NativeCrashParameter {
                key: "channel".to_owned(),
                value: "beta".to_owned(),
            }]),
            upload_endpoint: Some("https://crash.example.com/report".to_owned()),
            backtrace: Some("always".to_owned()),
            capture_signals: Some(false),
        };
        assert!(crash_reporter_options(base.clone()).is_ok());

        let mut invalid_endpoint = base.clone();
        invalid_endpoint.upload_endpoint = Some("http://crash.example.com".to_owned());
        assert!(crash_reporter_options(invalid_endpoint).is_err());

        let mut invalid_retention = base.clone();
        invalid_retention.max_reports = Some(0);
        assert!(crash_reporter_options(invalid_retention).is_err());

        let mut invalid_identity = base;
        invalid_identity.app_identifier = String::new();
        assert!(crash_reporter_options(invalid_identity).is_err());
    }

    #[test]
    fn synchronous_services_answer_routers_and_capabilities() {
        let id = call(
            "router-create",
            r#"{"routes":[{"id":"home","path":"/"},{"id":"project","path":"/projects/:id"}],"initialDestination":"/projects/quickgui"}"#,
        )
        .unwrap();
        let id = id.as_u64().unwrap();
        let state = call("router-state", &format!("{{\"id\":{id}}}")).unwrap();
        assert_eq!(state["matched"]["routeIds"][0], "project");
        assert_eq!(state["matched"]["params"][0]["value"], "quickgui");
        let pushed = call(
            "router-push",
            &format!("{{\"id\":{id},\"destination\":\"/\"}}"),
        )
        .unwrap();
        assert_eq!(pushed["canGoBack"], true);
        assert_eq!(
            call("router-release", &format!("{{\"id\":{id}}}")).unwrap(),
            true
        );
        assert!(call("router-state", &format!("{{\"id\":{id}}}")).is_err());
        assert_eq!(
            call("protocol-version", "").unwrap(),
            crate::PROTOCOL_VERSION
        );
        assert!(call("no-such-service", "").is_err());
        assert!(invoke("no-such-integration", "", 1).is_err());
    }
}
