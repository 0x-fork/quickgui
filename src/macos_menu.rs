use std::cell::{Cell, RefCell};

use objc2::{
    ClassType, DeclaredClass, declare_class, msg_send_id,
    mutability::MainThreadOnly,
    rc::Retained,
    runtime::{AnyObject, NSObjectProtocol, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSEvent, NSEventModifierFlags,
    NSImage, NSMenu, NSMenuDelegate, NSMenuItem, NSView,
};
use objc2_foundation::{MainThreadMarker, NSData, NSObject, NSPoint, NSString};
use winit::event_loop::EventLoopProxy;
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use crate::{
    Key, Keystroke, Menu, MenuIcon, MenuItem, MenuItemMark, Modifiers, OsAction, SystemMenuType,
    menu::validate_menus, runtime::RuntimeEvent,
};

struct MenuTargetIvars {
    proxy: EventLoopProxy<RuntimeEvent>,
    scope: MenuTargetScope,
    os_actions: RefCell<Vec<Option<OsAction>>>,
    typed_actions: RefCell<Vec<bool>>,
    native_focus_active: Cell<bool>,
}

#[derive(Clone, Copy)]
enum MenuTargetScope {
    Application,
    Dock,
    Popup(u64),
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
            if matches!(self.ivars().scope, MenuTargetScope::Application) {
                let _ = self.ivars().proxy.send_event(RuntimeEvent::MenuWillOpen);
            }
        }
    }

    unsafe impl QuickGuiMenuTarget {
        #[method(quickGuiPerformMenuAction:)]
        fn perform_menu_action(&self, sender: &NSMenuItem) {
            let tag = unsafe { sender.tag() };
            if let Ok(action_id) = usize::try_from(tag) {
                let os_action = self
                    .ivars()
                    .os_actions
                    .borrow()
                    .get(action_id)
                    .copied()
                    .flatten();
                let has_typed_action = self
                    .ivars()
                    .typed_actions
                    .borrow()
                    .get(action_id)
                    .copied()
                    .unwrap_or(false);
                if os_action.is_some_and(|action| {
                    (!has_typed_action
                        || self.ivars().native_focus_active.get() && action.is_text_editing())
                        && perform_os_action(action, sender)
                })
                {
                    return;
                }
                // The numeric id keeps the AppKit callback Send while typed actions remain cheap,
                // main-thread-only `Rc` values owned by the runtime.
                let event = match self.ivars().scope {
                    MenuTargetScope::Application => RuntimeEvent::MenuAction(action_id),
                    MenuTargetScope::Dock => RuntimeEvent::DockMenuAction(action_id),
                    MenuTargetScope::Popup(popup_id) => {
                        RuntimeEvent::NativePopupMenuAction(popup_id, action_id)
                    }
                };
                let _ = self.ivars().proxy.send_event(event);
            }
        }
    }
);

impl QuickGuiMenuTarget {
    fn new(
        mtm: MainThreadMarker,
        proxy: EventLoopProxy<RuntimeEvent>,
        scope: MenuTargetScope,
    ) -> Retained<QuickGuiMenuTarget> {
        let allocated = mtm.alloc().set_ivars(MenuTargetIvars {
            proxy,
            scope,
            os_actions: RefCell::new(Vec::new()),
            typed_actions: RefCell::new(Vec::new()),
            native_focus_active: Cell::new(false),
        });
        unsafe { msg_send_id![super(allocated), init] }
    }
}

pub(crate) struct MacMenuItemState {
    pub disabled: bool,
    pub action_available: bool,
    pub checked: bool,
    /// Explicit declaration shortcut, else the keymap binding QuickGUI would display.
    pub shortcut: Option<Keystroke>,
    pub hidden: bool,
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
        validate_menus(menus).map_err(|error| error.to_string())?;
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "native menus must be initialized on the AppKit main thread".to_owned()
        })?;
        let app = NSApplication::sharedApplication(mtm);
        let main_menu = unsafe { app.mainMenu() }
            .ok_or_else(|| "AppKit did not install an application menu bar".to_owned())?;
        let services_menu = unsafe { app.servicesMenu() };
        let windows_menu = unsafe { app.windowsMenu() };
        let help_menu = unsafe { app.helpMenu() };
        let target = QuickGuiMenuTarget::new(mtm, proxy, MenuTargetScope::Application);
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
                windows_menu.as_deref(),
                help_menu.as_deref(),
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
            let os_action = os_actions.get(index).copied().flatten();
            let native_available = os_action.is_some_and(|action| {
                (!action.is_text_editing() || native_focus_active)
                    && unsafe { app.targetForAction(os_action_selector(action)).is_some() }
            });
            unsafe {
                item.setEnabled(!state.disabled && (state.action_available || native_available));
                item.setHidden(state.hidden);
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
                .or_else(|| os_action.and_then(default_os_action_key_equivalent))
                .unwrap_or_else(|| (String::new(), NSEventModifierFlags::empty()));
            unsafe {
                item.setKeyEquivalent(&NSString::from_str(&key));
            }
            item.setKeyEquivalentModifierMask(modifiers);
            application_claims_close |= menu_item_claims_close(state, os_action, native_available);
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

fn default_os_action_key_equivalent(action: OsAction) -> Option<(String, NSEventModifierFlags)> {
    let command = NSEventModifierFlags::NSEventModifierFlagCommand;
    let command_option = command | NSEventModifierFlags::NSEventModifierFlagOption;
    let command_control = command | NSEventModifierFlags::NSEventModifierFlagControl;
    match action {
        OsAction::HideApplication => Some(("h".to_owned(), command)),
        OsAction::HideOtherApplications => Some(("h".to_owned(), command_option)),
        OsAction::Quit => Some(("q".to_owned(), command)),
        OsAction::CloseWindow => Some(("w".to_owned(), command)),
        OsAction::MinimizeWindow => Some(("m".to_owned(), command)),
        OsAction::ToggleFullscreen => Some(("f".to_owned(), command_control)),
        OsAction::PasteAndMatchStyle => Some((
            "v".to_owned(),
            command
                | NSEventModifierFlags::NSEventModifierFlagOption
                | NSEventModifierFlags::NSEventModifierFlagShift,
        )),
        OsAction::SelectNextTab
        | OsAction::SelectPreviousTab
        | OsAction::MergeAllWindows
        | OsAction::MoveTabToNewWindow
        | OsAction::ToggleTabBar
        | OsAction::ToggleTabOverview
        | OsAction::Delete
        | OsAction::StartSpeaking
        | OsAction::StopSpeaking => None,
        OsAction::Cut
        | OsAction::Copy
        | OsAction::Paste
        | OsAction::SelectAll
        | OsAction::Undo
        | OsAction::Redo
        | OsAction::About
        | OsAction::ShowAllApplications
        | OsAction::ZoomWindow
        | OsAction::BringAllToFront
        | OsAction::ShowHelp => None,
    }
}

fn menu_item_claims_close(
    state: &MacMenuItemState,
    os_action: Option<OsAction>,
    native_available: bool,
) -> bool {
    !state.hidden
        && !state.disabled
        && (state.action_available || native_available)
        && (os_action == Some(OsAction::CloseWindow)
            || state.shortcut.as_ref().is_some_and(is_close_shortcut))
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

/// Owns the application-declared menu returned by `applicationDockMenu:`.
pub(crate) struct MacDockMenuHost {
    native: Retained<NSMenu>,
    _action_items: Vec<Retained<NSMenuItem>>,
    _target: Retained<QuickGuiMenuTarget>,
}

impl MacDockMenuHost {
    pub(crate) fn new(menu: &Menu, proxy: EventLoopProxy<RuntimeEvent>) -> Result<Self, String> {
        validate_menus(std::slice::from_ref(menu)).map_err(|error| error.to_string())?;
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "the Dock menu must be initialized on the AppKit main thread".to_owned()
        })?;
        let app = NSApplication::sharedApplication(mtm);
        let services_menu = unsafe { app.servicesMenu() };
        let windows_menu = unsafe { app.windowsMenu() };
        let help_menu = unsafe { app.helpMenu() };
        let target = QuickGuiMenuTarget::new(mtm, proxy, MenuTargetScope::Dock);
        let mut next_action_id = 0;
        let mut action_items = Vec::new();
        let native = build_menu(
            menu,
            mtm,
            &target,
            services_menu.as_deref(),
            windows_menu.as_deref(),
            help_menu.as_deref(),
            &mut next_action_id,
            &mut action_items,
        );
        for (item, declared) in
            action_items
                .iter()
                .zip(crate::menu::collect_menu_actions(std::slice::from_ref(
                    menu,
                )))
        {
            unsafe {
                item.setEnabled(!declared.disabled);
                item.setHidden(declared.hidden);
                item.setState(if declared.checked {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
            let (key, modifiers) = declared
                .shortcut
                .as_ref()
                .and_then(appkit_key_equivalent)
                .or_else(|| {
                    declared
                        .os_action
                        .and_then(default_os_action_key_equivalent)
                })
                .unwrap_or_else(|| (String::new(), NSEventModifierFlags::empty()));
            unsafe { item.setKeyEquivalent(&NSString::from_str(&key)) };
            item.setKeyEquivalentModifierMask(modifiers);
        }
        Ok(Self {
            native,
            _action_items: action_items,
            _target: target,
        })
    }

    pub(crate) fn native_retained(&self) -> Retained<NSMenu> {
        self.native.clone()
    }
}

pub(crate) fn show_popup_menu(
    menu: &Menu,
    popup_id: u64,
    window: &Window,
    position: Option<crate::Point>,
    states: &[MacMenuItemState],
    native_focus_active: bool,
    proxy: EventLoopProxy<RuntimeEvent>,
) -> Result<bool, String> {
    validate_menus(std::slice::from_ref(menu)).map_err(|error| error.to_string())?;
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "native popup menus must be shown on the AppKit main thread".to_owned())?;
    let app = NSApplication::sharedApplication(mtm);
    let services_menu = unsafe { app.servicesMenu() };
    let windows_menu = unsafe { app.windowsMenu() };
    let help_menu = unsafe { app.helpMenu() };
    let target = QuickGuiMenuTarget::new(mtm, proxy, MenuTargetScope::Popup(popup_id));
    let mut next_action_id = 0;
    let mut action_items = Vec::new();
    let native = build_menu(
        menu,
        mtm,
        &target,
        services_menu.as_deref(),
        windows_menu.as_deref(),
        help_menu.as_deref(),
        &mut next_action_id,
        &mut action_items,
    );
    if action_items.len() != states.len() {
        return Err("native popup menu action state did not match its declaration".to_owned());
    }
    target.ivars().native_focus_active.set(native_focus_active);
    let os_actions = target.ivars().os_actions.borrow();
    for (index, (item, state)) in action_items.iter().zip(states).enumerate() {
        let os_action = os_actions.get(index).copied().flatten();
        let native_available = os_action.is_some_and(|action| {
            (!action.is_text_editing() || native_focus_active)
                && unsafe { app.targetForAction(os_action_selector(action)).is_some() }
        });
        unsafe {
            item.setEnabled(!state.disabled && (state.action_available || native_available));
            item.setHidden(state.hidden);
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
            .or_else(|| os_action.and_then(default_os_action_key_equivalent))
            .unwrap_or_else(|| (String::new(), NSEventModifierFlags::empty()));
        unsafe { item.setKeyEquivalent(&NSString::from_str(&key)) };
        item.setKeyEquivalentModifierMask(modifiers);
    }
    drop(os_actions);

    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("an AppKit popup menu requires an AppKit window handle".to_owned());
    };
    // SAFETY: Winit owns this NSView for at least the complete lifetime of `window`, retained by
    // the runtime while synchronous AppKit menu tracking is active.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let (location, view) = if let Some(position) = position {
        let frame = view.frame();
        (
            NSPoint::new(
                f64::from(position.x),
                frame.size.height - f64::from(position.y),
            ),
            Some(view),
        )
    } else {
        (unsafe { NSEvent::mouseLocation() }, None)
    };
    Ok(unsafe { native.popUpMenuPositioningItem_atLocation_inView(None, location, view) })
}

#[allow(clippy::too_many_arguments)]
fn build_menu(
    menu: &Menu,
    mtm: MainThreadMarker,
    target: &QuickGuiMenuTarget,
    services_menu: Option<&NSMenu>,
    windows_menu: Option<&NSMenu>,
    help_menu: Option<&NSMenu>,
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
                mark,
                icon,
                hidden,
                ..
            } => {
                append_action_item(
                    &native,
                    mtm,
                    target,
                    name,
                    *os_action,
                    true,
                    *checked,
                    *mark,
                    icon.as_ref(),
                    *hidden,
                    next_action_id,
                    action_items,
                );
            }
            MenuItem::Role {
                name,
                role,
                checked,
                mark,
                icon,
                hidden,
                ..
            } => {
                append_action_item(
                    &native,
                    mtm,
                    target,
                    name,
                    Some(*role),
                    false,
                    *checked,
                    *mark,
                    icon.as_ref(),
                    *hidden,
                    next_action_id,
                    action_items,
                );
            }
            MenuItem::Separator => native.addItem(&NSMenuItem::separatorItem(mtm)),
            MenuItem::Submenu(submenu) => {
                let item = menu_item(mtm, &submenu.name, None, "");
                let native_submenu = build_menu(
                    submenu,
                    mtm,
                    target,
                    services_menu,
                    windows_menu,
                    help_menu,
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
                    SystemMenuType::Window => item.setSubmenu(windows_menu),
                    SystemMenuType::Help => item.setSubmenu(help_menu),
                    SystemMenuType::RecentDocuments => {
                        item.setSubmenu(Some(&recent_documents_menu(mtm, &os_menu.name)));
                    }
                }
                native.addItem(&item);
            }
        }
    }
    native
}

#[allow(clippy::too_many_arguments)]
fn append_action_item(
    native: &NSMenu,
    mtm: MainThreadMarker,
    target: &QuickGuiMenuTarget,
    name: &str,
    os_action: Option<OsAction>,
    has_typed_action: bool,
    checked: bool,
    mark: MenuItemMark,
    icon: Option<&MenuIcon>,
    hidden: bool,
    next_action_id: &mut usize,
    action_items: &mut Vec<Retained<NSMenuItem>>,
) {
    let item = menu_item(mtm, name, Some(sel!(quickGuiPerformMenuAction:)), "");
    unsafe {
        item.setTarget(Some(target));
        item.setTag(*next_action_id as isize);
        item.setHidden(hidden);
        // Listener and native responder availability are known after the first render.
        item.setEnabled(false);
        item.setState(if checked {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
    }
    if mark == MenuItemMark::Radio {
        let symbol = NSString::from_str("circle.inset.filled");
        let description = NSString::from_str("Selected");
        let radio = unsafe {
            NSImage::imageWithSystemSymbolName_accessibilityDescription(&symbol, Some(&description))
        };
        unsafe { item.setOnStateImage(radio.as_deref()) };
    }
    if let Some(icon) = icon.and_then(|icon| native_menu_icon(mtm, icon)) {
        unsafe { item.setImage(Some(&icon)) };
    }
    target.ivars().os_actions.borrow_mut().push(os_action);
    target
        .ivars()
        .typed_actions
        .borrow_mut()
        .push(has_typed_action);
    *next_action_id += 1;
    native.addItem(&item);
    action_items.push(item);
}

/// Build the AppKit menu `NSDocumentController` populates with recent documents.
///
/// AppKit recognizes the menu by the `clearRecentDocuments:` item it owns, so the returned menu
/// starts with only "Clear Menu" and is filled by the operating system.
fn recent_documents_menu(mtm: MainThreadMarker, title: &str) -> Retained<NSMenu> {
    let native = unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(title)) };
    unsafe { native.setAutoenablesItems(true) };
    let clear = menu_item(mtm, "Clear Menu", Some(sel!(clearRecentDocuments:)), "");
    native.addItem(&clear);
    native
}

fn native_menu_icon(mtm: MainThreadMarker, icon: &MenuIcon) -> Option<Retained<NSImage>> {
    use image_codecs::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    let mut encoded = Vec::new();
    if let Err(error) = PngEncoder::new(&mut encoded).write_image(
        icon.rgba(),
        icon.width(),
        icon.height(),
        ExtendedColorType::Rgba8,
    ) {
        tracing::warn!(%error, "could not encode a native menu icon");
        return None;
    }
    NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(&encoded))
}

fn os_action_selector(action: OsAction) -> Sel {
    match action {
        OsAction::Cut => sel!(cut:),
        OsAction::Copy => sel!(copy:),
        OsAction::Paste => sel!(paste:),
        OsAction::SelectAll => sel!(selectAll:),
        OsAction::Undo => sel!(undo:),
        OsAction::Redo => sel!(redo:),
        OsAction::About => sel!(orderFrontStandardAboutPanel:),
        OsAction::HideApplication => sel!(hide:),
        OsAction::HideOtherApplications => sel!(hideOtherApplications:),
        OsAction::ShowAllApplications => sel!(unhideAllApplications:),
        OsAction::Quit => sel!(terminate:),
        OsAction::CloseWindow => sel!(performClose:),
        OsAction::MinimizeWindow => sel!(performMiniaturize:),
        OsAction::ZoomWindow => sel!(performZoom:),
        OsAction::ToggleFullscreen => sel!(toggleFullScreen:),
        OsAction::BringAllToFront => sel!(arrangeInFront:),
        OsAction::ShowHelp => sel!(showHelp:),
        OsAction::PasteAndMatchStyle => sel!(pasteAsPlainText:),
        OsAction::Delete => sel!(delete:),
        OsAction::StartSpeaking => sel!(startSpeaking:),
        OsAction::StopSpeaking => sel!(stopSpeaking:),
        OsAction::SelectNextTab => sel!(selectNextTab:),
        OsAction::SelectPreviousTab => sel!(selectPreviousTab:),
        OsAction::MergeAllWindows => sel!(mergeAllWindows:),
        OsAction::MoveTabToNewWindow => sel!(moveTabToNewWindow:),
        OsAction::ToggleTabBar => sel!(toggleTabBar:),
        OsAction::ToggleTabOverview => sel!(toggleTabOverview:),
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

        let available = MacMenuItemState {
            disabled: false,
            action_available: true,
            checked: false,
            shortcut: None,
            hidden: false,
        };
        assert!(menu_item_claims_close(
            &available,
            Some(OsAction::CloseWindow),
            false,
        ));

        let disabled = MacMenuItemState {
            disabled: true,
            ..available
        };
        assert!(!menu_item_claims_close(
            &disabled,
            Some(OsAction::CloseWindow),
            true,
        ));

        let hidden = MacMenuItemState {
            disabled: false,
            action_available: true,
            checked: false,
            shortcut: None,
            hidden: true,
        };
        assert!(!menu_item_claims_close(
            &hidden,
            Some(OsAction::CloseWindow),
            true,
        ));
    }

    #[test]
    fn declared_accelerators_outrank_default_role_key_equivalents() {
        let role_default = default_os_action_key_equivalent(OsAction::MinimizeWindow)
            .expect("Minimize has a default key equivalent");
        assert_eq!(role_default.0, "m");

        // `MacMenuHost::update` resolves the declaration shortcut before the role default, so an
        // explicit accelerator replaces the AppKit standard binding.
        let declared = crate::Accelerator::parse("Cmd+Shift+M").expect("valid accelerator");
        let resolved = Some(declared)
            .as_ref()
            .and_then(appkit_key_equivalent)
            .or_else(|| default_os_action_key_equivalent(OsAction::MinimizeWindow))
            .expect("an explicit accelerator resolves");
        assert_eq!(resolved.0, "m");
        assert!(
            resolved
                .1
                .contains(NSEventModifierFlags::NSEventModifierFlagShift)
        );
    }

    #[test]
    fn tab_and_speech_roles_leave_key_equivalents_to_the_declaration() {
        for role in [
            OsAction::SelectNextTab,
            OsAction::SelectPreviousTab,
            OsAction::MergeAllWindows,
            OsAction::MoveTabToNewWindow,
            OsAction::ToggleTabBar,
            OsAction::ToggleTabOverview,
            OsAction::Delete,
            OsAction::StartSpeaking,
            OsAction::StopSpeaking,
        ] {
            assert!(default_os_action_key_equivalent(role).is_none());
        }
        let paste_and_match =
            default_os_action_key_equivalent(OsAction::PasteAndMatchStyle).expect("standard");
        assert_eq!(paste_and_match.0, "v");
    }
}
