use std::collections::HashSet;

use quickgui::{Menu, MenuItem, OsAction};
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
        role: Option<String>,
    },
    Separator,
    Submenu {
        label: String,
        #[serde(default = "default_true")]
        enabled: bool,
        items: Vec<NativeApplicationMenuItem>,
    },
    Services {
        label: String,
    },
}

fn default_true() -> bool {
    true
}

pub(super) fn application_menus(json: &str) -> Result<Vec<Menu>, String> {
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
                role,
            } => {
                add_text(&label, text_bytes)?;
                if id == 0 || !ids.insert(id) {
                    return Err("native menu action ids must be nonzero and unique".to_owned());
                }
                let action = crate::NativeMenuAction(id);
                let item = match role.as_deref() {
                    None => MenuItem::action(label, action),
                    Some("cut") => MenuItem::os_action(label, action, OsAction::Cut),
                    Some("copy") => MenuItem::os_action(label, action, OsAction::Copy),
                    Some("paste") => MenuItem::os_action(label, action, OsAction::Paste),
                    Some("select-all") => MenuItem::os_action(label, action, OsAction::SelectAll),
                    Some("undo") => MenuItem::os_action(label, action, OsAction::Undo),
                    Some("redo") => MenuItem::os_action(label, action, OsAction::Redo),
                    Some(role) => return Err(format!("unknown native menu role `{role}`")),
                };
                item.checked(checked).enabled(enabled)
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
            NativeApplicationMenuItem::Services { label } => {
                add_text(&label, text_bytes)?;
                MenuItem::os_submenu(label, quickgui::SystemMenuType::Services)
            }
        });
    }
    Ok(converted)
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
        let json = r#"[{"label":"App","items":[{"type":"submenu","label":"Edit","items":[{"type":"action","id":1,"label":"Copy","role":"copy"}]},{"type":"services","label":"Services"}]}]"#;
        assert_eq!(application_menus(json).expect("valid menu").len(), 1);
    }
}
