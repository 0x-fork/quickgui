//! Extension registration happens before the host starts. Registered images remain loaded for
//! process lifetime; individual sessions still release all resources when they unmount.
use crate::extension_api::{self as abi, Extension, ServiceApi, TerminalApi};
use std::sync::OnceLock;

static UPDATER: OnceLock<ServiceApi> = OnceLock::new();

pub fn service(name: &str) -> Option<&'static ServiceApi> {
    match name {
        "updater" => UPDATER.get(),
        _ => None,
    }
}

pub fn shutdown_services() {
    if let Some(api) = UPDATER.get() {
        unsafe { (api.shutdown)() };
    }
}

static TERMINAL: OnceLock<TerminalApi> = OnceLock::new();

#[cfg(all(feature = "terminal-extension", not(feature = "terminal")))]
pub(crate) fn terminal() -> Option<&'static TerminalApi> {
    TERMINAL.get()
}

/// Register a provider obtained from `quickgui_extension_v1` through an in-process loader.
///
/// # Safety
/// The descriptor, its borrowed strings, and function table must be readable and remain valid
/// until process exit. All functions must implement the documented ABI and ownership contracts.
pub unsafe fn register_extension(
    pointer: *const Extension,
    expected_name: &[u8],
) -> Result<(), &'static str> {
    if pointer.is_null() {
        return Err("extension descriptor is null");
    }
    // Read only the fixed two-word header until the caller's layout is known to match.
    let header = pointer.cast::<u32>();
    if unsafe { header.read() } != abi::ABI_VERSION
        || unsafe { header.add(1).read() } as usize != size_of::<Extension>()
    {
        return Err("extension ABI version or descriptor size mismatch");
    }
    let descriptor = unsafe { &*pointer };
    if descriptor.name.len > abi::MAX_EXTENSION_NAME
        || descriptor.version.len > 64
        || descriptor.name.data.is_null()
        || descriptor.version.data.is_null()
    {
        return Err("invalid extension metadata");
    }
    let name = unsafe { std::slice::from_raw_parts(descriptor.name.data, descriptor.name.len) };
    if name != expected_name {
        return Err("loaded extension does not match the requested package");
    }
    let version =
        unsafe { std::slice::from_raw_parts(descriptor.version.data, descriptor.version.len) };
    if version != env!("CARGO_PKG_VERSION").as_bytes() {
        return Err("extension and core release versions differ");
    }
    if descriptor.kind == abi::UPDATER_EXTENSION && name == b"updater" {
        if descriptor.api.is_null() || descriptor.api_size as usize != size_of::<ServiceApi>() {
            return Err("updater extension function table mismatch");
        }
        let api = unsafe { *(descriptor.api.cast::<ServiceApi>()) };
        if let Err(api) = UPDATER.set(api) {
            let current = UPDATER.get().unwrap();
            if current.invoke as usize != api.invoke as usize
                || current.shutdown as usize != api.shutdown as usize
            {
                return Err("a different updater extension is already registered");
            }
        }
        return Ok(());
    }
    if descriptor.kind != abi::TERMINAL_EXTENSION || name != b"terminal" {
        return Err("extension is not supported by this core version");
    }
    if descriptor.api.is_null() || descriptor.api_size as usize != size_of::<TerminalApi>() {
        return Err("terminal extension function table mismatch");
    }
    let api = unsafe { *(descriptor.api.cast::<TerminalApi>()) };
    if let Err(api) = TERMINAL.set(api) {
        let current = TERMINAL.get().unwrap();
        if current.create as usize != api.create as usize
            || current.destroy as usize != api.destroy as usize
            || current.command as usize != api.command as usize
            || current.frame as usize != api.frame as usize
        {
            return Err("a different terminal extension is already registered");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn rejects_header_before_reading_an_unknown_descriptor_layout() {
        let header = [abi::ABI_VERSION + 1, 8];
        assert!(unsafe { register_extension(header.as_ptr().cast(), b"terminal") }.is_err());
        let short = [abi::ABI_VERSION, 8];
        assert!(unsafe { register_extension(short.as_ptr().cast(), b"terminal") }.is_err());
        assert!(unsafe { register_extension(ptr::null(), b"terminal") }.is_err());
    }

    #[test]
    fn rejects_wrong_package_release_and_function_table() {
        let mut descriptor = Extension {
            abi_version: abi::ABI_VERSION,
            descriptor_size: size_of::<Extension>() as u32,
            kind: abi::TERMINAL_EXTENSION,
            api_size: size_of::<TerminalApi>() as u32,
            name: abi::Bytes::new(b"terminal"),
            version: abi::Bytes::new(b"999.0.0"),
            api: ptr::null(),
        };
        assert_eq!(
            unsafe { register_extension(&descriptor, b"different") },
            Err("loaded extension does not match the requested package")
        );
        assert_eq!(
            unsafe { register_extension(&descriptor, b"terminal") },
            Err("extension and core release versions differ")
        );
        descriptor.version = abi::Bytes::new(env!("CARGO_PKG_VERSION").as_bytes());
        assert_eq!(
            unsafe { register_extension(&descriptor, b"terminal") },
            Err("terminal extension function table mismatch")
        );
    }
}
