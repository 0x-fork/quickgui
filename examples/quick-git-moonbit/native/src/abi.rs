//! Vendored QuickGUI service ABI v1. All calls use C layout and calling convention.
//! Source: https://github.com/egoist/quickgui/blob/main/include/quickgui_extension.h
use std::ffi::c_void;

pub const ABI_VERSION: u32 = 1;
pub const SERVICE: u32 = 2;
pub const MAX_PAYLOAD: usize = 64 * 1024;
pub const REPLY: u32 = 0;
pub const ERROR: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bytes {
    pub data: *const u8,
    pub len: usize,
}

impl Bytes {
    pub const fn new(value: &[u8]) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }
}

#[repr(C)]
pub struct ServiceSink {
    pub context: *mut c_void,
    pub emit: unsafe extern "C" fn(*mut c_void, u32, Bytes),
    pub release: unsafe extern "C" fn(*mut c_void),
}

#[repr(C)]
pub struct ServiceApi {
    pub invoke: unsafe extern "C" fn(u32, Bytes, Bytes, ServiceSink),
    pub shutdown: extern "C" fn(),
}

#[repr(C)]
pub struct Extension {
    pub abi_version: u32,
    pub descriptor_size: u32,
    pub kind: u32,
    pub api_size: u32,
    pub name: Bytes,
    pub version: Bytes,
    pub api: *const c_void,
}

// Only used for the immutable, process-lifetime descriptor and its static bytes/table.
unsafe impl Sync for Extension {}
