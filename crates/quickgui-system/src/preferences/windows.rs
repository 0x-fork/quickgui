use std::{ffi::c_void, mem, ptr};

use windows_sys::Win32::{
    Graphics::{
        Dwm::DwmGetColorizationColor,
        Gdi::{
            COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_HOTLIGHT,
            COLOR_WINDOW, COLOR_WINDOWTEXT, GetSysColor,
        },
    },
    System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
    UI::{
        Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
        WindowsAndMessaging::{
            SPI_GETCLIENTAREAANIMATION, SPI_GETHIGHCONTRAST, SPI_GETSCREENREADER,
            SystemParametersInfoW,
        },
    },
};

use super::{
    ColorScheme, PermissionKind, PermissionStatus, Result, SystemColor, SystemColorRole,
    SystemPreferences,
};
use crate::SystemIntegrationError;

const PERSONALIZE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

pub(super) fn system_preferences() -> Result<SystemPreferences> {
    let color_scheme = match read_registry_dword(PERSONALIZE_KEY, "AppsUseLightTheme") {
        Some(0) => ColorScheme::Dark,
        Some(_) => ColorScheme::Light,
        None => ColorScheme::Unknown,
    };
    let high_contrast = high_contrast();
    Ok(SystemPreferences::new(
        color_scheme,
        system_parameter_bool(SPI_GETCLIENTAREAANIMATION).map(|enabled| !enabled),
        read_registry_dword(PERSONALIZE_KEY, "EnableTransparency").map(|enabled| enabled == 0),
        high_contrast,
        high_contrast,
        None,
        system_parameter_bool(SPI_GETSCREENREADER),
        None,
    )
    .with_forced_colors(high_contrast)
    .with_system_color(SystemColorRole::Accent, accent_color())
    .with_system_color(
        SystemColorRole::Highlight,
        Some(system_color(COLOR_HIGHLIGHT)),
    )
    .with_system_color(
        SystemColorRole::HighlightText,
        Some(system_color(COLOR_HIGHLIGHTTEXT)),
    )
    .with_system_color(
        SystemColorRole::WindowBackground,
        Some(system_color(COLOR_WINDOW)),
    )
    .with_system_color(
        SystemColorRole::WindowText,
        Some(system_color(COLOR_WINDOWTEXT)),
    )
    .with_system_color(
        SystemColorRole::ControlBackground,
        Some(system_color(COLOR_BTNFACE)),
    )
    .with_system_color(
        SystemColorRole::ControlText,
        Some(system_color(COLOR_BTNTEXT)),
    )
    .with_system_color(SystemColorRole::Link, Some(system_color(COLOR_HOTLIGHT))))
}

fn system_color(index: i32) -> SystemColor {
    // COLORREF stores red in the low byte.
    let color = unsafe { GetSysColor(index) };
    SystemColor::rgb(
        (color & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        ((color >> 16) & 0xff) as u8,
    )
}

fn accent_color() -> Option<SystemColor> {
    let mut color = 0_u32;
    let mut opaque = 0_i32;
    // DWM returns the color as ARGB.
    (unsafe { DwmGetColorizationColor(&mut color, &mut opaque) } >= 0).then(|| {
        SystemColor::rgba(
            ((color >> 16) & 0xff) as u8,
            ((color >> 8) & 0xff) as u8,
            (color & 0xff) as u8,
            if opaque != 0 {
                255
            } else {
                ((color >> 24) & 0xff) as u8
            },
        )
    })
}

pub(super) fn permission_status(kind: PermissionKind) -> Result<PermissionStatus> {
    Ok(match kind {
        PermissionKind::ScreenRecording | PermissionKind::Accessibility => {
            PermissionStatus::Granted
        }
        PermissionKind::Camera | PermissionKind::Microphone => PermissionStatus::Unknown,
    })
}

pub(super) fn request_permission(
    _kind: PermissionKind,
    _completion: impl FnOnce(Result<PermissionStatus>) + Send + 'static,
) -> Result<()> {
    Err(SystemIntegrationError::Unsupported)
}

fn high_contrast() -> Option<bool> {
    let mut value = HIGHCONTRASTW {
        cbSize: mem::size_of::<HIGHCONTRASTW>() as u32,
        dwFlags: 0,
        lpszDefaultScheme: ptr::null_mut(),
    };
    (unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            value.cbSize,
            (&mut value as *mut HIGHCONTRASTW).cast(),
            0,
        )
    } != 0)
        .then_some(value.dwFlags & HCF_HIGHCONTRASTON != 0)
}

fn system_parameter_bool(action: u32) -> Option<bool> {
    let mut value = 0_i32;
    (unsafe { SystemParametersInfoW(action, 0, (&mut value as *mut i32).cast::<c_void>(), 0) } != 0)
        .then_some(value != 0)
}

fn read_registry_dword(subkey: &str, value_name: &str) -> Option<u32> {
    let subkey: Vec<u16> = subkey.encode_utf16().chain([0]).collect();
    let value_name: Vec<u16> = value_name.encode_utf16().chain([0]).collect();
    let mut value = 0_u32;
    let mut bytes = mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut bytes,
        )
    };
    (status == 0 && bytes as usize == mem::size_of::<u32>()).then_some(value)
}
