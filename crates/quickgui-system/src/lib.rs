//! Bounded, cross-platform operating-system integrations independent of QuickGUI's renderer.

#[cfg(feature = "autostart")]
mod autostart;
#[cfg(feature = "protocol")]
mod protocol;
#[cfg(feature = "secure-storage")]
mod secure_storage;
#[cfg(feature = "updater")]
mod update;

#[cfg(feature = "autostart")]
pub use autostart::{AutoStart, AutoStartMode, AutoStartOptions};
#[cfg(feature = "protocol")]
pub use protocol::{ProtocolRegistration, ProtocolRegistrationOptions};
#[cfg(feature = "secure-storage")]
pub use secure_storage::SecureStorage;
#[cfg(feature = "updater")]
pub use update::{
    AvailableUpdate, DEFAULT_MAX_UPDATE_BYTES, MAX_UPDATE_MANIFEST_BYTES,
    MAX_UPDATE_SIGNATURE_BYTES, UpdateClient, default_update_target,
};

use std::sync::Arc;

use thiserror::Error;

/// An operating-system integration rejected or failed an operation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SystemIntegrationError {
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

fn validate_text(value: &str, name: &str, maximum: usize) -> Result<()> {
    if value.is_empty() || value.contains('\0') || value.len() > maximum {
        return Err(invalid(format!(
            "{name} must be nonempty, NUL-free, and at most {maximum} UTF-8 bytes"
        )));
    }
    Ok(())
}
