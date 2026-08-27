use std::cell::{Cell, RefCell};

use objc2::{
    ClassType, DeclaredClass, declare_class, msg_send_id,
    mutability::MainThreadOnly,
    rc::Retained,
    runtime::{AnyObject, NSObjectProtocol, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSEventModifierFlags, NSMenu,
    NSMenuDelegate, NSMenuItem,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSString};
use winit::event_loop::EventLoopProxy;

use crate::{
    Key, Keystroke, Menu, MenuItem, Modifiers, OsAction, SystemMenuType, runtime::RuntimeEvent,
};

struct MenuTargetIvars {
    proxy: EventLoopProxy<RuntimeEvent>,
    os_actions: RefCell<Vec<Option<OsAction>>>,
    native_focus_active: Cell<bool>,
}

declare_class!(
    struct QuickGuiMenuTarget;

    unsafe impl ClassType for QuickGuiMenuTarget {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "QuickGuiMenuTarget";
    }

    impl DeclaredClass for QuickGuiMenuTarget {
        type Ivars = MenuTargetIvars;
    }

    unsafe impl NSObjectProtocol for QuickGuiMenuTarget {}

    unsafe impl NSMenuDelegate for QuickGuiMenuTarget {
        #[method(menuWillOpen:)]
        fn menu_will_open(&self, _menu: &NSMenu) {
            let _ = self.ivars().proxy.send_event(RuntimeEvent::MenuWillOpen);
        }
    }

    unsafe impl QuickGuiMenuTarget {
        #[method(quickGuiPerformMenuAction:)]
        fn perform_menu_action(&self, sender: &NSMenuItem) {
            let tag = unsafe { sender.tag() };
            if let Ok(action_id) = usize::try_from(tag) {
                if self.ivars().native_focus_active.get()
                    && self
                    .ivars()
                    .os_actions
                    .borrow()
                    .get(action_id)
                    .copied()
                    .flatten()
                    .is_some_and(|action| perform_os_action(action, sender))
                {
                    return;
                }
                // The numeric id keeps the AppKit callback Send while typed actions remain cheap,
                // main-thread-only `Rc` values owned by the runtime.
                let _ = self
                    .ivars()
                    .proxy
                    .send_event(RuntimeEvent::MenuAction(action_id));
            }
        }
    }
);

impl QuickGuiMenuTarget {
    fn new(
        mtm: MainThreadMarker,
        proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Retained<QuickGuiMenuTarget> {
        let allocated = mtm.alloc().set_ivars(MenuTargetIvars {
            proxy,
            os_actions: RefCell::new(Vec::new()),
            native_focus_active: Cell::new(false),
        });
        unsafe { msg_send_id![super(allocated), init] }
    }
}

pub(crate) struct MacMenuItemState {
    pub disabled: bool,
    pub action_available: bool,
    pub checked: bool,
    pub shortcut: Option<Keystroke>,
}

/// Owns only the application-declared portion of the global macOS menu bar.
///
/// Winit owns the standard application menu. QuickGUI appends its root items and removes those
/// exact objects on drop, leaving Winit's About, Services, Hide, and Quit integration untouched.
pub(crate) struct MacMenuHost {
    main_menu: Retained<NSMenu>,
    root_items: Vec<Retained<NSMenuItem>>,
    action_items: Vec<Retained<NSMenuItem>>,
    fallback_close_item: Retained<NSMenuItem>,
    target: Retained<QuickGuiMenuTarget>,
}

impl MacMenuHost {
    pub fn new(menus: &[Menu], proxy: EventLoopProxy<RuntimeEvent>) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "native menus must be initialized on the AppKit main thread".to_owned()
        })?;
        let app = NSApplication::sharedApplication(mtm);
        let main_menu = unsafe { app.mainMenu() }
            .ok_or_else(|| "AppKit did not install an application menu bar".to_owned())?;
        let services_menu = unsafe { app.servicesMenu() };
        let target = QuickGuiMenuTarget::new(mtm, proxy);
        let mut root_items = Vec::with_capacity(menus.len() + 1);
        let mut action_items = Vec::new();
        let mut next_action_id = 0;
        let mut file_menu = None;

        for menu in menus {
            let submenu = build_menu(
                menu,
                mtm,
                &target,
                services_menu.as_deref(),
                &mut next_action_id,
                &mut action_items,
            );
            let root_item = menu_item(mtm, &menu.name, None, "");
            root_item.setSubmenu(Some(&submenu));
            unsafe { root_item.setEnabled(!menu.disabled) };
            main_menu.addItem(&root_item);
            root_items.push(root_item);
            if menu.name.eq_ignore_ascii_case("file") {
                file_menu = Some(submenu);
            }
        }

        let file_menu = file_menu.unwrap_or_else(|| {
            let submenu =
                unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("File")) };
            unsafe { submenu.setAutoenablesItems(false) };
            let delegate = ProtocolObject::from_ref(&*target);
            unsafe { submenu.setDelegate(Some(delegate)) };
            let root = menu_item(mtm, "File", None, "");
            root.setSubmenu(Some(&submenu));
            main_menu.addItem(&root);
            root_items.push(root);
            submenu
        });
        if unsafe { file_menu.numberOfItems() } > 0 {
            file_menu.addItem(&NSMenuItem::separatorItem(mtm));
        }
        let fallback_close_item = menu_item(mtm, "Close Window", Some(sel!(performClose:)), "w");
        fallback_close_item
            .setKeyEquivalentModifierMask(NSEventModifierFlags::NSEventModifierFlagCommand);
        unsafe { fallback_close_item.setEnabled(true) };
        file_menu.addItem(&fallback_close_item);

        Ok(Self {
            main_menu,
            root_items,
            action_items,
            fallback_close_item,
            target,
        })
    }

    pub fn update(
        &self,
        states: &[MacMenuItemState],
        native_focus_active: bool,
        keymap_claims_close: bool,
    ) {
        debug_assert_eq!(self.action_items.len(), states.len());
        self.target
            .ivars()
            .native_focus_active
            .set(native_focus_active);
        let mtm = MainThreadMarker::new()
            .expect("QuickGUI native menu updates stay on the AppKit main thread");
        let app = NSApplication::sharedApplication(mtm);
        let os_actions = self.target.ivars().os_actions.borrow();
        let mut application_claims_close = keymap_claims_close;
        for (index, (item, state)) in self.action_items.iter().zip(states).enumerate() {
            let native_available = native_focus_active
                && os_actions
                    .get(index)
                    .copied()
                    .flatten()
                    .is_some_and(|action| unsafe {
                        app.targetForAction(os_action_selector(action)).is_some()
                    });
            unsafe {
                item.setEnabled(!state.disabled && (state.action_available || native_available));
                item.setState(if state.checked {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
            let (key, modifiers) = state
                .shortcut
                .as_ref()
                .and_then(appkit_key_equivalent)
                .unwrap_or_else(|| (String::new(), NSEventModifierFlags::empty()));
            unsafe {
                item.setKeyEquivalent(&NSString::from_str(&key));
            }
            item.setKeyEquivalentModifierMask(modifiers);
            application_claims_close |= !state.disabled
                && (state.action_available || native_available)
                && state.shortcut.as_ref().is_some_and(is_close_shortcut);
        }
        unsafe {
            self.fallback_close_item.setHidden(application_claims_close);
            self.fallback_close_item
                .setEnabled(!application_claims_close);
            self.fallback_close_item
                .setKeyEquivalent(&NSString::from_str(if application_claims_close {
                    ""
                } else {
                    "w"
                }));
        }
    }
}

fn is_close_shortcut(stroke: &Keystroke) -> bool {
    stroke.modifiers == Modifiers::SUPER
        && matches!(&stroke.key, Key::Character(value) if value.eq_ignore_ascii_case("w"))
}

impl Drop for MacMenuHost {
    fn drop(&mut self) {
        for item in self.root_items.drain(..) {
            if unsafe { self.main_menu.indexOfItem(&item) } >= 0 {
                unsafe { self.main_menu.removeItem(&item) };
            }
        }
    }
}

fn build_menu(
    menu: &Menu,
    mtm: MainThreadMarker,
    target: &QuickGuiMenuTarget,
    services_menu: Option<&NSMenu>,
    next_action_id: &mut usize,
    action_items: &mut Vec<Retained<NSMenuItem>>,
) -> Retained<NSMenu> {
    let native = unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(&menu.name)) };
    unsafe { native.setAutoenablesItems(false) };
    let delegate = ProtocolObject::from_ref(target);
    unsafe { native.setDelegate(Some(delegate)) };

    for declared in &menu.items {
        match declared {
            MenuItem::Action {
                name,
                os_action,
                checked,
                ..
            } => {
                let item = menu_item(mtm, name, Some(sel!(quickGuiPerformMenuAction:)), "");
                unsafe {
                    item.setTarget(Some(target));
                    item.setTag(*next_action_id as isize);
                    // Listener availability is known after the first retained-tree render.
                    item.setEnabled(false);
                    item.setState(if *checked {
                        NSControlStateValueOn
                    } else {
                        NSControlStateValueOff
                    });
                }
                target.ivars().os_actions.borrow_mut().push(*os_action);
                *next_action_id += 1;
                native.addItem(&item);
                action_items.push(item);
            }
            MenuItem::Separator => native.addItem(&NSMenuItem::separatorItem(mtm)),
            MenuItem::Submenu(submenu) => {
                let item = menu_item(mtm, &submenu.name, None, "");
                let native_submenu = build_menu(
                    submenu,
                    mtm,
                    target,
                    services_menu,
                    next_action_id,
                    action_items,
                );
                item.setSubmenu(Some(&native_submenu));
                unsafe { item.setEnabled(!submenu.disabled) };
                native.addItem(&item);
            }
            MenuItem::SystemMenu(os_menu) => {
                let item = menu_item(mtm, &os_menu.name, None, "");
                match os_menu.menu_type {
                    SystemMenuType::Services => item.setSubmenu(services_menu),
                }
                native.addItem(&item);
            }
        }
    }
    native
}

fn os_action_selector(action: OsAction) -> Sel {
    match action {
        OsAction::Cut => sel!(cut:),
        OsAction::Copy => sel!(copy:),
        OsAction::Paste => sel!(paste:),
        OsAction::SelectAll => sel!(selectAll:),
        OsAction::Undo => sel!(undo:),
        OsAction::Redo => sel!(redo:),
    }
}

fn perform_os_action(action: OsAction, sender: &NSMenuItem) -> bool {
    let app = NSApplication::sharedApplication(MainThreadMarker::from(sender));
    unsafe { app.sendAction_to_from(os_action_selector(action), None, Some(sender as &AnyObject)) }
}

fn menu_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    key_equivalent: &str,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key_equivalent),
        )
    };
    if item.respondsToSelector(sel!(setAllowsAutomaticKeyEquivalentLocalization:)) {
        // The keymap has already applied the binding's explicit localization policy. Letting
        // AppKit remap again would make the menu label and QuickGUI dispatch disagree.
        unsafe { item.setAllowsAutomaticKeyEquivalentLocalization(false) };
    }
    item
}

fn appkit_key_equivalent(stroke: &Keystroke) -> Option<(String, NSEventModifierFlags)> {
    let key = match &stroke.key {
        Key::Character(value) if value.chars().count() == 1 => value.to_lowercase(),
        Key::ArrowUp => "\u{f700}".to_owned(),
        Key::ArrowDown => "\u{f701}".to_owned(),
        Key::ArrowLeft => "\u{f702}".to_owned(),
        Key::ArrowRight => "\u{f703}".to_owned(),
        Key::Function(number @ 1..=24) => {
            char::from_u32(0xf704 + u32::from(*number) - 1)?.to_string()
        }
        Key::Insert => "\u{f727}".to_owned(),
        Key::Delete => "\u{f728}".to_owned(),
        Key::Home => "\u{f729}".to_owned(),
        Key::End => "\u{f72b}".to_owned(),
        Key::PageUp => "\u{f72c}".to_owned(),
        Key::PageDown => "\u{f72d}".to_owned(),
        Key::Enter => "\r".to_owned(),
        Key::Escape => "\u{1b}".to_owned(),
        Key::Space => " ".to_owned(),
        Key::Tab => "\t".to_owned(),
        Key::Backspace => "\u{8}".to_owned(),
        Key::Function(_) | Key::Character(_) | Key::Other => return None,
    };
    let mut modifiers = NSEventModifierFlags::empty();
    modifiers.set(
        NSEventModifierFlags::NSEventModifierFlagShift,
        stroke.modifiers.contains(Modifiers::SHIFT),
    );
    modifiers.set(
        NSEventModifierFlags::NSEventModifierFlagControl,
        stroke.modifiers.contains(Modifiers::CONTROL),
    );
    modifiers.set(
        NSEventModifierFlags::NSEventModifierFlagOption,
        stroke.modifiers.contains(Modifiers::ALT),
    );
    modifiers.set(
        NSEventModifierFlags::NSEventModifierFlagCommand,
        stroke.modifiers.contains(Modifiers::SUPER),
    );
    Some((key, modifiers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_printable_and_function_shortcuts() {
        let save = appkit_key_equivalent(&Keystroke::parse("cmd-shift-s").unwrap()).unwrap();
        assert_eq!(save.0, "s");
        assert!(
            save.1
                .contains(NSEventModifierFlags::NSEventModifierFlagCommand)
        );
        assert!(
            save.1
                .contains(NSEventModifierFlags::NSEventModifierFlagShift)
        );

        let next = appkit_key_equivalent(&Keystroke::parse("ctrl-f12").unwrap()).unwrap();
        assert_eq!(next.0, "\u{f70f}");
    }

    #[test]
    fn rejects_keys_appkit_cannot_represent() {
        assert!(appkit_key_equivalent(&Keystroke::new(Key::Other, Modifiers::SUPER)).is_none());
        assert!(
            appkit_key_equivalent(&Keystroke::new(
                Key::Character("ab".to_owned()),
                Modifiers::SUPER,
            ))
            .is_none()
        );
    }

    #[test]
    fn native_close_fallback_yields_only_to_exact_cmd_w() {
        assert!(is_close_shortcut(&Keystroke::parse("cmd-w").unwrap()));
        assert!(!is_close_shortcut(
            &Keystroke::parse("cmd-shift-w").unwrap()
        ));
        assert!(!is_close_shortcut(&Keystroke::parse("ctrl-w").unwrap()));
    }
}
