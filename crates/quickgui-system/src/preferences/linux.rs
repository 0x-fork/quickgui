use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{OwnedValue, Value},
};

use super::{
    ColorScheme, PermissionKind, PermissionStatus, Result, SystemColor, SystemColorRole,
    SystemPreferences,
};
use crate::SystemIntegrationError;

const APPEARANCE_NAMESPACE: &str = "org.freedesktop.appearance";

pub(super) fn system_preferences() -> Result<SystemPreferences> {
    let color_scheme = match portal_u32("color-scheme") {
        Some(1) => ColorScheme::Dark,
        Some(2) => ColorScheme::Light,
        _ => environment_color_scheme(),
    };
    let contrast = portal_u32("contrast").map(|value| value == 1);
    Ok(SystemPreferences::new(
        color_scheme,
        portal_u32("reduced-motion").map(|value| value == 1),
        None,
        contrast,
        contrast,
        None,
        accessibility_bus_enabled(),
        None,
    )
    .with_forced_colors(contrast)
    .with_system_color(SystemColorRole::Accent, portal_color("accent-color")))
}

pub(super) fn permission_status(_kind: PermissionKind) -> Result<PermissionStatus> {
    Ok(PermissionStatus::Unknown)
}

pub(super) fn request_permission(
    _kind: PermissionKind,
    _completion: impl FnOnce(Result<PermissionStatus>) + Send + 'static,
) -> Result<()> {
    Err(SystemIntegrationError::Unsupported)
}

fn portal_u32(key: &str) -> Option<u32> {
    let value = portal_value(key)?;
    if let Ok(value) = value.downcast_ref::<Value>() {
        u32::try_from(value.try_to_owned().ok()?).ok()
    } else {
        u32::try_from(value).ok()
    }
}

fn portal_color(key: &str) -> Option<SystemColor> {
    let value = portal_value(key)?;
    let components = if let Ok(value) = value.downcast_ref::<Value>() {
        <(f64, f64, f64)>::try_from(value.try_to_owned().ok()?).ok()?
    } else {
        <(f64, f64, f64)>::try_from(value).ok()?
    };
    let component = |value: f64| {
        value
            .is_finite()
            .then(|| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
    };
    Some(SystemColor::rgb(
        component(components.0)?,
        component(components.1)?,
        component(components.2)?,
    ))
}

fn portal_value(key: &str) -> Option<OwnedValue> {
    let connection = Connection::session().ok()?;
    let settings = Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    )
    .ok()?;
    settings.call("Read", &(APPEARANCE_NAMESPACE, key)).ok()
}

fn environment_color_scheme() -> ColorScheme {
    std::env::var("GTK_THEME")
        .ok()
        .filter(|theme| theme.to_ascii_lowercase().contains("dark"))
        .map_or(ColorScheme::Unknown, |_| ColorScheme::Dark)
}

fn accessibility_bus_enabled() -> Option<bool> {
    let connection = Connection::session().ok()?;
    let bus = Proxy::new(&connection, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus").ok()?;
    bus.get_property("IsEnabled").ok()
}
