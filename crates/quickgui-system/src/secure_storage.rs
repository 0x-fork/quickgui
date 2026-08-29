use crate::{Result, invalid, platform, validate_text};

pub const MAX_SECURE_STORAGE_NAME_BYTES: usize = 4 * 1024;
pub const MAX_SECURE_STORAGE_SECRET_BYTES: usize = 1024 * 1024;

/// Native credential storage: Keychain Services, Windows Credential Manager, or Secret Service.
pub struct SecureStorage;

impl SecureStorage {
    pub const fn is_supported() -> bool {
        cfg!(any(unix, windows))
    }

    pub fn set(service: &str, account: &str, secret: &[u8]) -> Result<()> {
        validate_entry(service, account)?;
        if secret.len() > MAX_SECURE_STORAGE_SECRET_BYTES {
            return Err(invalid("secure-storage values cannot exceed 1 MiB"));
        }
        keyring::Entry::new(service, account)
            .and_then(|entry| entry.set_secret(secret))
            .map_err(platform)
    }

    pub fn get(service: &str, account: &str) -> Result<Option<Vec<u8>>> {
        validate_entry(service, account)?;
        let entry = keyring::Entry::new(service, account).map_err(platform)?;
        match entry.get_secret() {
            Ok(secret) if secret.len() <= MAX_SECURE_STORAGE_SECRET_BYTES => Ok(Some(secret)),
            Ok(_) => Err(invalid(
                "the native credential exceeds QuickGUI's 1 MiB result limit",
            )),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(platform(error)),
        }
    }

    pub fn delete(service: &str, account: &str) -> Result<bool> {
        validate_entry(service, account)?;
        let entry = keyring::Entry::new(service, account).map_err(platform)?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(error) => Err(platform(error)),
        }
    }
}

fn validate_entry(service: &str, account: &str) -> Result<()> {
    validate_text(
        service,
        "the secure-storage service",
        MAX_SECURE_STORAGE_NAME_BYTES,
    )?;
    validate_text(
        account,
        "the secure-storage account",
        MAX_SECURE_STORAGE_NAME_BYTES,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_names_and_value_bounds_without_touching_the_keyring() {
        assert!(validate_entry("com.example.app", "token").is_ok());
        assert!(validate_entry("", "token").is_err());
        assert!(validate_entry("app", "bad\0account").is_err());
    }
}
