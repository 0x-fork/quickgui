use std::sync::Arc;

use crate::{Action, AnyAction};

/// A declarative application menu projected to the platform's native menu system.
///
/// Menus own typed actions rather than callbacks. Selecting an action item dispatches through the
/// same focused action path as a key binding, so keyboard shortcuts, menus, and programmatic
/// commands share one command implementation.
#[derive(Clone, Debug)]
pub struct Menu {
    pub name: Arc<str>,
    pub items: Vec<MenuItem>,
    pub disabled: bool,
}

impl Menu {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            items: Vec::new(),
            disabled: false,
        }
    }

    /// Replace this menu's items, matching GPUI's menu construction shape.
    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Append one item for fluent, web-like menu construction.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn action<A: Action>(self, name: impl Into<Arc<str>>, action: A) -> Self {
        self.item(MenuItem::action(name, action))
    }

    pub fn separator(self) -> Self {
        self.item(MenuItem::separator())
    }

    pub fn submenu(self, menu: Menu) -> Self {
        self.item(MenuItem::submenu(menu))
    }

    pub fn push(&mut self, item: MenuItem) {
        self.items.push(item);
    }
}

/// A menu populated and managed by the operating system.
#[derive(Clone, Debug)]
pub struct OsMenu {
    pub name: Arc<str>,
    pub menu_type: SystemMenuType,
}

/// Native menus with operating-system-owned contents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemMenuType {
    /// The macOS Services menu.
    Services,
}

/// Commands that should first follow the operating system's native responder chain.
///
/// If no native responder accepts the selector, QuickGUI falls back to dispatching the associated
/// typed action through the focused retained tree. This makes one Edit menu work for both embedded
/// NSView controls and GPU-rendered QuickGUI inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OsAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
    Undo,
    Redo,
}

/// One action, separator, or nested menu in a Menu.
#[derive(Clone, Debug)]
pub enum MenuItem {
    Separator,
    Submenu(Menu),
    SystemMenu(OsMenu),
    Action {
        name: Arc<str>,
        action: AnyAction,
        os_action: Option<OsAction>,
        checked: bool,
        disabled: bool,
    },
}

impl MenuItem {
    pub fn action<A: Action>(name: impl Into<Arc<str>>, action: A) -> Self {
        Self::Action {
            name: name.into(),
            action: AnyAction::new(action),
            os_action: None,
            checked: false,
            disabled: false,
        }
    }

    pub fn os_action<A: Action>(name: impl Into<Arc<str>>, action: A, os_action: OsAction) -> Self {
        Self::Action {
            name: name.into(),
            action: AnyAction::new(action),
            os_action: Some(os_action),
            checked: false,
            disabled: false,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }

    pub fn submenu(menu: Menu) -> Self {
        Self::Submenu(menu)
    }

    pub fn os_submenu(name: impl Into<Arc<str>>, menu_type: SystemMenuType) -> Self {
        Self::SystemMenu(OsMenu {
            name: name.into(),
            menu_type,
        })
    }

    pub fn checked(mut self, value: bool) -> Self {
        if let Self::Action { checked, .. } = &mut self {
            *checked = value;
        }
        self
    }

    pub fn is_checked(&self) -> bool {
        matches!(self, Self::Action { checked: true, .. })
    }

    pub fn disabled(mut self, value: bool) -> Self {
        match &mut self {
            Self::Action { disabled, .. } => *disabled = value,
            Self::Submenu(menu) => menu.disabled = value,
            Self::Separator | Self::SystemMenu(_) => {}
        }
        self
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Action { disabled, .. } => *disabled,
            Self::Submenu(menu) => menu.disabled,
            Self::Separator | Self::SystemMenu(_) => false,
        }
    }

    /// Convenience inverse of disabled.
    pub fn enabled(self, value: bool) -> Self {
        self.disabled(!value)
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }
}

pub(crate) struct MenuAction {
    pub action: AnyAction,
    pub os_action: Option<OsAction>,
    pub disabled: bool,
    pub checked: bool,
}

pub(crate) fn collect_menu_actions(menus: &[Menu]) -> Vec<MenuAction> {
    fn collect(menu: &Menu, actions: &mut Vec<MenuAction>) {
        for item in &menu.items {
            match item {
                MenuItem::Action {
                    action,
                    os_action,
                    disabled,
                    checked,
                    ..
                } => actions.push(MenuAction {
                    action: action.clone(),
                    os_action: *os_action,
                    disabled: *disabled,
                    checked: *checked,
                }),
                MenuItem::Submenu(menu) => collect(menu, actions),
                MenuItem::Separator | MenuItem::SystemMenu(_) => {}
            }
        }
    }

    let mut actions = Vec::new();
    for menu in menus {
        collect(menu, &mut actions);
    }
    actions
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct OpenFile;

    #[derive(Clone, Debug, PartialEq)]
    struct ToggleSidebar;

    #[test]
    fn preserves_nested_action_order_and_state() {
        let menus = [Menu::new("File")
            .action("Open", OpenFile)
            .submenu(
                Menu::new("View").item(MenuItem::action("Sidebar", ToggleSidebar).checked(true)),
            )
            .item(MenuItem::action("Disabled", OpenFile).disabled(true))];

        let actions = collect_menu_actions(&menus);
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].action.type_id(), TypeId::of::<OpenFile>());
        assert_eq!(actions[1].action.type_id(), TypeId::of::<ToggleSidebar>());
        assert!(actions[1].checked);
        assert!(actions[2].disabled);
    }

    #[test]
    fn gpui_shaped_builders_cover_disabled_and_system_items() {
        let menu = Menu::new("Application")
            .items([
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
            ])
            .disabled(true);
        assert_eq!(menu.name.as_ref(), "Application");
        assert_eq!(menu.items.len(), 2);
        assert!(menu.disabled);
        assert!(MenuItem::submenu(Menu::new("Nested").disabled(true)).is_disabled());
    }
}
