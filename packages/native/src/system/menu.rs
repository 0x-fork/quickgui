use std::collections::HashSet;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use quickgui::{Menu, MenuIcon, MenuItem, OsAction, SystemMenuType};
use serde::Deserialize;

const MAX_MENU_JSON_BYTES: usize = 1024 * 1024;
const MAX_MENU_ITEMS: usize = 4_096;
const MAX_MENU_DEPTH: usize = 16;
const MAX_MENU_TEXT_BYTES: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeApplicationMenu {
    label: String,
    #[serde(default = "default_true")]
    enabled: bool,
    items: Vec<NativeApplicationMenuItem>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum NativeApplicationMenuItem {
    Action {
        id: u32,
        label: String,
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        checked: bool,
        mark: Option<String>,
        role: Option<String>,
        icon: Option<NativeMenuIcon>,
    },
    Role {
        label: String,
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        checked: bool,
        mark: Option<String>,
        role: String,
        icon: Option<NativeMenuIcon>,
    },
    Separator,
    Submenu {
        label: String,
        #[serde(default = "default_true")]
        enabled: bool,
        items: Vec<NativeApplicationMenuItem>,
    },
    SystemMenu {
        label: String,
        menu: String,
    },
    Services {
        label: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeMenuIcon {
    path: Option<String>,
    data_base64: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

fn default_true() -> bool {
    true
}

pub(crate) fn application_menus(json: &str) -> Result<Vec<Menu>, String> {
    if json.len() > MAX_MENU_JSON_BYTES {
        return Err("the native application menu exceeds 1 MiB".to_owned());
    }
    let definitions: Vec<NativeApplicationMenu> =
        serde_json::from_str(json).map_err(|error| format!("invalid application menu: {error}"))?;
    let mut ids = HashSet::new();
    let mut item_count = 0_usize;
    let mut text_bytes = 0_usize;

    definitions
        .into_iter()
        .map(|definition| {
            add_text(&definition.label, &mut text_bytes)?;
            Ok(Menu::new(definition.label)
                .items(convert_items(
                    definition.items,
                    1,
                    &mut ids,
                    &mut item_count,
                    &mut text_bytes,
                )?)
                .disabled(!definition.enabled))
        })
        .collect()
}

fn add_text(value: &str, total: &mut usize) -> Result<(), String> {
    if value.is_empty() || value.contains('\0') || value.len() > 16 * 1024 {
        return Err("native menu labels must be nonempty, bounded, and NUL-free".to_owned());
    }
    *total = total.saturating_add(value.len());
    if *total > MAX_MENU_TEXT_BYTES {
        return Err("the native application menu contains too much text".to_owned());
    }
    Ok(())
}

fn convert_items(
    items: Vec<NativeApplicationMenuItem>,
    depth: usize,
    ids: &mut HashSet<u32>,
    item_count: &mut usize,
    text_bytes: &mut usize,
) -> Result<Vec<MenuItem>, String> {
    if depth > MAX_MENU_DEPTH {
        return Err("the native application menu is nested too deeply".to_owned());
    }
    let mut converted = Vec::with_capacity(items.len());
    for item in items {
        *item_count += 1;
        if *item_count > MAX_MENU_ITEMS {
            return Err("the native application menu contains too many items".to_owned());
        }
        converted.push(match item {
            NativeApplicationMenuItem::Action {
                id,
                label,
                enabled,
                checked,
                mark,
                role,
                icon,
            } => {
                add_text(&label, text_bytes)?;
                if id == 0 || !ids.insert(id) {
                    return Err("native menu action ids must be nonzero and unique".to_owned());
                }
                let action = crate::NativeMenuAction(id);
                let item = match role.as_deref() {
                    None => MenuItem::action(label, action),
                    Some(role) => MenuItem::os_action(label, action, os_action(role)?),
                };
                decorate_item(item, checked, mark.as_deref(), icon)?.enabled(enabled)
            }
            NativeApplicationMenuItem::Role {
                label,
                enabled,
                checked,
                mark,
                role,
                icon,
            } => {
                add_text(&label, text_bytes)?;
                let item = MenuItem::role(label, os_action(&role)?);
                decorate_item(item, checked, mark.as_deref(), icon)?.enabled(enabled)
            }
            NativeApplicationMenuItem::Separator => MenuItem::separator(),
            NativeApplicationMenuItem::Submenu {
                label,
                enabled,
                items,
            } => {
                add_text(&label, text_bytes)?;
                MenuItem::submenu(
                    Menu::new(label)
                        .items(convert_items(
                            items,
                            depth + 1,
                            ids,
                            item_count,
                            text_bytes,
                        )?)
                        .disabled(!enabled),
                )
            }
            NativeApplicationMenuItem::SystemMenu { label, menu } => {
                add_text(&label, text_bytes)?;
                MenuItem::os_submenu(
                    label,
                    match menu.as_str() {
                        "services" => SystemMenuType::Services,
                        "window" => SystemMenuType::Window,
                        "help" => SystemMenuType::Help,
                        value => return Err(format!("unknown native system menu `{value}`")),
                    },
                )
            }
            NativeApplicationMenuItem::Services { label } => {
                add_text(&label, text_bytes)?;
                MenuItem::os_submenu(label, SystemMenuType::Services)
            }
        });
    }
    Ok(converted)
}

fn os_action(role: &str) -> Result<OsAction, String> {
    match role {
        "cut" => Ok(OsAction::Cut),
        "copy" => Ok(OsAction::Copy),
        "paste" => Ok(OsAction::Paste),
        "select-all" => Ok(OsAction::SelectAll),
        "undo" => Ok(OsAction::Undo),
        "redo" => Ok(OsAction::Redo),
        "about" => Ok(OsAction::About),
        "hide-application" => Ok(OsAction::HideApplication),
        "hide-other-applications" => Ok(OsAction::HideOtherApplications),
        "show-all-applications" => Ok(OsAction::ShowAllApplications),
        "quit" => Ok(OsAction::Quit),
        "close-window" => Ok(OsAction::CloseWindow),
        "minimize-window" => Ok(OsAction::MinimizeWindow),
        "zoom-window" => Ok(OsAction::ZoomWindow),
        "toggle-fullscreen" => Ok(OsAction::ToggleFullscreen),
        "bring-all-to-front" => Ok(OsAction::BringAllToFront),
        "show-help" => Ok(OsAction::ShowHelp),
        value => Err(format!("unknown native menu role `{value}`")),
    }
}

fn decorate_item(
    mut item: MenuItem,
    checked: bool,
    mark: Option<&str>,
    icon: Option<NativeMenuIcon>,
) -> Result<MenuItem, String> {
    item = match mark.unwrap_or(if checked { "check" } else { "none" }) {
        "none" if !checked => item,
        "none" => return Err("a checked menu item cannot use the `none` mark".to_owned()),
        "check" => item.checked(checked),
        "radio" => item.radio(checked),
        value => return Err(format!("unknown native menu mark `{value}`")),
    };
    if let Some(icon) = icon {
        item = item.icon(native_menu_icon(icon)?);
    }
    Ok(item)
}

fn native_menu_icon(icon: NativeMenuIcon) -> Result<MenuIcon, String> {
    match (icon.path, icon.data_base64) {
        (Some(path), None) if icon.width.is_none() && icon.height.is_none() => {
            MenuIcon::open(path).map_err(|error| error.to_string())
        }
        (None, Some(data)) => {
            let data = STANDARD
                .decode(data)
                .map_err(|_| "native menu icon data is not valid base64".to_owned())?;
            match (icon.width, icon.height) {
                (Some(width), Some(height)) => MenuIcon::from_rgba(width, height, data),
                (None, None) => MenuIcon::decode(data),
                _ => {
                    return Err(
                        "raw native menu icon data requires both width and height".to_owned()
                    );
                }
            }
            .map_err(|error| error.to_string())
        }
        _ => Err("a native menu icon requires exactly one path or data source".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_json_rejects_duplicate_action_ids() {
        let json = r#"[{"label":"App","items":[{"type":"action","id":1,"label":"One"},{"type":"action","id":1,"label":"Two"}]}]"#;
        assert!(application_menus(json).unwrap_err().contains("unique"));
    }

    #[test]
    fn menu_json_accepts_nested_and_system_items() {
        let json = r#"[{"label":"App","items":[{"type":"submenu","label":"Edit","items":[{"type":"action","id":1,"label":"Copy","role":"copy"}]},{"type":"system-menu","label":"Services","menu":"services"}]}]"#;
        assert_eq!(application_menus(json).expect("valid menu").len(), 1);
    }
}
