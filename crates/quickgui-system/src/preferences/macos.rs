use std::sync::{Arc, Mutex};

use block2::RcBlock;
use core_foundation::{
    base::TCFType,
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    string::{CFString, CFStringRef},
};
use objc2::{
    msg_send,
    runtime::{AnyClass, Bool},
};
use objc2_app_kit::{NSColor, NSColorSpace, NSWorkspace};
use objc2_foundation::{NSBundle, NSString, NSUserDefaults};

use super::{
    ColorScheme, PermissionKind, PermissionStatus, Result, SystemColor, SystemColorRole,
    SystemPreferences,
};
use crate::{SystemIntegrationError, invalid};

#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

pub(super) fn system_preferences() -> Result<SystemPreferences> {
    let defaults = unsafe { NSUserDefaults::standardUserDefaults() };
    let interface_style =
        unsafe { defaults.stringForKey(&NSString::from_str("AppleInterfaceStyle")) };
    let color_scheme = if interface_style
        .as_ref()
        .is_some_and(|style| style.to_string().eq_ignore_ascii_case("dark"))
    {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    };
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let preferences = SystemPreferences::new(
        color_scheme,
        Some(unsafe { workspace.accessibilityDisplayShouldReduceMotion() }),
        Some(unsafe { workspace.accessibilityDisplayShouldReduceTransparency() }),
        Some(unsafe { workspace.accessibilityDisplayShouldIncreaseContrast() }),
        Some(unsafe { workspace.accessibilityDisplayShouldDifferentiateWithoutColor() }),
        Some(unsafe { workspace.accessibilityDisplayShouldInvertColors() }),
        Some(unsafe { workspace.isVoiceOverEnabled() }),
        Some(unsafe { workspace.isSwitchControlEnabled() }),
    );
    Ok(preferences
        .with_system_color(
            SystemColorRole::Accent,
            native_system_color(unsafe { NSColor::controlAccentColor() }),
        )
        .with_system_color(
            SystemColorRole::Highlight,
            native_system_color(unsafe { NSColor::selectedContentBackgroundColor() }),
        )
        .with_system_color(
            SystemColorRole::HighlightText,
            native_system_color(unsafe { NSColor::selectedTextColor() }),
        )
        .with_system_color(
            SystemColorRole::WindowBackground,
            native_system_color(unsafe { NSColor::windowBackgroundColor() }),
        )
        .with_system_color(
            SystemColorRole::WindowText,
            native_system_color(unsafe { NSColor::labelColor() }),
        )
        .with_system_color(
            SystemColorRole::ControlBackground,
            native_system_color(unsafe { NSColor::controlBackgroundColor() }),
        )
        .with_system_color(
            SystemColorRole::ControlText,
            native_system_color(unsafe { NSColor::controlTextColor() }),
        )
        .with_system_color(
            SystemColorRole::Link,
            native_system_color(unsafe { NSColor::linkColor() }),
        ))
}

fn native_system_color(color: objc2::rc::Retained<NSColor>) -> Option<SystemColor> {
    // SAFETY: AppKit returns value-like retained colors and component scalars. Converting dynamic
    // system colors into sRGB resolves them for the current effective appearance.
    let color = unsafe { color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()) }?;
    let component = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(SystemColor::rgba(
        component(unsafe { color.redComponent() }),
        component(unsafe { color.greenComponent() }),
        component(unsafe { color.blueComponent() }),
        component(unsafe { color.alphaComponent() }),
    ))
}

pub(super) fn permission_status(kind: PermissionKind) -> Result<PermissionStatus> {
    Ok(match kind {
        PermissionKind::Camera | PermissionKind::Microphone => av_permission_status(kind)?,
        PermissionKind::ScreenRecording => {
            if unsafe { CGPreflightScreenCaptureAccess() } {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            }
        }
        PermissionKind::Accessibility => {
            if unsafe { AXIsProcessTrusted() } {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            }
        }
    })
}

pub(super) fn request_permission(
    kind: PermissionKind,
    completion: impl FnOnce(Result<PermissionStatus>) + Send + 'static,
) -> Result<()> {
    match kind {
        PermissionKind::Camera | PermissionKind::Microphone => {
            let current = av_permission_status(kind)?;
            if current != PermissionStatus::NotDetermined {
                completion(Ok(current));
                return Ok(());
            }
            validate_usage_description(kind)?;
            request_av_permission(kind, completion)
        }
        PermissionKind::ScreenRecording => {
            let granted = unsafe { CGRequestScreenCaptureAccess() };
            completion(Ok(if granted {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            }));
            Ok(())
        }
        PermissionKind::Accessibility => {
            let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
            let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
            let granted = unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };
            completion(Ok(if granted {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            }));
            Ok(())
        }
    }
}

fn av_permission_status(kind: PermissionKind) -> Result<PermissionStatus> {
    let class = AnyClass::get("AVCaptureDevice").ok_or(SystemIntegrationError::Unsupported)?;
    let media_type = media_type(kind)?;
    let status: isize =
        unsafe { msg_send![class, authorizationStatusForMediaType: media_type.as_ref()] };
    Ok(match status {
        0 => PermissionStatus::NotDetermined,
        1 => PermissionStatus::Restricted,
        2 => PermissionStatus::Denied,
        3 => PermissionStatus::Granted,
        _ => PermissionStatus::Unknown,
    })
}

fn request_av_permission(
    kind: PermissionKind,
    completion: impl FnOnce(Result<PermissionStatus>) + Send + 'static,
) -> Result<()> {
    let class = AnyClass::get("AVCaptureDevice").ok_or(SystemIntegrationError::Unsupported)?;
    let media_type = media_type(kind)?;
    let completion = Arc::new(Mutex::new(Some(completion)));
    let block = RcBlock::new(move |granted: Bool| {
        if let Ok(mut completion) = completion.lock()
            && let Some(completion) = completion.take()
        {
            completion(Ok(if granted.as_bool() {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            }));
        }
    });
    unsafe {
        let _: () = msg_send![
            class,
            requestAccessForMediaType: media_type.as_ref(),
            completionHandler: &*block
        ];
    }
    Ok(())
}

fn media_type(kind: PermissionKind) -> Result<objc2::rc::Retained<NSString>> {
    match kind {
        PermissionKind::Camera => Ok(NSString::from_str("vide")),
        PermissionKind::Microphone => Ok(NSString::from_str("soun")),
        _ => Err(invalid(
            "only camera and microphone permissions have an AVFoundation media type",
        )),
    }
}

fn validate_usage_description(kind: PermissionKind) -> Result<()> {
    let key = match kind {
        PermissionKind::Camera => "NSCameraUsageDescription",
        PermissionKind::Microphone => "NSMicrophoneUsageDescription",
        _ => return Ok(()),
    };
    let key_string = NSString::from_str(key);
    if unsafe { NSBundle::mainBundle().objectForInfoDictionaryKey(&key_string) }.is_none() {
        return Err(invalid(format!(
            "the macOS application bundle must define {key} before requesting this permission"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_preferences_and_permission_statuses_are_queryable_without_prompting() {
        let preferences = system_preferences().unwrap();
        assert!(matches!(
            preferences.color_scheme(),
            ColorScheme::Light | ColorScheme::Dark
        ));
        assert!(preferences.reduce_motion().is_some());
        assert!(preferences.reduce_transparency().is_some());
        permission_status(PermissionKind::Camera).unwrap();
        permission_status(PermissionKind::Microphone).unwrap();
        permission_status(PermissionKind::ScreenRecording).unwrap();
        permission_status(PermissionKind::Accessibility).unwrap();
    }
}
