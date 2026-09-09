//! Async service extensions share the host event queue, without another renderer or Go callback
//! crossing native threads. A session sink stays alive until the extension's last worker releases it.
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
    if method.is_empty()
        || method.len() > 64
        || !method.as_bytes()[0].is_ascii_alphabetic()
        || !method
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        || params.len() > 64 * 1024
    {
        crate::publish_reply(
            "invoke",
            request,
            Err("invalid extension method or oversized payload".into()),
        );
        return;
    }
    let Some(api) = quickgui::extensions::service(name) else {
        crate::publish_reply(
            "invoke",
            request,
            Err(format!(
                "extension {name} is not loaded; import its Go package and rebuild the application"
            )),
        );
        return;
    };
    let context = Box::into_raw(Box::new(Context {
        request,
        replied: AtomicBool::new(false),
    }))
    .cast();
    // SAFETY: the registered extension owns the context and copies inputs it retains after return.
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
    if kind > 2 || bytes.len > 64 * 1024 || bytes.data.is_null() {
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) };
    if kind < 2 && context.replied.swap(true, Ordering::AcqRel) {
        return;
    }
    let value = if kind == 1 {
        Err(String::from_utf8_lossy(bytes).into_owned())
    } else {
        json_response(bytes)
    };
    let event_kind = if kind == 2 {
        "extension-event"
    } else {
        "invoke"
    };
    let event = match value {
        Ok(value) => crate::NativeEvent::reply(event_kind, context.request, Some(value), None),
        Err(error) => crate::NativeEvent::reply(event_kind, context.request, None, Some(error)),
    };
    crate::HOST.publish_events([event]);
}

fn json_response(bytes: &[u8]) -> Result<String, String> {
    let json = std::str::from_utf8(bytes)
        .map_err(|error| format!("invalid extension response: {error}"))?;
    // Validate without allocating a Value tree or rewriting the extension's schema.
    // Large numbers, explicit nulls, and whitespace survive the native boundary.
    serde_json::from_str::<serde::de::IgnoredAny>(json)
        .map_err(|error| format!("invalid extension response: {error}"))?;
    Ok(json.to_owned())
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

#[cfg(test)]
mod tests {
    use super::json_response;

    #[test]
    fn service_payloads_preserve_extension_data_without_a_value_tree() {
        let source = r#"{ "null": null, "integer": 900719925474099312345, "fraction": 0.12345678901234567890123, "items": [null, false, "hello"] }"#;
        assert_eq!(json_response(source.as_bytes()).unwrap(), source);
        assert_eq!(json_response(b"null").unwrap(), "null");
        for invalid in [b"not json".as_slice(), b"null trailing", b"{", &[0xff]] {
            assert!(
                json_response(invalid)
                    .unwrap_err()
                    .contains("invalid extension response")
            );
        }
    }
}
