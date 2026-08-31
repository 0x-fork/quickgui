use super::*;

use tray_icon::menu::{
    CheckMenuItem, ContextMenu, Icon, IconMenuItem, IsMenuItem, Menu as NativeMenu,
    MenuItem as NativeMenuItem, PredefinedMenuItem, Submenu,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Owns the Win32 menu bar and its attachments. `muda` detaches every HWND when dropped.
pub(super) struct WindowsMenuHost {
    menu: NativeMenu,
}

impl WindowsMenuHost {
    pub(super) fn new(menus: &[Menu], proxy: EventLoopProxy<RuntimeEvent>) -> Result<Self, String> {
        crate::menu::validate_menus(menus).map_err(|error| error.to_string())?;
        tray::install_native_menu_handlers(proxy);
        let menu = NativeMenu::new();
        let mut next_action = 0_usize;
        for declared in menus {
            let submenu = build_submenu(declared, &mut next_action, MenuIdScope::Application)?;
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

    pub(super) fn detach(&self, window: &Window) -> Result<(), String> {
        let handle = window.window_handle().map_err(|error| error.to_string())?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err("a Win32 application menu requires a Win32 window handle".to_owned());
        };
        unsafe { self.menu.remove_for_hwnd(handle.hwnd.get()) }.map_err(|error| error.to_string())
    }
}

#[derive(Clone, Copy)]
enum MenuIdScope {
    Application,
    Popup(u64),
}

pub(super) fn show_popup_menu(
    menu: &Menu,
    popup_id: u64,
    window: &Window,
    position: Option<Point>,
) -> Result<bool, String> {
    crate::menu::validate_menus(std::slice::from_ref(menu)).map_err(|error| error.to_string())?;
    let mut next_action = 0;
    let native = build_submenu(menu, &mut next_action, MenuIdScope::Popup(popup_id))?;
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("a Win32 popup menu requires a Win32 window handle".to_owned());
    };
    let position = position.map(|position| {
        tray_icon::menu::dpi::Position::Logical(tray_icon::menu::dpi::LogicalPosition::new(
            f64::from(position.x),
            f64::from(position.y),
        ))
    });
    Ok(unsafe { native.show_context_menu_for_hwnd(handle.hwnd.get(), position) })
}

fn build_submenu(
    menu: &Menu,
    next_action: &mut usize,
    scope: MenuIdScope,
) -> Result<Submenu, String> {
    let native = Submenu::new(menu.name.as_ref(), !menu.disabled);
    append_items(&native, &menu.items, next_action, scope)?;
    Ok(native)
}

fn append_items(
    parent: &Submenu,
    items: &[crate::MenuItem],
    next_action: &mut usize,
    scope: MenuIdScope,
) -> Result<(), String> {
    for item in items {
        match item {
            crate::MenuItem::Action {
                name,
                checked,
                mark,
                icon,
                disabled,
                ..
            }
            | crate::MenuItem::Role {
                name,
                checked,
                mark,
                icon,
                disabled,
                ..
            } => {
                let id = match scope {
                    MenuIdScope::Application => format!("quickgui-app-menu-{next_action}"),
                    MenuIdScope::Popup(popup) => {
                        format!("quickgui-popup-menu-{popup}-action-{next_action}")
                    }
                };
                *next_action += 1;
                if *mark != crate::MenuItemMark::None {
                    let position = parent.items().len();
                    append(
                        parent,
                        &CheckMenuItem::with_id(id, name.as_ref(), !disabled, *checked, None),
                    )?;
                    if *mark == crate::MenuItemMark::Radio {
                        set_radio_style(parent, position)?;
                    }
                } else if let Some(icon) = icon {
                    let icon = Icon::from_rgba(icon.rgba().to_vec(), icon.width(), icon.height())
                        .map_err(|error| error.to_string())?;
                    append(
                        parent,
                        &IconMenuItem::with_id(id, name.as_ref(), !disabled, Some(icon), None),
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
                append(parent, &build_submenu(menu, next_action, scope)?)?;
            }
            // The Services menu is an operating-system-owned macOS concept.
            crate::MenuItem::SystemMenu(_) => {}
        }
    }
    Ok(())
}

fn set_radio_style(parent: &Submenu, position: usize) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMenuItemInfoW, MENUITEMINFOW, MFT_RADIOCHECK, MIIM_FTYPE, SetMenuItemInfoW,
    };

    let position =
        u32::try_from(position).map_err(|_| "native menu position overflow".to_owned())?;
    let mut info = MENUITEMINFOW {
        cbSize: u32::try_from(std::mem::size_of::<MENUITEMINFOW>())
            .expect("MENUITEMINFOW size fits u32"),
        fMask: MIIM_FTYPE,
        ..unsafe { std::mem::zeroed() }
    };
    let menu = parent.hpopupmenu() as _;
    if unsafe { GetMenuItemInfoW(menu, position, 1, &mut info) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    info.fType |= MFT_RADIOCHECK;
    if unsafe { SetMenuItemInfoW(menu, position, 1, &info) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn append(parent: &Submenu, item: &dyn IsMenuItem) -> Result<(), String> {
    parent.append(item).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestAction;

    #[test]
    fn action_ids_follow_core_collection_order() {
        let menus = [Menu::new("File")
            .action("Open", TestAction)
            .submenu(Menu::new("More").action("Quit", TestAction))];
        let mut next = 0;
        let _ = build_submenu(&menus[0], &mut next, MenuIdScope::Application).unwrap();
        assert_eq!(next, collect_menu_actions(&menus).len());
    }
}
