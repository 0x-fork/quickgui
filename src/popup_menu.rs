use std::{
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use thiserror::Error;

use crate::{
    AccessibilityPopup, AccessibilityRole, Action, AnyAction, Element, ElementId, EventContext,
    FocusHandle, Key, KeyBinding, Modifiers, ViewContext,
};

/// Maximum entries retained by one popup-menu tree, including nested submenus.
pub const MAX_POPUP_MENU_ITEMS: usize = 2_048;
/// Maximum nested popup-menu levels retained by one model.
pub const MAX_POPUP_MENU_DEPTH: usize = 8;
/// Maximum UTF-8 bytes retained by one item label, shortcut, or typeahead label.
pub const MAX_POPUP_MENU_ITEM_TEXT_BYTES: usize = 16 * 1024;
/// Maximum text retained by one complete popup-menu tree.
pub const MAX_POPUP_MENU_TEXT_BYTES: usize = 4 * 1024 * 1024;
/// Maximum normalized text-navigation prefix retained between key presses.
pub const MAX_POPUP_MENU_TYPEAHEAD_BYTES: usize = 256;
/// Inactivity interval after which the next printable key starts a fresh search.
pub const POPUP_MENU_TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(500);

/// Context installed on an interactive [`PopupMenu`] root.
pub const POPUP_MENU_KEY_CONTEXT: &str = "PopupMenu";

const ITEM_ID_TAG: u64 = 0x09b5_05a4_420a_f4d1;
const GROUP_LABEL_ID_TAG: u64 = 0xd4c3_7b1a_91e5_208f;

/// Highlight the previous enabled popup-menu item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuPrevious;
/// Highlight the next enabled popup-menu item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuNext;
/// Highlight the first enabled popup-menu item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuFirst;
/// Highlight the final enabled popup-menu item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuLast;
/// Activate the highlighted popup-menu item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuActivate;
/// Open the highlighted submenu without activating an ordinary command.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuOpenSubmenu;
/// Dismiss the current popup-menu level.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PopupMenuClose;

/// Contextual keyboard behavior for an open popup menu.
pub fn popup_menu_key_bindings() -> [KeyBinding; 9] {
    [
        KeyBinding::new("up", PopupMenuPrevious, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("down", PopupMenuNext, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("home", PopupMenuFirst, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("end", PopupMenuLast, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("enter", PopupMenuActivate, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("space", PopupMenuActivate, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("right", PopupMenuOpenSubmenu, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("left", PopupMenuClose, Some(POPUP_MENU_KEY_CONTEXT)),
        KeyBinding::new("escape", PopupMenuClose, Some(POPUP_MENU_KEY_CONTEXT)),
    ]
}

/// The structural role of one unstyled popup-menu entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopupMenuItemKind {
    Action,
    Checkbox,
    Radio,
    Submenu,
    Separator,
    GroupLabel,
}

#[derive(Clone)]
enum PopupMenuCommand {
    Fixed(AnyAction),
    Toggle(Rc<dyn Fn(bool) -> AnyAction>),
}

impl PopupMenuCommand {
    fn resolve(&self, checked: Option<bool>) -> AnyAction {
        match self {
            Self::Fixed(action) => action.clone(),
            Self::Toggle(action) => action(checked.unwrap_or(false)),
        }
    }
}

#[derive(Clone)]
enum PopupMenuItemContent {
    Action(PopupMenuCommand),
    Checkbox {
        checked: bool,
        command: PopupMenuCommand,
    },
    Radio {
        group: ElementId,
        selected: bool,
        command: PopupMenuCommand,
    },
    Submenu(Box<PopupMenu>),
    Separator,
    GroupLabel,
}

/// One retained, appearance-free entry in a [`PopupMenu`].
///
/// Interactive entries require a stable ID. The ID is namespaced by the menu root when converted
/// into an element, so the same application command ID can safely appear in different submenus.
#[derive(Clone)]
pub struct PopupMenuItem {
    id: Option<ElementId>,
    label: Arc<str>,
    search_label: Option<Arc<str>>,
    shortcut: Option<Arc<str>>,
    disabled: bool,
    close_on_activate: bool,
    content: PopupMenuItemContent,
}

impl fmt::Debug for PopupMenuItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PopupMenuItem")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("shortcut", &self.shortcut)
            .field("disabled", &self.disabled)
            .field("close_on_activate", &self.close_on_activate)
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl PopupMenuItem {
    /// Create an ordinary command item that closes the menu after dispatch by default.
    pub fn action<A: Action>(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        action: A,
    ) -> Self {
        Self::action_any(id, label, AnyAction::new(action))
    }

    /// Create an ordinary command item from a heterogeneous command registry.
    pub fn action_any(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        action: AnyAction,
    ) -> Self {
        Self::interactive(
            id.into(),
            label.into(),
            true,
            PopupMenuItemContent::Action(PopupMenuCommand::Fixed(action)),
        )
    }

    /// Create a checkbox item. Checkbox items remain open after activation by default.
    pub fn checkbox<A: Action>(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        checked: bool,
        action: A,
    ) -> Self {
        Self::checkbox_any(id, label, checked, AnyAction::new(action))
    }

    /// Create a checkbox item from a heterogeneous command registry.
    pub fn checkbox_any(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        checked: bool,
        action: AnyAction,
    ) -> Self {
        Self::interactive(
            id.into(),
            label.into(),
            false,
            PopupMenuItemContent::Checkbox {
                checked,
                command: PopupMenuCommand::Fixed(action),
            },
        )
    }

    /// Create a checkbox whose typed action contains the newly toggled value.
    pub fn checkbox_with<A, F>(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        checked: bool,
        action: F,
    ) -> Self
    where
        A: Action,
        F: Fn(bool) -> A + 'static,
    {
        Self::interactive(
            id.into(),
            label.into(),
            false,
            PopupMenuItemContent::Checkbox {
                checked,
                command: PopupMenuCommand::Toggle(Rc::new(move |checked| {
                    AnyAction::new(action(checked))
                })),
            },
        )
    }

    /// Create one radio item. Radio items remain open after activation by default.
    pub fn radio<A: Action>(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        group: impl Into<ElementId>,
        selected: bool,
        action: A,
    ) -> Self {
        let group = group.into();
        Self::interactive(
            id.into(),
            label.into(),
            false,
            PopupMenuItemContent::Radio {
                group,
                selected,
                command: PopupMenuCommand::Fixed(AnyAction::new(action)),
            },
        )
    }

    /// Create an item that opens another validated popup-menu model.
    pub fn submenu(
        id: impl Into<ElementId>,
        label: impl Into<Arc<str>>,
        submenu: PopupMenu,
    ) -> Self {
        Self::interactive(
            id.into(),
            label.into(),
            false,
            PopupMenuItemContent::Submenu(Box::new(submenu)),
        )
    }

    /// Create a visual separator. The caller supplies its complete appearance.
    pub fn separator() -> Self {
        Self {
            id: None,
            label: Arc::from(""),
            search_label: None,
            shortcut: None,
            disabled: false,
            close_on_activate: false,
            content: PopupMenuItemContent::Separator,
        }
    }

    /// Create a non-interactive label for an application-composed semantic group.
    pub fn group_label(label: impl Into<Arc<str>>) -> Self {
        Self {
            id: None,
            label: label.into(),
            search_label: None,
            shortcut: None,
            disabled: false,
            close_on_activate: false,
            content: PopupMenuItemContent::GroupLabel,
        }
    }

    fn interactive(
        id: ElementId,
        label: Arc<str>,
        close_on_activate: bool,
        content: PopupMenuItemContent,
    ) -> Self {
        let search_label = Arc::from(label.to_lowercase());
        Self {
            id: Some(id),
            label,
            search_label: Some(search_label),
            shortcut: None,
            disabled: false,
            close_on_activate,
            content,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<Arc<str>>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Override the label used for alphanumeric keyboard navigation.
    pub fn typeahead_label(mut self, label: impl Into<Arc<str>>) -> Self {
        if self.is_interactive() {
            let label: Arc<str> = label.into();
            self.search_label = Some(Arc::from(label.to_lowercase()));
        }
        self
    }

    pub const fn close_on_activate(mut self, close: bool) -> Self {
        self.close_on_activate = close;
        self
    }

    pub const fn id(&self) -> Option<ElementId> {
        self.id
    }

    pub fn label(&self) -> &Arc<str> {
        &self.label
    }

    pub fn shortcut_text(&self) -> Option<&Arc<str>> {
        self.shortcut.as_ref()
    }

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub const fn closes_on_activate(&self) -> bool {
        self.close_on_activate
    }

    pub const fn kind(&self) -> PopupMenuItemKind {
        match self.content {
            PopupMenuItemContent::Action(_) => PopupMenuItemKind::Action,
            PopupMenuItemContent::Checkbox { .. } => PopupMenuItemKind::Checkbox,
            PopupMenuItemContent::Radio { .. } => PopupMenuItemKind::Radio,
            PopupMenuItemContent::Submenu(_) => PopupMenuItemKind::Submenu,
            PopupMenuItemContent::Separator => PopupMenuItemKind::Separator,
            PopupMenuItemContent::GroupLabel => PopupMenuItemKind::GroupLabel,
        }
    }

    pub const fn is_interactive(&self) -> bool {
        matches!(
            self.content,
            PopupMenuItemContent::Action(_)
                | PopupMenuItemContent::Checkbox { .. }
                | PopupMenuItemContent::Radio { .. }
                | PopupMenuItemContent::Submenu(_)
        )
    }

    pub const fn checked(&self) -> Option<bool> {
        match self.content {
            PopupMenuItemContent::Checkbox { checked, .. } => Some(checked),
            PopupMenuItemContent::Radio { selected, .. } => Some(selected),
            _ => None,
        }
    }

    pub fn submenu_menu(&self) -> Option<&PopupMenu> {
        match &self.content {
            PopupMenuItemContent::Submenu(menu) => Some(menu),
            _ => None,
        }
    }
}

/// A construction error that prevents unbounded or ambiguous retained menu state.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PopupMenuError {
    #[error("a popup menu tree cannot retain more than {MAX_POPUP_MENU_ITEMS} entries")]
    TooManyItems,
    #[error("a popup menu tree cannot nest deeper than {MAX_POPUP_MENU_DEPTH} levels")]
    TooDeep,
    #[error(
        "one popup-menu label, shortcut, or typeahead label cannot exceed {MAX_POPUP_MENU_ITEM_TEXT_BYTES} UTF-8 bytes"
    )]
    ItemTextTooLong,
    #[error("a popup menu tree cannot retain more than {MAX_POPUP_MENU_TEXT_BYTES} text bytes")]
    TooMuchText,
    #[error("interactive popup-menu item ID {0:?} appears more than once at one menu level")]
    DuplicateItemId(ElementId),
    #[error("radio group {0:?} contains more than one initially selected item")]
    MultipleSelectedRadioItems(ElementId),
}

/// Result of activating one enabled popup-menu item.
#[derive(Clone, Debug)]
pub enum PopupMenuActivation {
    Command {
        item_index: usize,
        action: AnyAction,
        close_menu: bool,
    },
    Submenu {
        item_index: usize,
        menu: PopupMenu,
    },
}

/// Render-state supplied to an application's unstyled item renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PopupMenuItemState {
    pub highlighted: bool,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub has_submenu: bool,
}

/// Bounded, zero-idle interaction state for an unstyled popup menu.
///
/// The model owns no window, renderer, timer, task, observer, or theme. Typeahead expiration is
/// evaluated only when the next key arrives. Use [`crate::AnchoredPopover`] as its native surface
/// when content must extend past the owner window.
#[derive(Clone, Debug)]
pub struct PopupMenu {
    items: Vec<PopupMenuItem>,
    active: Option<usize>,
    loop_focus: bool,
    typeahead: String,
    typeahead_at: Option<Instant>,
}

impl PopupMenu {
    pub fn new(items: impl IntoIterator<Item = PopupMenuItem>) -> Result<Self, PopupMenuError> {
        let items = items.into_iter().collect::<Vec<_>>();
        validate_menu_tree(&items)?;
        let active = items.iter().position(selectable);
        Ok(Self {
            items,
            active,
            loop_focus: true,
            typeahead: String::new(),
            typeahead_at: None,
        })
    }

    pub fn items(&self) -> &[PopupMenuItem] {
        &self.items
    }

    pub const fn active_index(&self) -> Option<usize> {
        self.active
    }

    pub fn active_item(&self) -> Option<&PopupMenuItem> {
        self.active.and_then(|index| self.items.get(index))
    }

    pub const fn loops_focus(&self) -> bool {
        self.loop_focus
    }

    pub const fn loop_focus(mut self, enabled: bool) -> Self {
        self.loop_focus = enabled;
        self
    }

    /// Atomically replace the menu contents while preserving the highlighted stable ID.
    pub fn set_items(
        &mut self,
        items: impl IntoIterator<Item = PopupMenuItem>,
    ) -> Result<bool, PopupMenuError> {
        let items = items.into_iter().collect::<Vec<_>>();
        validate_menu_tree(&items)?;
        let previous_id = self.active_item().and_then(PopupMenuItem::id);
        self.items = items;
        self.active = previous_id
            .and_then(|id| self.items.iter().position(|item| item.id() == Some(id)))
            .filter(|index| selectable(&self.items[*index]))
            .or_else(|| self.items.iter().position(selectable));
        self.clear_typeahead();
        Ok(previous_id != self.active_item().and_then(PopupMenuItem::id))
    }

    pub fn highlight(&mut self, index: usize) -> bool {
        if self.items.get(index).is_none_or(|item| !selectable(item)) || self.active == Some(index)
        {
            return false;
        }
        self.active = Some(index);
        self.clear_typeahead();
        true
    }

    pub fn select_previous(&mut self) -> bool {
        self.move_active(false)
    }

    pub fn select_next(&mut self) -> bool {
        self.move_active(true)
    }

    pub fn select_first(&mut self) -> bool {
        self.select_boundary(false)
    }

    pub fn select_last(&mut self) -> bool {
        self.select_boundary(true)
    }

    fn select_boundary(&mut self, last: bool) -> bool {
        let next = if last {
            self.items.iter().rposition(selectable)
        } else {
            self.items.iter().position(selectable)
        };
        if next == self.active {
            return false;
        }
        self.active = next;
        self.clear_typeahead();
        true
    }

    fn move_active(&mut self, forward: bool) -> bool {
        let count = self.items.len();
        if count == 0 {
            return false;
        }
        let Some(current) = self.active else {
            return self.select_boundary(!forward);
        };
        for step in 1..=count {
            let candidate = if forward {
                current.saturating_add(step)
            } else {
                current.checked_sub(step).unwrap_or(count)
            };
            let candidate = if candidate >= count {
                if !self.loop_focus {
                    break;
                }
                if forward {
                    candidate % count
                } else {
                    count - (step - current) % count
                }
            } else {
                candidate
            };
            let candidate = candidate % count;
            if selectable(&self.items[candidate]) {
                if candidate == current {
                    return false;
                }
                self.active = Some(candidate);
                self.clear_typeahead();
                return true;
            }
        }
        false
    }

    /// Update bounded typeahead state and highlight the next prefix match.
    ///
    /// `now` is supplied by the caller so deterministic tests do not depend on wall-clock sleeps.
    /// The return value reports whether the highlighted item changed.
    pub fn typeahead(&mut self, value: &str, now: Instant) -> bool {
        let input = normalized_typeahead_input(value);
        if input.is_empty() {
            return false;
        }
        if self.typeahead_at.is_none_or(|previous| {
            now.saturating_duration_since(previous) > POPUP_MENU_TYPEAHEAD_TIMEOUT
        }) {
            self.typeahead.clear();
        }
        self.typeahead_at = Some(now);
        push_bounded(&mut self.typeahead, &input, MAX_POPUP_MENU_TYPEAHEAD_BYTES);
        if self.typeahead.is_empty() {
            return false;
        }

        let first = self.typeahead.chars().next();
        let repeated_character = first.is_some()
            && self
                .typeahead
                .chars()
                .all(|character| Some(character) == first);
        let repeated_prefix;
        let prefix = if repeated_character && self.typeahead.chars().count() > 1 {
            repeated_prefix = first
                .expect("a non-empty typeahead has one character")
                .to_string();
            repeated_prefix.as_str()
        } else {
            self.typeahead.as_str()
        };

        let count = self.items.len();
        let start = self.active.map_or(0, |active| (active + 1) % count.max(1));
        for step in 0..count {
            let index = (start + step) % count;
            let item = &self.items[index];
            if !selectable(item)
                || !item
                    .search_label
                    .as_deref()
                    .is_some_and(|label| label.starts_with(prefix))
            {
                continue;
            }
            if self.active == Some(index) {
                return false;
            }
            self.active = Some(index);
            return true;
        }
        false
    }

    pub fn activate_active(&mut self) -> Option<PopupMenuActivation> {
        self.active.and_then(|index| self.activate(index))
    }

    pub fn activate(&mut self, index: usize) -> Option<PopupMenuActivation> {
        if self.items.get(index).is_none_or(|item| !selectable(item)) {
            return None;
        }

        let active_item_id = self.items[index].id;
        let radio_group = match self.items[index].content {
            PopupMenuItemContent::Radio { group, .. } => Some(group),
            _ => None,
        };
        if let Some(group) = radio_group {
            for item in &mut self.items {
                if let PopupMenuItemContent::Radio {
                    group: item_group,
                    selected,
                    ..
                } = &mut item.content
                    && *item_group == group
                {
                    *selected = item.id == active_item_id;
                }
            }
        }

        let item = &mut self.items[index];
        let checked = match &mut item.content {
            PopupMenuItemContent::Checkbox { checked, .. } => {
                *checked = !*checked;
                Some(*checked)
            }
            PopupMenuItemContent::Radio { selected, .. } => Some(*selected),
            _ => None,
        };
        let activation = match &item.content {
            PopupMenuItemContent::Action(command)
            | PopupMenuItemContent::Checkbox { command, .. }
            | PopupMenuItemContent::Radio { command, .. } => PopupMenuActivation::Command {
                item_index: index,
                action: command.resolve(checked),
                close_menu: item.close_on_activate,
            },
            PopupMenuItemContent::Submenu(menu) => PopupMenuActivation::Submenu {
                item_index: index,
                menu: menu.as_ref().clone(),
            },
            PopupMenuItemContent::Separator | PopupMenuItemContent::GroupLabel => return None,
        };
        self.active = Some(index);
        self.clear_typeahead();
        Some(activation)
    }

    pub fn item_state(&self, index: usize) -> Option<PopupMenuItemState> {
        let item = self.items.get(index)?;
        Some(PopupMenuItemState {
            highlighted: self.active == Some(index),
            disabled: item.disabled,
            checked: item.checked(),
            has_submenu: item.kind() == PopupMenuItemKind::Submenu,
        })
    }

    /// Resolve the retained element identity for an interactive item or structural group label.
    ///
    /// Interactive identity follows the application's stable item ID. A group label has no
    /// command identity, so its accessibility-only identity is derived from the menu root and
    /// bounded structural index. Separators intentionally remain anonymous.
    pub fn item_element_id(
        &self,
        menu_id: impl Into<ElementId>,
        index: usize,
    ) -> Option<ElementId> {
        let item = self.items.get(index)?;
        let menu_id = menu_id.into();
        match (item.id, item.kind()) {
            (Some(item_id), _) => Some(derived_item_id(menu_id, item_id)),
            (None, PopupMenuItemKind::GroupLabel) => Some(derived_group_label_id(menu_id, index)),
            (None, _) => None,
        }
    }

    /// Decorate a caller-owned menu root with focus, keyboard, drag-region, and accessibility
    /// semantics without adding any appearance.
    pub fn root_part(&self, id: impl Into<ElementId>, root: Element) -> Element {
        let id = id.into();
        let mut root = root
            .id(id)
            .track_focus(FocusHandle::new(id))
            .auto_focus()
            .tab_index(-1)
            .key_context(POPUP_MENU_KEY_CONTEXT)
            .accessibility_role(AccessibilityRole::Menu)
            .app_region_no_drag()
            .user_select_none()
            .cursor_default();
        if let Some(active) = self.active
            && let Some(item) = self.item_element_id(id, active)
        {
            root = root.accessibility_active_descendant(item);
        }
        root
    }

    /// Decorate one caller-owned row with its exact menu-item semantics and stable derived ID.
    pub fn item_part(
        &self,
        menu_id: impl Into<ElementId>,
        index: usize,
        item_root: Element,
    ) -> Option<Element> {
        let item = self.items.get(index)?;
        let menu_id = menu_id.into();
        let mut root = item_root
            .app_region_no_drag()
            .user_select_none()
            .cursor_default();
        if item.is_interactive() {
            root = root
                .id(self.item_element_id(menu_id, index)?)
                .clickable()
                .tab_index(-1)
                .disabled(item.disabled)
                .accessibility_label(item.label.clone())
                .cursor_default();
        }
        root = match item.kind() {
            PopupMenuItemKind::Action => root.accessibility_role(AccessibilityRole::MenuItem),
            PopupMenuItemKind::Checkbox => root
                .accessibility_role(AccessibilityRole::MenuItemCheckBox)
                .checked(item.checked().unwrap_or(false)),
            PopupMenuItemKind::Radio => root
                .accessibility_role(AccessibilityRole::MenuItemRadio)
                .checked(item.checked().unwrap_or(false)),
            PopupMenuItemKind::Submenu => root
                .accessibility_role(AccessibilityRole::MenuItem)
                .accessibility_has_popup(AccessibilityPopup::Menu),
            PopupMenuItemKind::Separator => root.accessibility_role(AccessibilityRole::Separator),
            PopupMenuItemKind::GroupLabel => root
                .id(self.item_element_id(menu_id, index)?)
                .accessibility_role(AccessibilityRole::Label)
                .accessibility_label(item.label.clone()),
        };
        Some(root)
    }

    /// Decorate an application-composed wrapper around a related item group.
    pub fn group_part(group: Element) -> Element {
        group
            .accessibility_role(AccessibilityRole::Group)
            .app_region_no_drag()
            .cursor_default()
    }

    /// Decorate a caller-composed item group and name it from one mounted group-label row.
    ///
    /// Returns `None` when `label_index` is not a group label. The caller owns visual nesting and
    /// layout; this helper adds only the native group role and mounted `labelled-by` relationship.
    pub fn labeled_group_part(
        &self,
        menu_id: impl Into<ElementId>,
        label_index: usize,
        group: Element,
    ) -> Option<Element> {
        if self.items.get(label_index)?.kind() != PopupMenuItemKind::GroupLabel {
            return None;
        }
        let label = self.item_element_id(menu_id, label_index)?;
        Some(Self::group_part(group).accessibility_labelled_by(label))
    }

    /// Build the complete unstyled interaction layer for one menu level.
    ///
    /// `render_item` owns all row geometry, colors, indicators, icons, and typography. Commands
    /// dispatch through the popup owner's ordinary typed-action path before `dismiss` runs.
    pub fn element<V, Render, Dismiss>(
        &self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        access: fn(&mut V) -> &mut PopupMenu,
        root: Element,
        render_item: Render,
        dismiss: Dismiss,
    ) -> Element
    where
        V: 'static,
        Render: Fn(&PopupMenuItem, PopupMenuItemState) -> Element,
        Dismiss: Fn(&mut V, &mut EventContext) + Clone + 'static,
    {
        self.element_with_submenus(
            cx,
            id,
            access,
            root,
            render_item,
            dismiss,
            |_view, _anchor, _menu, _cx| {},
        )
    }

    /// Build the complete unstyled interaction layer and expose submenu activation to the owner.
    #[allow(clippy::too_many_arguments)]
    pub fn element_with_submenus<V, Render, Dismiss, OpenSubmenu>(
        &self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        access: fn(&mut V) -> &mut PopupMenu,
        root: Element,
        render_item: Render,
        dismiss: Dismiss,
        open_submenu: OpenSubmenu,
    ) -> Element
    where
        V: 'static,
        Render: Fn(&PopupMenuItem, PopupMenuItemState) -> Element,
        Dismiss: Fn(&mut V, &mut EventContext) + Clone + 'static,
        OpenSubmenu: Fn(&mut V, ElementId, PopupMenu, &mut EventContext) + Clone + 'static,
    {
        self.element_with_submenus_and_hover(
            cx,
            id,
            access,
            root,
            render_item,
            dismiss,
            open_submenu,
            move |view, index, hovered, cx| {
                if hovered && access(view).highlight(index) {
                    cx.invalidate();
                }
            },
        )
    }

    /// Build the complete unstyled interaction layer with owner-controlled hover behavior.
    ///
    /// This is the native-overflow adapter hook for delayed submenu opening and menu-aim safe
    /// corridors. `hover_item` receives entry and exit transitions for enabled interactive rows;
    /// it owns highlighting and any delayed work. The ordinary [`Self::element_with_submenus`]
    /// method preserves immediate-highlight behavior without allocating a timer.
    #[allow(clippy::too_many_arguments)]
    pub fn element_with_submenus_and_hover<V, Render, Dismiss, OpenSubmenu, HoverItem>(
        &self,
        cx: &mut ViewContext<'_, V>,
        id: impl Into<ElementId>,
        access: fn(&mut V) -> &mut PopupMenu,
        root: Element,
        render_item: Render,
        dismiss: Dismiss,
        open_submenu: OpenSubmenu,
        hover_item: HoverItem,
    ) -> Element
    where
        V: 'static,
        Render: Fn(&PopupMenuItem, PopupMenuItemState) -> Element,
        Dismiss: Fn(&mut V, &mut EventContext) + Clone + 'static,
        OpenSubmenu: Fn(&mut V, ElementId, PopupMenu, &mut EventContext) + Clone + 'static,
        HoverItem: Fn(&mut V, usize, bool, &mut EventContext) + Clone + 'static,
    {
        let id = id.into();
        let focus = FocusHandle::new(id);

        let previous = cx.action_listener(id, move |view, _: &PopupMenuPrevious, cx| {
            if access(view).select_previous() {
                cx.invalidate();
            }
        });
        let next = cx.action_listener(id, move |view, _: &PopupMenuNext, cx| {
            if access(view).select_next() {
                cx.invalidate();
            }
        });
        let first = cx.action_listener(id, move |view, _: &PopupMenuFirst, cx| {
            if access(view).select_first() {
                cx.invalidate();
            }
        });
        let last = cx.action_listener(id, move |view, _: &PopupMenuLast, cx| {
            if access(view).select_last() {
                cx.invalidate();
            }
        });

        let activate_dismiss = dismiss.clone();
        let activate_submenu = open_submenu.clone();
        let activate = cx.action_listener(id, move |view, _: &PopupMenuActivate, cx| {
            if let Some(index) = access(view).active_index() {
                handle_activation(
                    view,
                    index,
                    PopupMenuActivationHandler {
                        menu_id: id,
                        focus,
                        access,
                        dismiss: &activate_dismiss,
                        open_submenu: &activate_submenu,
                    },
                    cx,
                );
            }
        });

        let keyboard_submenu = open_submenu.clone();
        let open = cx.action_listener(id, move |view, _: &PopupMenuOpenSubmenu, cx| {
            let Some(index) = access(view).active_index() else {
                return;
            };
            let Some(PopupMenuActivation::Submenu { menu, .. }) = access(view).activate(index)
            else {
                return;
            };
            let Some(anchor) = access(view).item_element_id(id, index) else {
                return;
            };
            keyboard_submenu(view, anchor, menu, cx);
        });

        let close_dismiss = dismiss.clone();
        let close = cx.action_listener(id, move |view, _: &PopupMenuClose, cx| {
            close_dismiss(view, cx);
            cx.invalidate();
        });

        let typeahead = cx.key_down_listener(id, move |view, event, cx| {
            if event
                .modifiers
                .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
            {
                return;
            }
            let key = event.key_char.as_ref().unwrap_or(&event.key);
            let Key::Character(value) = key else {
                return;
            };
            if normalized_typeahead_input(value).is_empty() {
                return;
            }
            if access(view).typeahead(value, Instant::now()) {
                cx.invalidate();
            }
            cx.prevent_default();
            cx.stop_propagation();
        });

        let mut children = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let state = self
                .item_state(index)
                .expect("a retained popup-menu item has render state");
            let mut row = render_item(item, state);
            if item.is_interactive() && !item.disabled {
                let row_id = self
                    .item_element_id(id, index)
                    .expect("an interactive popup-menu item has a stable ID");
                let hover_item = hover_item.clone();
                let hover = cx.hover_listener(row_id, move |view, hovered, cx| {
                    hover_item(view, index, *hovered, cx);
                });
                let click_dismiss = dismiss.clone();
                let click_submenu = open_submenu.clone();
                let click = cx.listener(row_id, move |view, cx| {
                    handle_activation(
                        view,
                        index,
                        PopupMenuActivationHandler {
                            menu_id: id,
                            focus,
                            access,
                            dismiss: &click_dismiss,
                            open_submenu: &click_submenu,
                        },
                        cx,
                    );
                });
                row = row.on_hover(hover).on_click(click);
            }
            children.push(
                self.item_part(id, index, row)
                    .expect("a retained popup-menu index remains valid during rendering"),
            );
        }

        self.root_part(id, root)
            .on_action(previous)
            .on_action(next)
            .on_action(first)
            .on_action(last)
            .on_action(activate)
            .on_action(open)
            .on_action(close)
            .on_key_down(typeahead)
            .children(children)
    }

    fn clear_typeahead(&mut self) {
        self.typeahead.clear();
        self.typeahead_at = None;
    }
}

struct PopupMenuActivationHandler<'a, V, Dismiss, OpenSubmenu> {
    menu_id: ElementId,
    focus: FocusHandle,
    access: fn(&mut V) -> &mut PopupMenu,
    dismiss: &'a Dismiss,
    open_submenu: &'a OpenSubmenu,
}

fn handle_activation<V, Dismiss, OpenSubmenu>(
    view: &mut V,
    index: usize,
    handler: PopupMenuActivationHandler<'_, V, Dismiss, OpenSubmenu>,
    cx: &mut EventContext,
) where
    Dismiss: Fn(&mut V, &mut EventContext),
    OpenSubmenu: Fn(&mut V, ElementId, PopupMenu, &mut EventContext),
{
    let Some(activation) = (handler.access)(view).activate(index) else {
        return;
    };
    match activation {
        PopupMenuActivation::Command {
            action, close_menu, ..
        } => {
            let delivered = if cx.popup_owner_window_handle().is_some() {
                cx.dispatch_any_action_to_popup_owner(action)
            } else {
                cx.dispatch_any_action(action);
                true
            };
            if delivered && close_menu {
                (handler.dismiss)(view, cx);
            } else {
                cx.focus(handler.focus);
            }
            cx.invalidate();
        }
        PopupMenuActivation::Submenu { menu, .. } => {
            let Some(anchor) = (handler.access)(view).item_element_id(handler.menu_id, index)
            else {
                return;
            };
            (handler.open_submenu)(view, anchor, menu, cx);
        }
    }
}

fn selectable(item: &PopupMenuItem) -> bool {
    item.is_interactive() && !item.disabled
}

fn validate_menu_tree(items: &[PopupMenuItem]) -> Result<(), PopupMenuError> {
    let mut count = 0_usize;
    let mut text_bytes = 0_usize;
    validate_menu_level(items, 1, &mut count, &mut text_bytes)
}

fn validate_menu_level(
    items: &[PopupMenuItem],
    depth: usize,
    count: &mut usize,
    text_bytes: &mut usize,
) -> Result<(), PopupMenuError> {
    if depth > MAX_POPUP_MENU_DEPTH {
        return Err(PopupMenuError::TooDeep);
    }
    *count = count.saturating_add(items.len());
    if *count > MAX_POPUP_MENU_ITEMS {
        return Err(PopupMenuError::TooManyItems);
    }

    let mut ids = HashSet::with_capacity(items.len());
    let mut selected_radios = HashMap::new();
    for item in items {
        if let Some(id) = item.id
            && !ids.insert(id)
        {
            return Err(PopupMenuError::DuplicateItemId(id));
        }
        add_text_bytes(item.label.len(), text_bytes)?;
        if let Some(label) = &item.search_label {
            add_text_bytes(label.len(), text_bytes)?;
        }
        if let Some(shortcut) = &item.shortcut {
            add_text_bytes(shortcut.len(), text_bytes)?;
        }
        if let PopupMenuItemContent::Radio {
            group,
            selected: true,
            ..
        } = item.content
            && selected_radios.insert(group, item.id).is_some()
        {
            return Err(PopupMenuError::MultipleSelectedRadioItems(group));
        }
        if let PopupMenuItemContent::Submenu(menu) = &item.content {
            validate_menu_level(&menu.items, depth + 1, count, text_bytes)?;
        }
    }
    Ok(())
}

fn add_text_bytes(bytes: usize, total: &mut usize) -> Result<(), PopupMenuError> {
    if bytes > MAX_POPUP_MENU_ITEM_TEXT_BYTES {
        return Err(PopupMenuError::ItemTextTooLong);
    }
    *total = total.saturating_add(bytes);
    if *total > MAX_POPUP_MENU_TEXT_BYTES {
        return Err(PopupMenuError::TooMuchText);
    }
    Ok(())
}

fn normalized_typeahead_input(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn push_bounded(target: &mut String, value: &str, maximum: usize) {
    if target.len() >= maximum {
        return;
    }
    let remaining = maximum - target.len();
    if value.len() <= remaining {
        target.push_str(value);
        return;
    }
    let mut end = remaining;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    target.push_str(&value[..end]);
}

fn derived_item_id(parent: ElementId, item: ElementId) -> ElementId {
    let mut hash =
        parent.as_u64() ^ ITEM_ID_TAG ^ item.as_u64().wrapping_mul(0x9e37_79b9_7f4a_7c15);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == parent.as_u64() || hash == u64::MAX {
        hash ^= ITEM_ID_TAG.rotate_left(17);
    }
    ElementId::new(hash)
}

fn derived_group_label_id(parent: ElementId, index: usize) -> ElementId {
    debug_assert!(index < MAX_POPUP_MENU_ITEMS);
    let mut hash = parent.as_u64()
        ^ GROUP_LABEL_ID_TAG
        ^ (index as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == 0 || hash == parent.as_u64() || hash == u64::MAX {
        hash ^= GROUP_LABEL_ID_TAG.rotate_left(13);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppRegion, Color, IntoElement, PopupOptions, Rect, TestAppContext, View, WindowOptions,
        div, text,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Command {
        Open,
        Toggle(bool),
        Theme(&'static str),
    }

    fn sample_menu() -> PopupMenu {
        PopupMenu::new([
            PopupMenuItem::group_label("File"),
            PopupMenuItem::action("new", "New", Command::Open).shortcut("⌘N"),
            PopupMenuItem::action("disabled", "Disabled", Command::Open).disabled(true),
            PopupMenuItem::separator(),
            PopupMenuItem::checkbox_with("sidebar", "Sidebar", false, Command::Toggle),
            PopupMenuItem::radio("light", "Light", "theme", true, Command::Theme("light")),
            PopupMenuItem::radio("dark", "Dark", "theme", false, Command::Theme("dark")),
        ])
        .unwrap()
    }

    #[test]
    fn navigation_skips_structure_and_disabled_items_and_loops_by_default() {
        let mut menu = sample_menu();
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "New");
        assert!(menu.select_next());
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "Sidebar");
        assert!(menu.select_last());
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "Dark");
        assert!(menu.select_next());
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "New");

        let mut no_loop = sample_menu().loop_focus(false);
        assert!(no_loop.select_last());
        assert!(!no_loop.select_next());
        assert_eq!(no_loop.active_item().unwrap().label().as_ref(), "Dark");
    }

    #[test]
    fn activation_preserves_typed_payloads_and_local_toggle_preview() {
        let mut menu = sample_menu();
        let sidebar = menu
            .items()
            .iter()
            .position(|item| item.id() == Some("sidebar".into()))
            .unwrap();
        let PopupMenuActivation::Command {
            action, close_menu, ..
        } = menu.activate(sidebar).unwrap()
        else {
            panic!("checkbox activates a command")
        };
        assert_eq!(
            action.downcast_ref::<Command>(),
            Some(&Command::Toggle(true))
        );
        assert!(!close_menu);
        assert_eq!(menu.items()[sidebar].checked(), Some(true));

        let dark = menu
            .items()
            .iter()
            .position(|item| item.id() == Some("dark".into()))
            .unwrap();
        menu.activate(dark).unwrap();
        assert_eq!(menu.items()[dark].checked(), Some(true));
        let light = menu
            .items()
            .iter()
            .position(|item| item.id() == Some("light".into()))
            .unwrap();
        assert_eq!(menu.items()[light].checked(), Some(false));
    }

    #[test]
    fn typeahead_is_bounded_expires_without_a_timer_and_cycles_repeated_letters() {
        let mut menu = PopupMenu::new([
            PopupMenuItem::action("apple", "Apple", Command::Open),
            PopupMenuItem::action("apricot", "Apricot", Command::Open),
            PopupMenuItem::action("banana", "Banana", Command::Open),
        ])
        .unwrap();
        let now = Instant::now();
        assert!(menu.typeahead("a", now));
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "Apricot");
        assert!(menu.typeahead("a", now + Duration::from_millis(20)));
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "Apple");
        assert!(menu.typeahead(
            "b",
            now + Duration::from_millis(20)
                + POPUP_MENU_TYPEAHEAD_TIMEOUT
                + Duration::from_millis(1)
        ));
        assert_eq!(menu.active_item().unwrap().label().as_ref(), "Banana");
        menu.typeahead(&"z".repeat(MAX_POPUP_MENU_TYPEAHEAD_BYTES * 2), now);
        assert!(menu.typeahead.len() <= MAX_POPUP_MENU_TYPEAHEAD_BYTES);
    }

    #[test]
    fn replacement_is_atomic_and_preserves_a_stable_highlight() {
        let mut menu = sample_menu();
        let dark = menu
            .items()
            .iter()
            .position(|item| item.id() == Some("dark".into()))
            .unwrap();
        assert!(menu.highlight(dark));
        assert!(
            !menu
                .set_items([
                    PopupMenuItem::action("new", "New", Command::Open),
                    PopupMenuItem::radio("dark", "Dark", "theme", true, Command::Theme("dark")),
                ])
                .unwrap()
        );
        assert_eq!(menu.active_item().unwrap().id(), Some("dark".into()));

        let before = menu.items().len();
        let error = menu.set_items([
            PopupMenuItem::action("same", "One", Command::Open),
            PopupMenuItem::action("same", "Two", Command::Open),
        ]);
        assert_eq!(error, Err(PopupMenuError::DuplicateItemId("same".into())));
        assert_eq!(menu.items().len(), before);
    }

    #[test]
    fn unstyled_parts_add_behavior_without_appearance() {
        let menu = sample_menu();
        let root = menu.root_part("menu", div());
        assert_eq!(root.accessibility.role, AccessibilityRole::Menu);
        assert!(root.focusable);
        assert!(root.auto_focus);
        assert_eq!(root.app_region, Some(AppRegion::NoDrag));
        assert_eq!(root.visual.background, None);
        assert_eq!(root.visual.border_color, None);

        let action = menu.item_part("menu", 1, div()).unwrap();
        assert_eq!(action.accessibility.role, AccessibilityRole::MenuItem);
        assert!(action.clickable);
        assert_eq!(action.visual.background, None);
        assert_eq!(action.visual.border_color, None);

        let checkbox = menu.item_part("menu", 4, div()).unwrap();
        assert_eq!(
            checkbox.accessibility.role,
            AccessibilityRole::MenuItemCheckBox
        );
        assert_eq!(checkbox.accessibility.toggled, Some(false.into()));

        let group_label_id = menu.item_element_id("menu", 0).unwrap();
        let group_label = menu.item_part("menu", 0, div()).unwrap();
        assert_eq!(group_label.explicit_id, Some(group_label_id));
        assert_eq!(group_label.accessibility.role, AccessibilityRole::Label);
        assert_eq!(group_label.accessibility.label.as_deref(), Some("File"));

        let separator = menu.item_part("menu", 3, div()).unwrap();
        assert_eq!(separator.accessibility.role, AccessibilityRole::Separator);
        assert!(!separator.clickable);
        assert_eq!(menu.item_element_id("menu", 3), None);

        let group = menu
            .labeled_group_part("menu", 0, div())
            .expect("the first row is a group label");
        assert_eq!(group.accessibility.role, AccessibilityRole::Group);
        assert_eq!(
            group.accessibility.relations.labelled_by(),
            Some(group_label_id)
        );
        assert_eq!(group.visual.background, None);
        assert!(menu.labeled_group_part("menu", 1, div()).is_none());

        let styled = menu.root_part("styled", div().bg(Color::BLACK));
        assert_eq!(styled.visual.background, Some(Color::BLACK));
    }

    #[test]
    fn nested_validation_counts_the_complete_tree() {
        let mut menu =
            PopupMenu::new([PopupMenuItem::action("leaf", "Leaf", Command::Open)]).unwrap();
        for depth in 1..MAX_POPUP_MENU_DEPTH {
            menu = PopupMenu::new([PopupMenuItem::submenu(depth, "More", menu)]).unwrap();
        }
        assert_eq!(
            PopupMenu::new([PopupMenuItem::submenu("too-deep", "More", menu)]).unwrap_err(),
            PopupMenuError::TooDeep
        );
    }

    #[derive(Default)]
    struct CommandOwner {
        received: usize,
    }

    impl View for CommandOwner {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let command = cx.action_listener("command-owner", |view, _: &Command, cx| {
                view.received += 1;
                cx.invalidate();
            });
            div()
                .focus_scope(cx.focus_handle("command-owner"))
                .on_action(command)
                .child(div().id("owner-focus").focusable().auto_focus())
        }
    }

    struct IntermediatePopup;

    impl View for IntermediatePopup {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div().size_full()
        }
    }

    struct MenuPopup {
        menu: PopupMenu,
    }

    impl MenuPopup {
        fn new() -> Self {
            Self {
                menu: PopupMenu::new([PopupMenuItem::action("open", "Open", Command::Open)])
                    .unwrap(),
            }
        }
    }

    impl View for MenuPopup {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            self.menu.element(
                cx,
                "menu-popup",
                |view| &mut view.menu,
                div().size_full().flex_col(),
                |item, state| {
                    div()
                        .h(32.0)
                        .w_full()
                        .child(text(item.label().clone()))
                        .opacity(if state.highlighted { 1.0 } else { 0.9 })
                },
                |_view, cx| {
                    assert!(cx.close_popup_chain());
                },
            )
        }
    }

    #[test]
    fn nested_popup_menu_commands_reach_the_non_popup_owner_before_close() {
        let (mut cx, owner) = TestAppContext::new(CommandOwner::default()).unwrap();
        let first = cx
            .update(owner, |_view, cx| {
                cx.open_window(
                    IntermediatePopup,
                    WindowOptions::new("First popup")
                        .size(160.0, 100.0)
                        .anchored_popup(PopupOptions::new(Rect::new(10.0, 10.0, 20.0, 20.0))),
                )
            })
            .unwrap();
        let first = cx.typed_window::<IntermediatePopup>(first).unwrap();
        let second = cx
            .update(first, |_view, cx| {
                cx.open_window(
                    MenuPopup::new(),
                    WindowOptions::new("Nested menu")
                        .size(160.0, 100.0)
                        .anchored_popup(PopupOptions::new(Rect::new(20.0, 20.0, 20.0, 20.0))),
                )
            })
            .unwrap();
        let item_id = MenuPopup::new()
            .menu
            .item_element_id("menu-popup", 0)
            .unwrap();

        cx.click(second, item_id).unwrap();

        assert!(!cx.is_window_open(second));
        assert_eq!(cx.read(owner, |view| view.received).unwrap(), 1);
        assert!(!cx.is_window_open(first.window_handle()));
    }
}
