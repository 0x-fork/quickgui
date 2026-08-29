use std::{path::PathBuf, sync::Arc};

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use quickgui::{TrayIconImage, TrayIconOptions, TrayMenuItem};
use serde::Deserialize;

const MAX_TRAY_MENU_JSON_BYTES: usize = 256 * 1024;

#[napi(object)]
pub struct NativeTrayIconOptions {
    pub id: u32,
    /// Encoded image bytes, or raw RGBA8 when width and height are both supplied.
    pub icon_data: Option<Buffer>,
    /// Image path used when iconData is omitted.
    pub icon_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub tooltip: Option<String>,
    pub title: Option<String>,
    pub icon_is_template: Option<bool>,
    pub menu_on_left_click: Option<bool>,
    pub visible: Option<bool>,
    /// Bounded JSON encoding of the declarative tray menu.
    pub menu: String,
}

pub(crate) enum TrayAction {
    Set(TrayIconOptions),
    Remove(u32),
    ShowMenu(u32),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum NativeTrayMenuItem {
    Action {
        id: u32,
        label: String,
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        checked: bool,
    },
    Separator,
    Submenu {
        label: String,
        #[serde(default = "default_true")]
        enabled: bool,
        items: Vec<NativeTrayMenuItem>,
    },
}

fn default_true() -> bool {
    true
}

pub(super) fn tray_options(native: NativeTrayIconOptions) -> Result<TrayIconOptions, String> {
    let image = match (native.icon_data, native.icon_path) {
        (Some(data), None) => match (native.width, native.height) {
            (Some(width), Some(height)) => {
                TrayIconImage::from_rgba(Arc::<[u8]>::from(data.as_ref()), width, height)
            }
            (None, None) => TrayIconImage::from_encoded(data.as_ref()),
            _ => {
                return Err(
                    "raw tray icon data requires both width and height, or neither for encoded data"
                        .to_owned(),
                );
            }
        },
        (None, Some(path)) if native.width.is_none() && native.height.is_none() => {
            TrayIconImage::from_path(PathBuf::from(path))
        }
        (Some(_), Some(_)) => {
            return Err("a tray icon accepts iconData or iconPath, but not both".to_owned());
        }
        _ => return Err("a tray icon requires iconData or iconPath".to_owned()),
    }
    .map_err(|error| error.to_string())?;
    let menu = tray_menu(&native.menu)?;
    let mut options = TrayIconOptions::new(native.id, image);
    options.tooltip = native.tooltip.map(Arc::from);
    options.title = native.title.map(Arc::from);
    options.icon_is_template = native.icon_is_template.unwrap_or(false);
    options.menu_on_left_click = native.menu_on_left_click.unwrap_or(true);
    options.visible = native.visible.unwrap_or(true);
    options.menu = menu;
    Ok(options)
}

fn tray_menu(json: &str) -> Result<Vec<TrayMenuItem>, String> {
    if json.len() > MAX_TRAY_MENU_JSON_BYTES {
        return Err("the native tray menu exceeds 256 KiB".to_owned());
    }
    let items: Vec<NativeTrayMenuItem> =
        serde_json::from_str(json).map_err(|error| format!("invalid tray menu: {error}"))?;
    Ok(convert_items(items))
}

fn convert_items(items: Vec<NativeTrayMenuItem>) -> Vec<TrayMenuItem> {
    items
        .into_iter()
        .map(|item| match item {
            NativeTrayMenuItem::Action {
                id,
                label,
                enabled,
                checked,
            } => TrayMenuItem::action(id, label)
                .enabled(enabled)
                .checked(checked),
            NativeTrayMenuItem::Separator => TrayMenuItem::separator(),
            NativeTrayMenuItem::Submenu {
                label,
                enabled,
                items,
            } => TrayMenuItem::submenu(label, convert_items(items)).enabled(enabled),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_tray_menu() {
        let menu = tray_menu(
            r#"[{"type":"submenu","label":"More","items":[{"type":"action","id":1,"label":"Quit"}]}]"#,
        )
        .unwrap();
        assert_eq!(menu.len(), 1);
    }

    #[test]
    fn rejects_partial_raw_dimensions() {
        let options = NativeTrayIconOptions {
            id: 1,
            icon_data: Some(Buffer::from(vec![0, 0, 0, 0])),
            icon_path: None,
            width: Some(1),
            height: None,
            tooltip: None,
            title: None,
            icon_is_template: None,
            menu_on_left_click: None,
            visible: None,
            menu: "[]".to_owned(),
        };
        assert!(tray_options(options).unwrap_err().contains("both width"));
    }
}
