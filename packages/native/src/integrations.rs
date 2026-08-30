use std::path::PathBuf;

use napi::{
    Env, Error, Result, Task,
    bindgen_prelude::{AsyncTask, Buffer},
};
use napi_derive::napi;
use quickgui::{
    AutoStart, AutoStartMode, AutoStartOptions, AvailableUpdate, InstalledUpdate,
    ProtocolRegistration, ProtocolRegistrationOptions, SecureStorage, SystemIntegrationError,
    UpdateClient, UpdateInstallDisposition, UpdateInstallOptions, WindowsUpdateInstallMode,
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
}
