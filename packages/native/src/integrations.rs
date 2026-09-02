use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use napi::{
    Env, Error, Result, Status, Task,
    bindgen_prelude::{AsyncTask, Buffer},
    threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use quickgui::{
    AutoStart, AutoStartMode, AutoStartOptions, AvailableUpdate, BacktracePolicy, CpuUsage,
    CpuUsageSampler, CrashReport, CrashReporter, CrashReporterOptions, InstalledUpdate,
    ProcessMetrics, ProtocolRegistration, ProtocolRegistrationOptions, SecureStorage,
    SystemIntegrationError, SystemMemory, UpdateClient, UpdateInstallDisposition,
    UpdateInstallOptions, UpdateProgress, UploadSummary, WindowsUpdateInstallMode,
    default_update_target as core_default_update_target,
};

#[derive(Clone)]
#[napi(object)]
pub struct NativeAutoStartOptions {
    pub app_name: String,
    pub executable: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub mode: Option<String>,
    pub bundle_identifier: Option<String>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeProtocolRegistrationOptions {
    pub scheme: String,
    pub app_name: String,
    pub app_id: String,
    pub executable: Option<String>,
    pub arguments: Option<Vec<String>>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeUpdateClientOptions {
    pub current_version: String,
    pub public_key: String,
    pub target: Option<String>,
    pub maximum_download_bytes: Option<u32>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeAvailableUpdate {
    pub version: String,
    pub current_version: String,
    pub target: String,
    pub url: String,
    pub signature: String,
    pub notes: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Clone, Default)]
#[napi(object)]
pub struct NativeUpdateInstallOptions {
    pub target_executable: Option<String>,
    pub retain_backup: Option<bool>,
    pub windows_mode: Option<String>,
    pub installer_arguments: Option<Vec<String>>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeInstalledUpdate {
    pub version: String,
    pub disposition: String,
    pub installed_path: String,
    pub backup_path: Option<String>,
    pub installer_process_id: Option<u32>,
    pub requires_application_exit: bool,
    pub relaunch_recommended: bool,
}

impl From<InstalledUpdate> for NativeInstalledUpdate {
    fn from(update: InstalledUpdate) -> Self {
        Self {
            version: update.version().to_owned(),
            disposition: match update.disposition() {
                UpdateInstallDisposition::Applied => "applied",
                UpdateInstallDisposition::InstallerLaunched => "installer-launched",
            }
            .to_owned(),
            installed_path: update.installed_path().to_string_lossy().into_owned(),
            backup_path: update
                .backup_path()
                .map(|path| path.to_string_lossy().into_owned()),
            installer_process_id: update.installer_process_id(),
            requires_application_exit: update.requires_application_exit(),
            relaunch_recommended: update.relaunch_recommended(),
        }
    }
}

impl From<AvailableUpdate> for NativeAvailableUpdate {
    fn from(update: AvailableUpdate) -> Self {
        Self {
            version: update.version,
            current_version: update.current_version,
            target: update.target,
            url: update.url,
            signature: update.signature,
            notes: update.notes,
            published_at: update.published_at,
        }
    }
}

impl From<NativeAvailableUpdate> for AvailableUpdate {
    fn from(update: NativeAvailableUpdate) -> Self {
        Self {
            version: update.version,
            current_version: update.current_version,
            target: update.target,
            url: update.url,
            signature: update.signature,
            notes: update.notes,
            published_at: update.published_at,
        }
    }
}

enum AutoStartMutation {
    Enable,
    Disable,
}

#[doc(hidden)]
pub struct AutoStartMutationTask {
    options: NativeAutoStartOptions,
    mutation: AutoStartMutation,
}

impl Task for AutoStartMutationTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        let auto_start = auto_start(self.options.clone())?;
        match self.mutation {
            AutoStartMutation::Enable => auto_start.enable(),
            AutoStartMutation::Disable => auto_start.disable(),
        }
        .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[doc(hidden)]
pub struct AutoStartStatusTask {
    options: NativeAutoStartOptions,
}

impl Task for AutoStartStatusTask {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<Self::Output> {
        auto_start(self.options.clone())?
            .is_enabled()
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

enum ProtocolMutation {
    Register,
    Unregister,
}

#[doc(hidden)]
pub struct ProtocolMutationTask {
    options: NativeProtocolRegistrationOptions,
    mutation: ProtocolMutation,
}

impl Task for ProtocolMutationTask {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<Self::Output> {
        let registration = protocol_registration(self.options.clone())?;
        match self.mutation {
            ProtocolMutation::Register => registration.register().map(|()| true),
            ProtocolMutation::Unregister => registration.unregister(),
        }
        .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[doc(hidden)]
pub struct ProtocolStatusTask {
    options: NativeProtocolRegistrationOptions,
}

impl Task for ProtocolStatusTask {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<Self::Output> {
        protocol_registration(self.options.clone())?
            .is_registered()
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

enum SecureStorageMutation {
    Set(Vec<u8>),
    Delete,
}

#[doc(hidden)]
pub struct SecureStorageMutationTask {
    service: String,
    account: String,
    mutation: SecureStorageMutation,
}

impl Task for SecureStorageMutationTask {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<Self::Output> {
        match &self.mutation {
            SecureStorageMutation::Set(secret) => {
                SecureStorage::set(&self.service, &self.account, secret)
                    .map(|()| true)
                    .map_err(system_error)
            }
            SecureStorageMutation::Delete => {
                SecureStorage::delete(&self.service, &self.account).map_err(system_error)
            }
        }
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[doc(hidden)]
pub struct SecureStorageGetTask {
    service: String,
    account: String,
}

impl Task for SecureStorageGetTask {
    type Output = Option<Vec<u8>>;
    type JsValue = Option<Buffer>;

    fn compute(&mut self) -> Result<Self::Output> {
        SecureStorage::get(&self.service, &self.account).map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.map(Buffer::from))
    }
}

#[doc(hidden)]
pub struct UpdateCheckTask {
    options: NativeUpdateClientOptions,
    endpoint: String,
}

impl Task for UpdateCheckTask {
    type Output = Option<AvailableUpdate>;
    type JsValue = Option<NativeAvailableUpdate>;

    fn compute(&mut self) -> Result<Self::Output> {
        update_client(self.options.clone())?
            .check(&self.endpoint)
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.map(Into::into))
    }
}

#[doc(hidden)]
pub struct UpdateStageTask {
    options: NativeUpdateClientOptions,
    update: NativeAvailableUpdate,
    destination_directory: String,
}

impl Task for UpdateStageTask {
    type Output = PathBuf;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        update_client(self.options.clone())?
            .download_and_stage(
                &self.update.clone().into(),
                PathBuf::from(&self.destination_directory),
            )
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.to_string_lossy().into_owned())
    }
}

#[doc(hidden)]
pub struct UpdateVerifyTask {
    options: NativeUpdateClientOptions,
    path: String,
    signature: String,
}

#[doc(hidden)]
pub struct UpdateInstallTask {
    options: NativeUpdateClientOptions,
    update: NativeAvailableUpdate,
    artifact: String,
    install_options: NativeUpdateInstallOptions,
}

impl Task for UpdateInstallTask {
    type Output = InstalledUpdate;
    type JsValue = NativeInstalledUpdate;

    fn compute(&mut self) -> Result<Self::Output> {
        let install_options = update_install_options(self.install_options.clone())?;
        update_client(self.options.clone())?
            .install_staged(
                &self.update.clone().into(),
                PathBuf::from(&self.artifact),
                install_options,
            )
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into())
    }
}

impl Task for UpdateVerifyTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        update_client(self.options.clone())?
            .verify_file(PathBuf::from(&self.path), &self.signature)
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[napi(ts_return_type = "Promise<void>")]
pub fn enable_auto_start(options: NativeAutoStartOptions) -> AsyncTask<AutoStartMutationTask> {
    AsyncTask::new(AutoStartMutationTask {
        options,
        mutation: AutoStartMutation::Enable,
    })
}

#[napi]
pub fn is_auto_start_supported() -> bool {
    AutoStart::is_supported()
}

#[napi(ts_return_type = "Promise<void>")]
pub fn disable_auto_start(options: NativeAutoStartOptions) -> AsyncTask<AutoStartMutationTask> {
    AsyncTask::new(AutoStartMutationTask {
        options,
        mutation: AutoStartMutation::Disable,
    })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn is_auto_start_enabled(options: NativeAutoStartOptions) -> AsyncTask<AutoStartStatusTask> {
    AsyncTask::new(AutoStartStatusTask { options })
}

#[napi(ts_return_type = "Promise<void>")]
pub fn register_protocol(
    options: NativeProtocolRegistrationOptions,
) -> AsyncTask<ProtocolMutationTask> {
    AsyncTask::new(ProtocolMutationTask {
        options,
        mutation: ProtocolMutation::Register,
    })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn unregister_protocol(
    options: NativeProtocolRegistrationOptions,
) -> AsyncTask<ProtocolMutationTask> {
    AsyncTask::new(ProtocolMutationTask {
        options,
        mutation: ProtocolMutation::Unregister,
    })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn is_protocol_registered(
    options: NativeProtocolRegistrationOptions,
) -> AsyncTask<ProtocolStatusTask> {
    AsyncTask::new(ProtocolStatusTask { options })
}

#[napi]
pub fn supports_dynamic_protocol_registration() -> bool {
    ProtocolRegistration::supports_dynamic_registration()
}

#[napi(ts_return_type = "Promise<void>")]
pub fn set_secure_storage(
    service: String,
    account: String,
    secret: Buffer,
) -> AsyncTask<SecureStorageMutationTask> {
    AsyncTask::new(SecureStorageMutationTask {
        service,
        account,
        mutation: SecureStorageMutation::Set(secret.to_vec()),
    })
}

#[napi(ts_return_type = "Promise<Buffer | undefined>")]
pub fn get_secure_storage(service: String, account: String) -> AsyncTask<SecureStorageGetTask> {
    AsyncTask::new(SecureStorageGetTask { service, account })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn delete_secure_storage(
    service: String,
    account: String,
) -> AsyncTask<SecureStorageMutationTask> {
    AsyncTask::new(SecureStorageMutationTask {
        service,
        account,
        mutation: SecureStorageMutation::Delete,
    })
}

#[napi]
pub fn is_secure_storage_supported() -> bool {
    SecureStorage::is_supported()
}

#[napi(ts_return_type = "Promise<NativeAvailableUpdate | undefined>")]
pub fn check_for_update(
    endpoint: String,
    options: NativeUpdateClientOptions,
) -> AsyncTask<UpdateCheckTask> {
    AsyncTask::new(UpdateCheckTask { options, endpoint })
}

#[napi(ts_return_type = "Promise<string>")]
pub fn stage_update(
    update: NativeAvailableUpdate,
    destination_directory: String,
    options: NativeUpdateClientOptions,
) -> AsyncTask<UpdateStageTask> {
    AsyncTask::new(UpdateStageTask {
        options,
        update,
        destination_directory,
    })
}

#[napi(ts_return_type = "Promise<void>")]
pub fn verify_update(
    path: String,
    signature: String,
    options: NativeUpdateClientOptions,
) -> AsyncTask<UpdateVerifyTask> {
    AsyncTask::new(UpdateVerifyTask {
        options,
        path,
        signature,
    })
}

#[napi(ts_return_type = "Promise<NativeInstalledUpdate>")]
pub fn install_update(
    update: NativeAvailableUpdate,
    artifact: String,
    install_options: NativeUpdateInstallOptions,
    options: NativeUpdateClientOptions,
) -> AsyncTask<UpdateInstallTask> {
    AsyncTask::new(UpdateInstallTask {
        options,
        update,
        artifact,
        install_options,
    })
}

#[napi]
pub fn default_update_target() -> String {
    core_default_update_target()
}

// ---------------------------------------------------------------------------
// Crash reporting
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[napi(object)]
pub struct NativeCrashParameter {
    pub key: String,
    pub value: String,
}

#[derive(Clone)]
#[napi(object)]
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

#[derive(Clone)]
#[napi(object)]
pub struct NativeCrashLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone)]
#[napi(object)]
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

#[derive(Clone)]
#[napi(object)]
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
        value => Err(Error::from_reason(format!(
            "unknown crash backtrace policy `{value}`"
        ))),
    }
}

fn installed_reporter() -> Result<CrashReporter> {
    CrashReporter::current().ok_or_else(|| {
        Error::from_reason(
            "the crash reporter has not been started; call CrashReporter.start first",
        )
    })
}

#[doc(hidden)]
pub struct CrashReporterStartTask {
    options: Option<NativeCrashReporterOptions>,
}

impl Task for CrashReporterStartTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        let options = self
            .options
            .take()
            .ok_or_else(|| Error::from_reason("the crash reporter task was already computed"))?;
        if let Some(existing) = CrashReporter::current() {
            return Ok(existing.directory().to_string_lossy().into_owned());
        }
        let reporter =
            CrashReporter::install(crash_reporter_options(options)?).map_err(system_error)?;
        Ok(reporter.directory().to_string_lossy().into_owned())
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

enum CrashQuery {
    Last,
    Pending,
}

#[doc(hidden)]
pub struct CrashReportQueryTask {
    query: CrashQuery,
}

impl Task for CrashReportQueryTask {
    type Output = Vec<CrashReport>;
    type JsValue = Vec<NativeCrashReport>;

    fn compute(&mut self) -> Result<Self::Output> {
        let reporter = installed_reporter()?;
        match self.query {
            CrashQuery::Last => Ok(reporter
                .last_crash_report()
                .map_err(system_error)?
                .into_iter()
                .collect()),
            CrashQuery::Pending => reporter.pending_reports().map_err(system_error),
        }
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into_iter().map(Into::into).collect())
    }
}

enum CrashParameterMutation {
    Add { key: String, value: String },
    Remove { key: String },
}

#[doc(hidden)]
pub struct CrashParameterTask {
    mutation: CrashParameterMutation,
}

impl Task for CrashParameterTask {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<Self::Output> {
        let reporter = installed_reporter()?;
        match &self.mutation {
            CrashParameterMutation::Add { key, value } => reporter
                .add_extra_parameter(key.clone(), value.clone())
                .map(|()| true)
                .map_err(system_error),
            CrashParameterMutation::Remove { key } => Ok(reporter.remove_extra_parameter(key)),
        }
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[doc(hidden)]
pub struct CrashReportDeleteTask {
    id: String,
}

impl Task for CrashReportDeleteTask {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<Self::Output> {
        installed_reporter()?
            .delete_report(&self.id)
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[doc(hidden)]
pub struct CrashUploadTask {
    endpoint: Option<String>,
}

impl Task for CrashUploadTask {
    type Output = UploadSummary;
    type JsValue = NativeCrashUploadSummary;

    fn compute(&mut self) -> Result<Self::Output> {
        let reporter = installed_reporter()?;
        let endpoint = match self.endpoint.clone() {
            Some(endpoint) => endpoint,
            None => reporter
                .upload_endpoint()
                .map(str::to_owned)
                .ok_or_else(|| Error::from_reason("no crash upload endpoint is configured"))?,
        };
        reporter.upload_pending(&endpoint).map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into())
    }
}

#[napi(ts_return_type = "Promise<string>")]
pub fn start_crash_reporter(
    options: NativeCrashReporterOptions,
) -> AsyncTask<CrashReporterStartTask> {
    AsyncTask::new(CrashReporterStartTask {
        options: Some(options),
    })
}

#[napi]
pub fn is_crash_reporter_started() -> bool {
    CrashReporter::current().is_some()
}

#[napi(ts_return_type = "Promise<NativeCrashReport[]>")]
pub fn get_last_crash_report() -> AsyncTask<CrashReportQueryTask> {
    AsyncTask::new(CrashReportQueryTask {
        query: CrashQuery::Last,
    })
}

#[napi(ts_return_type = "Promise<NativeCrashReport[]>")]
pub fn get_pending_crash_reports() -> AsyncTask<CrashReportQueryTask> {
    AsyncTask::new(CrashReportQueryTask {
        query: CrashQuery::Pending,
    })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn add_crash_extra_parameter(key: String, value: String) -> AsyncTask<CrashParameterTask> {
    AsyncTask::new(CrashParameterTask {
        mutation: CrashParameterMutation::Add { key, value },
    })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn remove_crash_extra_parameter(key: String) -> AsyncTask<CrashParameterTask> {
    AsyncTask::new(CrashParameterTask {
        mutation: CrashParameterMutation::Remove { key },
    })
}

#[napi(ts_return_type = "Promise<boolean>")]
pub fn delete_crash_report(id: String) -> AsyncTask<CrashReportDeleteTask> {
    AsyncTask::new(CrashReportDeleteTask { id })
}

#[napi(ts_return_type = "Promise<NativeCrashUploadSummary>")]
pub fn upload_pending_crash_reports(endpoint: Option<String>) -> AsyncTask<CrashUploadTask> {
    AsyncTask::new(CrashUploadTask { endpoint })
}

// ---------------------------------------------------------------------------
// Process and system metrics
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[napi(object)]
pub struct NativeProcessMetrics {
    pub cpu_user_seconds: f64,
    pub cpu_system_seconds: f64,
    pub resident_bytes: f64,
    pub footprint_bytes: Option<f64>,
    pub virtual_bytes: f64,
    pub thread_count: Option<u32>,
    pub uptime_seconds: f64,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeSystemMemory {
    pub total_bytes: f64,
    pub available_bytes: f64,
    pub free_bytes: f64,
    pub used_bytes: f64,
}

#[derive(Clone)]
#[napi(object)]
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

#[doc(hidden)]
pub struct ProcessMetricsTask;

impl Task for ProcessMetricsTask {
    type Output = ProcessMetrics;
    type JsValue = NativeProcessMetrics;

    fn compute(&mut self) -> Result<Self::Output> {
        ProcessMetrics::current().map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into())
    }
}

#[doc(hidden)]
pub struct SystemMemoryTask;

impl Task for SystemMemoryTask {
    type Output = SystemMemory;
    type JsValue = NativeSystemMemory;

    fn compute(&mut self) -> Result<Self::Output> {
        SystemMemory::current().map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into())
    }
}

#[doc(hidden)]
pub struct CpuSampleTask {
    sampler: Arc<Mutex<CpuUsageSampler>>,
}

impl Task for CpuSampleTask {
    type Output = CpuUsage;
    type JsValue = NativeCpuUsage;

    fn compute(&mut self) -> Result<Self::Output> {
        let mut sampler = self
            .sampler
            .lock()
            .map_err(|_| Error::from_reason("the CPU usage sampler is poisoned"))?;
        sampler.sample().map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into())
    }
}

/// Stateful CPU sampler. Each `sample()` reports usage since the previous call on this instance.
#[napi]
pub struct NativeCpuUsageSampler {
    sampler: Arc<Mutex<CpuUsageSampler>>,
}

#[napi]
impl NativeCpuUsageSampler {
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            sampler: Arc::new(Mutex::new(CpuUsageSampler::new())),
        }
    }

    #[napi(ts_return_type = "Promise<NativeCpuUsage>")]
    pub fn sample(&self) -> AsyncTask<CpuSampleTask> {
        AsyncTask::new(CpuSampleTask {
            sampler: Arc::clone(&self.sampler),
        })
    }
}

#[napi(ts_return_type = "Promise<NativeProcessMetrics>")]
pub fn get_process_metrics() -> AsyncTask<ProcessMetricsTask> {
    AsyncTask::new(ProcessMetricsTask)
}

#[napi(ts_return_type = "Promise<NativeSystemMemory>")]
pub fn get_system_memory() -> AsyncTask<SystemMemoryTask> {
    AsyncTask::new(SystemMemoryTask)
}

// ---------------------------------------------------------------------------
// Updater download progress
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[napi(object)]
pub struct NativeUpdateProgress {
    pub phase: String,
    pub chunk_bytes: Option<f64>,
    pub downloaded_bytes: Option<f64>,
    pub total_bytes: Option<f64>,
    pub path: Option<String>,
}

fn native_update_progress(progress: &UpdateProgress) -> Option<NativeUpdateProgress> {
    let empty = NativeUpdateProgress {
        phase: String::new(),
        chunk_bytes: None,
        downloaded_bytes: None,
        total_bytes: None,
        path: None,
    };
    Some(match progress {
        UpdateProgress::DownloadStarted { total_bytes } => NativeUpdateProgress {
            phase: "download-started".to_owned(),
            total_bytes: total_bytes.map(|bytes| bytes as f64),
            ..empty
        },
        UpdateProgress::Downloaded {
            chunk_bytes,
            downloaded_bytes,
            total_bytes,
        } => NativeUpdateProgress {
            phase: "downloaded".to_owned(),
            chunk_bytes: Some(*chunk_bytes as f64),
            downloaded_bytes: Some(*downloaded_bytes as f64),
            total_bytes: total_bytes.map(|bytes| bytes as f64),
            ..empty
        },
        UpdateProgress::DownloadFinished { downloaded_bytes } => NativeUpdateProgress {
            phase: "download-finished".to_owned(),
            downloaded_bytes: Some(*downloaded_bytes as f64),
            ..empty
        },
        UpdateProgress::VerificationStarted => NativeUpdateProgress {
            phase: "verification-started".to_owned(),
            ..empty
        },
        UpdateProgress::VerificationFinished => NativeUpdateProgress {
            phase: "verification-finished".to_owned(),
            ..empty
        },
        UpdateProgress::Staged { path } => NativeUpdateProgress {
            phase: "staged".to_owned(),
            path: Some(path.to_string_lossy().into_owned()),
            ..empty
        },
        _ => return None,
    })
}

type UpdateProgressCallback =
    ThreadsafeFunction<NativeUpdateProgress, (), NativeUpdateProgress, Status, false>;

#[doc(hidden)]
pub struct UpdateStageProgressTask {
    options: NativeUpdateClientOptions,
    update: NativeAvailableUpdate,
    destination_directory: String,
    on_progress: UpdateProgressCallback,
}

impl Task for UpdateStageProgressTask {
    type Output = PathBuf;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        let callback = &self.on_progress;
        update_client(self.options.clone())?
            .download_and_stage_with_progress(
                &self.update.clone().into(),
                PathBuf::from(&self.destination_directory),
                |progress| {
                    if let Some(progress) = native_update_progress(&progress) {
                        callback.call(progress, ThreadsafeFunctionCallMode::NonBlocking);
                    }
                },
            )
            .map_err(system_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.to_string_lossy().into_owned())
    }
}

#[napi(ts_return_type = "Promise<string>")]
pub fn stage_update_with_progress(
    update: NativeAvailableUpdate,
    destination_directory: String,
    options: NativeUpdateClientOptions,
    #[napi(ts_arg_type = "(progress: NativeUpdateProgress) => void")]
    on_progress: UpdateProgressCallback,
) -> AsyncTask<UpdateStageProgressTask> {
    AsyncTask::new(UpdateStageProgressTask {
        options,
        update,
        destination_directory,
        on_progress,
    })
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
        mode => Err(Error::from_reason(format!(
            "unknown native autostart mode `{mode}`"
        ))),
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

fn update_install_options(options: NativeUpdateInstallOptions) -> Result<UpdateInstallOptions> {
    let mut native = UpdateInstallOptions::new();
    if let Some(executable) = options.target_executable {
        native = native.target_executable(executable);
    }
    if let Some(retain) = options.retain_backup {
        native = native.retain_backup(retain);
    }
    if let Some(mode) = options.windows_mode {
        native = native.windows_mode(match mode.as_str() {
            "basic-ui" | "basicUi" => WindowsUpdateInstallMode::BasicUi,
            "quiet" => WindowsUpdateInstallMode::Quiet,
            "passive" => WindowsUpdateInstallMode::Passive,
            value => {
                return Err(Error::from_reason(format!(
                    "unknown Windows update install mode `{value}`"
                )));
            }
        });
    }
    if let Some(arguments) = options.installer_arguments {
        native = native.installer_arguments(arguments);
    }
    Ok(native)
}

fn update_client(options: NativeUpdateClientOptions) -> Result<UpdateClient> {
    let mut client =
        UpdateClient::new(&options.current_version, options.public_key).map_err(system_error)?;
    if let Some(target) = options.target {
        client = client.target(target).map_err(system_error)?;
    }
    if let Some(maximum) = options.maximum_download_bytes {
        client = client
            .maximum_download_bytes(u64::from(maximum))
            .map_err(system_error)?;
    }
    Ok(client)
}

fn system_error(error: SystemIntegrationError) -> Error {
    Error::from_reason(error.to_string())
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
    fn crash_reporter_options_bound_every_javascript_input() {
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
    fn maps_reportable_update_progress_phases() {
        let started = native_update_progress(&UpdateProgress::DownloadStarted {
            total_bytes: Some(1024),
        })
        .unwrap();
        assert_eq!(started.phase, "download-started");
        assert_eq!(started.total_bytes, Some(1024.0));
        assert_eq!(started.downloaded_bytes, None);

        let downloaded = native_update_progress(&UpdateProgress::Downloaded {
            chunk_bytes: 64,
            downloaded_bytes: 512,
            total_bytes: None,
        })
        .unwrap();
        assert_eq!(downloaded.phase, "downloaded");
        assert_eq!(downloaded.chunk_bytes, Some(64.0));
        assert_eq!(downloaded.downloaded_bytes, Some(512.0));
        assert_eq!(downloaded.total_bytes, None);

        assert_eq!(
            native_update_progress(&UpdateProgress::VerificationStarted)
                .unwrap()
                .phase,
            "verification-started"
        );
        let staged = native_update_progress(&UpdateProgress::Staged {
            path: PathBuf::from("/tmp/App.app.tar.gz"),
        })
        .unwrap();
        assert_eq!(staged.phase, "staged");
        assert_eq!(staged.path.as_deref(), Some("/tmp/App.app.tar.gz"));

        // Installation milestones belong to `installUpdate`, not to the staging callback.
        assert!(
            native_update_progress(&UpdateProgress::InstallStarted {
                artifact: PathBuf::from("/tmp/App.app.tar.gz"),
            })
            .is_none()
        );
    }

    #[test]
    fn converts_core_metrics_into_javascript_objects() {
        let metrics = ProcessMetrics {
            cpu_user: std::time::Duration::from_millis(1_500),
            cpu_system: std::time::Duration::from_millis(500),
            resident_bytes: 4096,
            footprint_bytes: Some(8192),
            virtual_bytes: 16_384,
            thread_count: Some(7),
            uptime: std::time::Duration::from_secs(30),
        };
        let native: NativeProcessMetrics = metrics.into();
        assert_eq!(native.cpu_user_seconds, 1.5);
        assert_eq!(native.cpu_system_seconds, 0.5);
        assert_eq!(native.resident_bytes, 4096.0);
        assert_eq!(native.footprint_bytes, Some(8192.0));
        assert_eq!(native.thread_count, Some(7));
        assert_eq!(native.uptime_seconds, 30.0);

        let memory: NativeSystemMemory = SystemMemory {
            total_bytes: 100,
            available_bytes: 40,
            free_bytes: 10,
            used_bytes: 60,
        }
        .into();
        assert_eq!(memory.total_bytes, 100.0);
        assert_eq!(memory.used_bytes, 60.0);

        let usage: NativeCpuUsage = CpuUsage {
            percent: Some(12.5),
            interval: std::time::Duration::from_secs(2),
            cpu_time: std::time::Duration::from_millis(250),
            total_cpu_time: std::time::Duration::from_secs(4),
        }
        .into();
        assert_eq!(usage.percent, Some(12.5));
        assert_eq!(usage.interval_seconds, 2.0);
        assert_eq!(usage.cpu_seconds, 0.25);
        assert_eq!(usage.total_cpu_seconds, 4.0);

        let summary: NativeCrashUploadSummary = UploadSummary {
            attempted: 3,
            uploaded: 2,
            failed: 1,
        }
        .into();
        assert_eq!(
            (summary.attempted, summary.uploaded, summary.failed),
            (3, 2, 1)
        );
    }

    #[test]
    fn flattens_crash_report_parameters_for_javascript() {
        let mut parameters = std::collections::BTreeMap::new();
        parameters.insert("channel".to_owned(), "beta".to_owned());
        parameters.insert("build".to_owned(), "42".to_owned());
        let report = CrashReport {
            schema_version: 1,
            id: "1-2-3".to_owned(),
            kind: quickgui::CrashKind::Signal,
            timestamp: "2026-09-03T12:00:00Z".to_owned(),
            app_name: "Demo".to_owned(),
            app_version: "1.0.0".to_owned(),
            app_identifier: "com.example.demo".to_owned(),
            operating_system: "Mac OS".to_owned(),
            operating_system_version: Some("15.0".to_owned()),
            architecture: "aarch64".to_owned(),
            process_id: 4321,
            thread: Some("0x1f".to_owned()),
            message: "fatal fault".to_owned(),
            location: None,
            backtrace: None,
            signal: Some(11),
            signal_name: Some("SIGSEGV".to_owned()),
            fault_address: Some("0x0".to_owned()),
            parameters,
        };
        let native: NativeCrashReport = report.into();
        assert_eq!(native.kind, "signal");
        assert_eq!(native.signal, Some(11));
        assert_eq!(native.parameters.len(), 2);
        // `BTreeMap` iteration is sorted, so the JavaScript array order is deterministic.
        assert_eq!(native.parameters[0].key, "build");
        assert_eq!(native.parameters[1].key, "channel");
    }
}
