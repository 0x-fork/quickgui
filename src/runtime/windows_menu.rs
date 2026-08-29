use super::*;

use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu as NativeMenu, MenuItem as NativeMenuItem};
use tray_icon::menu::{PredefinedMenuItem, Submenu};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Owns the Win32 menu bar and its attachments. `muda` detaches every HWND when dropped.
pub(super) struct WindowsMenuHost {
    menu: NativeMenu,
}

impl WindowsMenuHost {
    pub(super) fn new(menus: &[Menu], proxy: EventLoopProxy<RuntimeEvent>) -> Result<Self, String> {
        tray::install_native_menu_handlers(proxy);
        let menu = NativeMenu::new();
        let mut next_action = 0_usize;
        for declared in menus {
            let submenu = build_submenu(declared, &mut next_action)?;
            menu.append(&submenu).map_err(|error| error.to_string())?;
        }
        Ok(Self { menu })
    }

    pub(super) fn attach(&self, window: &Window) -> Result<(), String> {
        let handle = window.window_handle().map_err(|error| error.to_string())?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err("a Win32 application menu requires a Win32 window handle".to_owned());
        };
        unsafe { self.menu.init_for_hwnd(handle.hwnd.get()) }.map_err(|error| error.to_string())
    }
}

fn build_submenu(menu: &Menu, next_action: &mut usize) -> Result<Submenu, String> {
    let native = Submenu::new(menu.name.as_ref(), !menu.disabled);
    append_items(&native, &menu.items, next_action)?;
    Ok(native)
}

fn append_items(
    parent: &Submenu,
    items: &[crate::MenuItem],
    next_action: &mut usize,
) -> Result<(), String> {
    for item in items {
        match item {
            crate::MenuItem::Action {
                name,
                checked,
                disabled,
                ..
            } => {
                let id = format!("quickgui-app-menu-{next_action}");
                *next_action += 1;
                if *checked {
                    append(
                        parent,
                        &CheckMenuItem::with_id(id, name.as_ref(), !disabled, true, None),
                    )?;
                } else {
                    append(
                        parent,
                        &NativeMenuItem::with_id(id, name.as_ref(), !disabled, None),
                    )?;
                }
            }
            crate::MenuItem::Separator => append(parent, &PredefinedMenuItem::separator())?,
            crate::MenuItem::Submenu(menu) => {
                append(parent, &build_submenu(menu, next_action)?)?;
            }
            // The Services menu is an operating-system-owned macOS concept.
            crate::MenuItem::SystemMenu(_) => {}
        }
    }
    Ok(())
}

fn append(parent: &Submenu, item: &dyn IsMenuItem) -> Result<(), String> {
    parent.append(item).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestAction;

    #[test]
    fn action_ids_follow_core_collection_order() {
        let menus = [Menu::new("File")
            .action("Open", TestAction)
            .submenu(Menu::new("More").action("Quit", TestAction))];
        let mut next = 0;
        let _ = build_submenu(&menus[0], &mut next).unwrap();
        assert_eq!(next, collect_menu_actions(&menus).len());
    }
}
