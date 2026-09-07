//! Async service extensions share the host event queue, without another renderer or Go callback
//! crossing native threads. A session sink stays alive until the provider's last worker releases it.
use quickgui::extension_api::{Bytes, ServiceSink};
use std::{
    ffi::c_void,
    sync::atomic::{AtomicBool, Ordering},
};

struct Context {
    request: u32,
    replied: AtomicBool,
}

pub(crate) struct ApplicationServices;

impl Drop for ApplicationServices {
    fn drop(&mut self) {
        quickgui::extensions::shutdown_services();
    }
}

pub(crate) fn invoke(request: u32, service: &str, params: &str) {
    let Some((name, method)) = service.split_once('/') else {
        crate::publish_reply("invoke", request, Err("invalid extension service".into()));
        return;
    };
    let Some(api) = quickgui::extensions::service(name) else {
        crate::publish_reply(
            "invoke",
            request,
            Err(format!(
                "import github.com/egoist/quickgui/go/{name} to load this extension"
            )),
        );
        return;
    };
    let context = Box::into_raw(Box::new(Context {
        request,
        replied: AtomicBool::new(false),
    }))
    .cast();
    // SAFETY: the registered provider owns the context and copies both input spans before return.
    unsafe {
        (api.invoke)(
            request,
            Bytes::new(method.as_bytes()),
            Bytes::new(params.as_bytes()),
            ServiceSink {
                context,
                emit,
                release,
            },
        );
    }
}

unsafe extern "C" fn emit(context: *mut c_void, kind: u32, bytes: Bytes) {
    let context = unsafe { &*context.cast::<Context>() };
    if bytes.len > 64 * 1024 || bytes.data.is_null() {
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) };
    if kind < 2 && context.replied.swap(true, Ordering::AcqRel) {
        return;
    }
    let value = if kind == 1 {
        Err(String::from_utf8_lossy(bytes).into_owned())
    } else {
        serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid extension response: {error}"))
    };
    crate::publish_reply(
        if kind == 2 {
            "extension-event"
        } else {
            "invoke"
        },
        context.request,
        value,
    );
}

unsafe extern "C" fn release(context: *mut c_void) {
    let context = unsafe { Box::from_raw(context.cast::<Context>()) };
    if !context.replied.load(Ordering::Acquire) {
        crate::publish_reply(
            "invoke",
            context.request,
            Err("extension stopped before completing the request".into()),
        );
    }
}
