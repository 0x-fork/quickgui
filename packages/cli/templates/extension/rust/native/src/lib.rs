mod abi;
mod build_info;

use abi::{Bytes, Extension, ServiceApi, ServiceSink};
use std::{ffi::c_void, mem::size_of, slice};

unsafe extern "C" fn invoke(_request: u32, method: Bytes, params: Bytes, sink: ServiceSink) {
    // This bounded CPU-only example replies immediately without allocating. Copy
    // borrowed input and retain the sink when queuing asynchronous work instead.
    if method.len == 4
        && !method.data.is_null()
        && slice::from_raw_parts(method.data, method.len) == b"echo"
        && params.len <= abi::MAX_PAYLOAD
    {
        (sink.emit)(sink.context, abi::REPLY, params);
    } else {
        (sink.emit)(
            sink.context,
            abi::ERROR,
            Bytes::new(b"Unknown method or oversized payload"),
        );
    }
    (sink.release)(sink.context);
}

extern "C" fn shutdown() {
    // No sessions or workers in this example. Cancel yours here without waiting.
}

static API: ServiceApi = ServiceApi { invoke, shutdown };
static DESCRIPTOR: Extension = Extension {
    abi_version: abi::ABI_VERSION,
    descriptor_size: size_of::<Extension>() as u32,
    kind: abi::SERVICE,
    api_size: size_of::<ServiceApi>() as u32,
    name: Bytes::new(build_info::NAME.as_bytes()),
    version: Bytes::new(build_info::VERSION.as_bytes()),
    api: &API as *const ServiceApi as *const c_void,
};

#[no_mangle]
pub extern "C" fn quickgui_extension_v1() -> *const Extension {
    &DESCRIPTOR
}
