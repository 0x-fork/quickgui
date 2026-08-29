//! Bounded, cross-platform operating-system integrations independent of QuickGUI's renderer.

mod app_environment;
#[cfg(feature = "autostart")]
mod autostart;
mod power;
mod preferences;
mod process;
#[cfg(feature = "protocol")]
mod protocol;
#[cfg(feature = "secure-storage")]
mod secure_storage;
#[cfg(feature = "updater")]
mod update;

pub use app_environment::{
    AppInfo, AppPaths, MAX_APP_IDENTIFIER_BYTES, MAX_APP_NAME_BYTES, MAX_APP_VERSION_BYTES,
    MAX_PREFERRED_LANGUAGES, MAX_SYSTEM_LOCALE_BYTES, MAX_SYSTEM_LOCALES_TOTAL_BYTES,
    MAX_SYSTEM_TEXT_BYTES, OperatingSystem, OperatingSystemFamily, SystemBitness, SystemInfo,
};
#[cfg(feature = "autostart")]
pub use autostart::{AutoStart, AutoStartMode, AutoStartOptions};
pub use power::{
    BatteryState, BatteryStatus, IdleState, MAX_IDLE_THRESHOLD, MAX_POWER_ASSERTION_REASON_BYTES,
    PowerAssertion, PowerAssertionKind, PowerMonitor, PowerSource, PowerState, SessionState,
    ThermalState,
};
pub use preferences::{
    ColorScheme, PermissionKind, PermissionManager, PermissionStatus, SystemColor, SystemColorRole,
    SystemPreferences,
};
pub use process::{
    MAX_RELAUNCH_ARGUMENT_BYTES, MAX_RELAUNCH_ARGUMENTS, MAX_RELAUNCH_VALUE_BYTES, RelaunchOptions,
    RelaunchRequest, RelaunchedProcess,
};
#[cfg(feature = "protocol")]
pub use protocol::{ProtocolRegistration, ProtocolRegistrationOptions};
#[cfg(feature = "secure-storage")]
pub use secure_storage::SecureStorage;
#[cfg(feature = "updater")]
pub use update::{
    AvailableUpdate, DEFAULT_MAX_EXPANDED_UPDATE_BYTES, DEFAULT_MAX_UPDATE_BYTES, InstalledUpdate,
    MAX_UPDATE_ARCHIVE_ENTRIES, MAX_UPDATE_INSTALLER_ARGUMENT_BYTES,
    MAX_UPDATE_INSTALLER_ARGUMENTS, MAX_UPDATE_MANIFEST_BYTES, MAX_UPDATE_SIGNATURE_BYTES,
    UpdateCancellation, UpdateClient, UpdateInstallDisposition, UpdateInstallOptions,
    UpdateProgress, WindowsUpdateInstallMode, default_update_target,
};

use std::sync::Arc;

use thiserror::Error;

/// An operating-system integration rejected or failed an operation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SystemIntegrationError {
    #[error("the operating-system integration operation was cancelled")]
    Cancelled,
    #[error("this integration is not supported on the current operating system")]
    Unsupported,
    #[error("invalid system integration input: {0}")]
    InvalidInput(Arc<str>),
    #[error("operating-system integration failed: {0}")]
    Platform(Arc<str>),
    #[error("network operation failed: {0}")]
    Network(Arc<str>),
    #[error("update metadata is invalid: {0}")]
    InvalidUpdate(Arc<str>),
    #[error("update signature verification failed: {0}")]
    InvalidSignature(Arc<str>),
}

pub type Result<T> = std::result::Result<T, SystemIntegrationError>;

fn invalid(message: impl Into<Arc<str>>) -> SystemIntegrationError {
    SystemIntegrationError::InvalidInput(message.into())
}

fn platform(error: impl ToString) -> SystemIntegrationError {
    SystemIntegrationError::Platform(Arc::from(error.to_string()))
}

#[cfg(any(
    feature = "autostart",
    feature = "protocol",
    feature = "secure-storage"
))]
fn validate_text(value: &str, name: &str, maximum: usize) -> Result<()> {
    if value.is_empty() || value.contains('\0') || value.len() > maximum {
        return Err(invalid(format!(
            "{name} must be nonempty, NUL-free, and at most {maximum} UTF-8 bytes"
        )));
    }
    Ok(())
}
