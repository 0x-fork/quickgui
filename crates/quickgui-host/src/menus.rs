//! The Base UI-aligned in-window `Menu` compound bound to the Rust core.
//!
//! [`quickgui::MenuState`] is the surface — the controlled open flag, Base UI's `Menu.Root` props,
//! the anchored placement its popup uses, and the exact hover deadlines a `openOnHover` trigger or
//! a submenu trigger needs. [`quickgui::PopoverMenu`] is the row model — highlighting, typeahead,
//! toggle policy, radio groups, and activation. This module only translates a declaration into
//! both and turns whatever the core decided into an asynchronous event.
//!
//! Unlike the JSON-`items` [`super::popover_menu`] binding, which hands the core a bounded model
//! it paints itself, the compound declares its rows as ordinary child nodes: the application owns
//! every pixel of a row and the core owns the row's identity, semantics, keyboard behavior, and
//! activation. The rows are gathered once per frame in declaration order, so a row's index — and
//! therefore its derived identity — never depends on a JavaScript registry.

use super::*;

use std::time::Duration;

/// Declared part name of a menu root.
pub(super) const MENU_PART: &str = "menu";
/// Declared part name of a menu trigger.
pub(super) const MENU_TRIGGER_PART: &str = "menu-trigger";
/// Declared part name of a menu portal boundary.
pub(super) const MENU_PORTAL_PART: &str = "menu-portal";
/// Declared part name of a menu backdrop.
pub(super) const MENU_BACKDROP_PART: &str = "menu-backdrop";
/// Declared part name of a menu positioner.
pub(super) const MENU_POSITIONER_PART: &str = "menu-positioner";
/// Declared part name of a menu popup.
pub(super) const MENU_POPUP_PART: &str = "menu-popup";
/// Declared part name of a menu arrow.
pub(super) const MENU_ARROW_PART: &str = "menu-arrow";
/// Declared part name of a command row.
pub(super) const MENU_ITEM_PART: &str = "menu-item";
/// Declared part name of a link row.
pub(super) const MENU_LINK_ITEM_PART: &str = "menu-link-item";
/// Declared part name of a nested level's structural wrapper.
pub(super) const MENU_SUBMENU_ROOT_PART: &str = "menu-submenu-root";
/// Declared part name of a submenu trigger row.
pub(super) const MENU_SUBMENU_TRIGGER_PART: &str = "menu-submenu-trigger";
/// Declared part name of an item group.
pub(super) const MENU_GROUP_PART: &str = "menu-group";
/// Declared part name of a group label row.
pub(super) const MENU_GROUP_LABEL_PART: &str = "menu-group-label";
/// Declared part name of a radio group.
pub(super) const MENU_RADIO_GROUP_PART: &str = "menu-radio-group";
/// Declared part name of a radio row.
pub(super) const MENU_RADIO_ITEM_PART: &str = "menu-radio-item";
/// Declared part name of a radio row's indicator.
pub(super) const MENU_RADIO_ITEM_INDICATOR_PART: &str = "menu-radio-item-indicator";
/// Declared part name of a checkbox row.
pub(super) const MENU_CHECKBOX_ITEM_PART: &str = "menu-checkbox-item";
/// Declared part name of a checkbox row's indicator.
pub(super) const MENU_CHECKBOX_ITEM_INDICATOR_PART: &str = "menu-checkbox-item-indicator";
/// Declared part name of a divider row.
pub(super) const MENU_SEPARATOR_PART: &str = "menu-separator";

/// Whether one declared part belongs to this module.
pub(super) fn owns_menu_part(part: &str) -> bool {
    matches!(
        part,
        MENU_PART
            | MENU_TRIGGER_PART
            | MENU_PORTAL_PART
            | MENU_BACKDROP_PART
            | MENU_POSITIONER_PART
            | MENU_POPUP_PART
            | MENU_ARROW_PART
            | MENU_ITEM_PART
            | MENU_LINK_ITEM_PART
            | MENU_SUBMENU_ROOT_PART
            | MENU_SUBMENU_TRIGGER_PART
            | MENU_GROUP_PART
            | MENU_GROUP_LABEL_PART
            | MENU_RADIO_GROUP_PART
            | MENU_RADIO_ITEM_PART
            | MENU_RADIO_ITEM_INDICATOR_PART
            | MENU_CHECKBOX_ITEM_PART
            | MENU_CHECKBOX_ITEM_INDICATOR_PART
            | MENU_SEPARATOR_PART
    )
}

/// Whether the core owns one part's own click activation.
///
/// A duplicate listener is made impossible rather than fatal: the declared `onClick` is dropped
/// and the activation the core decided is reported instead.
pub(super) fn menu_part_owns_click(part: &str) -> bool {
    matches!(
        part,
        MENU_TRIGGER_PART
            | MENU_ITEM_PART
            | MENU_LINK_ITEM_PART
            | MENU_CHECKBOX_ITEM_PART
            | MENU_RADIO_ITEM_PART
            | MENU_SUBMENU_TRIGGER_PART
    )
}

/// Whether the core owns one part's own Escape and outside-press dismissal.
pub(super) fn menu_part_owns_dismiss(part: &str) -> bool {
    matches!(part, MENU_POPUP_PART)
}

/// Whether one declared part is a row of its enclosing popup.
fn is_menu_row_part(part: &str) -> bool {
    matches!(
        part,
        MENU_ITEM_PART
            | MENU_LINK_ITEM_PART
            | MENU_CHECKBOX_ITEM_PART
            | MENU_RADIO_ITEM_PART
            | MENU_SUBMENU_TRIGGER_PART
            | MENU_GROUP_LABEL_PART
            | MENU_SEPARATOR_PART
    )
}

// ---------------------------------------------------------------------------
// Derived identities
// ---------------------------------------------------------------------------

fn derived(root: ElementId, rotate: u32, salt: u64) -> ElementId {
    ElementId::new(root.as_u64().rotate_left(rotate) ^ salt)
}

pub(super) fn menu_trigger_id(root: ElementId) -> ElementId {
    derived(root, 19, 0x9e37_79b9_7f4a_7c15)
}

pub(super) fn menu_popup_id(root: ElementId) -> ElementId {
    derived(root, 47, 0xc2b2_ae3d_27d4_eb4f)
}

// ---------------------------------------------------------------------------
// Declared rows
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuRowKind {
    Item,
    Link,
    Checkbox,
    Radio,
    Submenu,
    Separator,
    GroupLabel,
}

impl MenuRowKind {
    fn from_part(part: &str) -> Option<Self> {
        Some(match part {
            MENU_ITEM_PART => Self::Item,
            MENU_LINK_ITEM_PART => Self::Link,
            MENU_CHECKBOX_ITEM_PART => Self::Checkbox,
            MENU_RADIO_ITEM_PART => Self::Radio,
            MENU_SUBMENU_TRIGGER_PART => Self::Submenu,
            MENU_SEPARATOR_PART => Self::Separator,
            MENU_GROUP_LABEL_PART => Self::GroupLabel,
            _ => return None,
        })
    }
}

/// One declared row of one menu level.
#[derive(Clone, Debug)]
struct DeclaredMenuRow {
    node: u32,
    kind: MenuRowKind,
    id: Arc<str>,
    label: Arc<str>,
    disabled: bool,
    checked: bool,
    close_on_click: Option<bool>,
    /// The radio group this row belongs to, and the node that declared it.
    group: Option<Arc<str>>,
    group_node: Option<u32>,
    href: Option<Arc<str>>,
    /// The scope of the nested level a submenu trigger opens.
    submenu: Option<u64>,
    listens: bool,
    clicks: bool,
}

/// Where one declared row sits in its enclosing level's retained model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MenuRowBinding {
    /// The scope key of the level this row belongs to.
    pub(super) menu: u64,
    /// The row's index in that level's retained model.
    pub(super) index: usize,
}

fn bounded_text(node: &NativeNode, key: u16) -> Option<Arc<str>> {
    node.string(key)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(Arc::from)
}

fn bounded_label(value: &str) -> Arc<str> {
    if value.len() <= quickgui::MAX_POPOVER_MENU_ITEM_TEXT_BYTES {
        return Arc::from(value);
    }
    let mut end = quickgui::MAX_POPOVER_MENU_ITEM_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&value[..end])
}

/// The declared destination of a `Menu.LinkItem`, bounded exactly as the core bounds it.
fn bounded_href(node: &NativeNode) -> Option<Arc<str>> {
    node.string(property::HREF)
        .filter(|value| !value.is_empty() && value.len() <= quickgui::MAX_POPOVER_MENU_LINK_BYTES)
        .map(Arc::from)
}

/// Gather the declared rows of one popup, in declaration order.
///
/// The walk stops at a nested popup: a submenu is its own level with its own model, reached
/// through the submenu trigger row rather than by flattening the two together.
fn gather_rows(popup: u32, tree: &NativeTree) -> Vec<DeclaredMenuRow> {
    let Some(root) = tree.nodes.get(&popup) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    // Depth-first in declaration order, carrying the enclosing radio group down the walk.
    let mut pending = root
        .children
        .iter()
        .rev()
        .map(|id| (*id, None, None))
        .collect::<Vec<(u32, Option<Arc<str>>, Option<u32>)>>();
    while let Some((id, group, group_node)) = pending.pop() {
        if rows.len() >= quickgui::MAX_POPOVER_MENU_ITEMS {
            break;
        }
        let Some(node) = tree.nodes.get(&id) else {
            continue;
        };
        let part = node.string(property::PART).unwrap_or("");
        if part == MENU_POPUP_PART {
            continue;
        }
        if let Some(kind) = MenuRowKind::from_part(part) {
            if let Some(row) = declared_row(id, node, kind, group.clone(), group_node) {
                rows.push(row);
            }
            continue;
        }
        let (group, group_node) = if part == MENU_RADIO_GROUP_PART {
            (
                bounded_text(node, property::PART_VALUE)
                    .or_else(|| Some(Arc::from(format!("menu-radio-group-{id}").as_str()))),
                Some(id),
            )
        } else {
            (group, group_node)
        };
        pending.extend(
            node.children
                .iter()
                .rev()
                .map(|child| (*child, group.clone(), group_node)),
        );
    }
    rows
}

fn declared_row(
    id: u32,
    node: &NativeNode,
    kind: MenuRowKind,
    group: Option<Arc<str>>,
    group_node: Option<u32>,
) -> Option<DeclaredMenuRow> {
    let label = bounded_text(node, property::ACCESSIBILITY_LABEL)
        .or_else(|| bounded_text(node, property::VALUE))
        .unwrap_or_else(|| Arc::from(""));
    // Every interactive row needs a stable identity; the declared value is it, and a row without
    // one declares nothing rather than reaching a constructor that would reject the whole level.
    let declared_id = bounded_text(node, property::PART_VALUE);
    let id_value = match kind {
        MenuRowKind::Separator | MenuRowKind::GroupLabel => {
            declared_id.unwrap_or_else(|| Arc::from(""))
        }
        _ => declared_id?,
    };
    if matches!(
        kind,
        MenuRowKind::Item
            | MenuRowKind::Link
            | MenuRowKind::Checkbox
            | MenuRowKind::Radio
            | MenuRowKind::Submenu
    ) && label.is_empty()
    {
        return None;
    }
    Some(DeclaredMenuRow {
        node: id,
        kind,
        id: id_value,
        label: bounded_label(&label),
        disabled: node.boolean(property::DISABLED).unwrap_or(false),
        checked: node.boolean(property::CHECKED).unwrap_or(false),
        close_on_click: node.boolean(property::CLOSE_ON_CLICK),
        group: match kind {
            MenuRowKind::Radio => group.or_else(|| bounded_text(node, property::GROUP)),
            _ => None,
        },
        group_node,
        href: match kind {
            MenuRowKind::Link => bounded_href(node),
            _ => None,
        },
        submenu: match kind {
            MenuRowKind::Submenu => Some(component_key(id, node)),
            _ => None,
        },
        listens: declares_change(node),
        clicks: node.boolean(property::CLICK_LISTENER).unwrap_or(false),
    })
}

/// A fingerprint of everything one level's retained model is built from.
fn rows_declaration(rows: &[DeclaredMenuRow]) -> Arc<str> {
    let mut fingerprint = String::new();
    for row in rows {
        fingerprint.push('\u{1f}');
        fingerprint.push_str(match row.kind {
            MenuRowKind::Item => "item",
            MenuRowKind::Link => "link",
            MenuRowKind::Checkbox => "checkbox",
            MenuRowKind::Radio => "radio",
            MenuRowKind::Submenu => "submenu",
            MenuRowKind::Separator => "separator",
            MenuRowKind::GroupLabel => "group-label",
        });
        fingerprint.push('\u{1e}');
        fingerprint.push_str(&row.id);
        fingerprint.push('\u{1e}');
        fingerprint.push_str(&row.label);
        fingerprint.push('\u{1e}');
        fingerprint.push_str(if row.disabled { "1" } else { "0" });
        fingerprint.push('\u{1e}');
        fingerprint.push_str(row.group.as_deref().unwrap_or(""));
        fingerprint.push('\u{1e}');
        fingerprint.push_str(row.href.as_deref().unwrap_or(""));
    }
    Arc::from(fingerprint.as_str())
}

/// A unit action for a declared command row.
///
/// The binding activates rows through the model itself, so the payload the core carries is never
/// dispatched anywhere: the activation edge is reported from the same listener that produced it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DeclaredMenuCommand;

fn menu_items(rows: &[DeclaredMenuRow]) -> Vec<PopoverMenuItem> {
    rows.iter()
        .map(|row| {
            let mut item = match row.kind {
                MenuRowKind::Separator => PopoverMenuItem::separator(),
                MenuRowKind::GroupLabel => PopoverMenuItem::group_label(Arc::clone(&row.label)),
                MenuRowKind::Link => PopoverMenuItem::link(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    row.href.clone().unwrap_or_else(|| Arc::from("")),
                ),
                MenuRowKind::Checkbox => PopoverMenuItem::checkbox(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    row.checked,
                    DeclaredMenuCommand,
                ),
                MenuRowKind::Radio => PopoverMenuItem::radio(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    ElementId::named(row.group.as_deref().unwrap_or(row.id.as_ref())),
                    row.checked,
                    DeclaredMenuCommand,
                ),
                // The nested level is its own declared compound, so the model row carries an empty
                // submenu: it exists to give the row `menuitem` plus `has-popup` semantics and to
                // make Right open the level the application really declared.
                MenuRowKind::Submenu => PopoverMenuItem::submenu(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    PopoverMenu::new(Vec::new()).expect("an empty submenu is always valid"),
                ),
                MenuRowKind::Item => PopoverMenuItem::action(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    DeclaredMenuCommand,
                ),
            };
            item = item.disabled(row.disabled);
            if let Some(close) = row.close_on_click {
                item = item.close_on_activate(close);
            }
            item
        })
        .collect()
}

/// Build the core rows a cursor-point context menu paints from its declared child item parts.
///
/// The core renders a context menu in its own window from the bounded appearance declaration, so
/// these rows contribute a model rather than elements: every declared `Menu.*` row node under a
/// `ContextMenu.Trigger` unmounts and its facts arrive here instead. Nested levels are not part of
/// this shape — a context menu with submenus declares them through the `items` model.
pub(super) fn context_menu_rows(trigger: u32, tree: &NativeTree) -> Vec<PopoverMenuItem> {
    gather_rows(trigger, tree)
        .into_iter()
        .filter(|row| row.kind != MenuRowKind::Submenu)
        .map(|row| {
            let node = row.node;
            let value = Arc::clone(&row.id);
            let mut item = match row.kind {
                MenuRowKind::Separator => PopoverMenuItem::separator(),
                MenuRowKind::GroupLabel => PopoverMenuItem::group_label(Arc::clone(&row.label)),
                MenuRowKind::Link => PopoverMenuItem::link(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    row.href.clone().unwrap_or_else(|| Arc::from("")),
                ),
                MenuRowKind::Checkbox => PopoverMenuItem::checkbox_with(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    row.checked,
                    move |checked| NativeMenuSelect {
                        node,
                        item: Arc::clone(&value),
                        checked: Some(checked),
                    },
                ),
                MenuRowKind::Radio => PopoverMenuItem::radio(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    ElementId::named(row.group.as_deref().unwrap_or(row.id.as_ref())),
                    row.checked,
                    NativeMenuSelect {
                        node,
                        item: Arc::clone(&row.id),
                        checked: Some(true),
                    },
                ),
                MenuRowKind::Item | MenuRowKind::Submenu => PopoverMenuItem::action(
                    ElementId::named(row.id.as_ref()),
                    Arc::clone(&row.label),
                    NativeMenuSelect {
                        node,
                        item: Arc::clone(&row.id),
                        checked: None,
                    },
                ),
            };
            item = item.disabled(row.disabled);
            if let Some(close) = row.close_on_click {
                item = item.close_on_activate(close);
            }
            item
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Retained instances
// ---------------------------------------------------------------------------

/// One retained menu level.
pub(super) struct NativeMenuInstance {
    pub(super) state: MenuState,
    pub(super) menu: PopoverMenu,
    rows: Vec<DeclaredMenuRow>,
    declaration: Arc<str>,
    declared_open: bool,
    trigger_node: u32,
    listens: bool,
    reported_open: bool,
    reported_placement: Option<serde_json::Value>,
    reported_rows: HashMap<u32, serde_json::Value>,
}

impl NativeMenuInstance {
    fn new(trigger: ElementId, popup: ElementId) -> Self {
        Self {
            state: MenuState::new(trigger, popup),
            menu: PopoverMenu::new(Vec::new()).expect("an empty menu is always valid"),
            rows: Vec::new(),
            declaration: Arc::from(""),
            declared_open: false,
            trigger_node: 0,
            listens: false,
            reported_open: false,
            reported_placement: None,
            reported_rows: HashMap::new(),
        }
    }

    fn row_at(&self, index: usize) -> Option<&DeclaredMenuRow> {
        self.rows.get(index)
    }
}

/// Every declared menu level this window retains.
#[derive(Default)]
pub(super) struct NativeMenuCompounds {
    pub(super) menus: HashMap<u64, NativeMenuInstance>,
    /// Where each declared row node sits in its level's retained model.
    pub(super) rows: HashMap<u32, MenuRowBinding>,
}

impl NativeMenuCompounds {
    /// The identity one declared row node mounts under, resolved from its level's model.
    fn row_element_id(&self, node: u32) -> Option<ElementId> {
        let binding = self.rows.get(&node)?;
        let instance = self.menus.get(&binding.menu)?;
        instance
            .menu
            .item_element_id(menu_popup_id(ElementId::new(binding.menu)), binding.index)
    }

    /// This level followed by every menu level whose submenu row owns it.
    fn menu_chain_to_root(&self, menu: u64) -> Vec<u64> {
        let mut chain = Vec::new();
        let mut current = Some(menu);
        while let Some(key) = current {
            if chain.contains(&key) {
                break;
            }
            chain.push(key);
            current = self.menus.iter().find_map(|(parent, instance)| {
                instance
                    .rows
                    .iter()
                    .any(|row| row.submenu == Some(key))
                    .then_some(*parent)
            });
        }
        chain
    }
}

/// A per-instance accessor from the hosted view to one declared menu level's surface state.
fn menu_accessor(key: u64) -> StateAccessor<NativeView, MenuState> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view
            .components
            .menu_compound
            .menus
            .entry(key)
            .or_insert_with(|| {
                let root = ElementId::new(key);
                NativeMenuInstance::new(menu_trigger_id(root), menu_popup_id(root))
            })
            .state
    })
}

/// Read one bounded declared millisecond deadline that may be absent entirely.
fn optional_deadline(node: &NativeNode, key: u16, limit: Duration) -> Option<Duration> {
    node.number(key)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| Duration::from_secs_f32(value / 1000.0).min(limit))
}

fn declared_length(node: &NativeNode, key: u16, limit: f32) -> Option<f32> {
    node.number(key)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(-limit, limit))
}

impl NativeComponentStates {
    /// Reseed every declared menu level and report what the core decided.
    pub(super) fn sync_menus(&mut self, tree: &NativeTree, window: u32, events: &EventQueue) {
        // 1. Gather the declared rows of every mounted popup, in declaration order.
        let mut declared_rows: HashMap<u64, Vec<DeclaredMenuRow>> = HashMap::new();
        let mut radio_values: HashMap<u32, Arc<str>> = HashMap::new();
        let mut triggers: Vec<(u64, u32, bool)> = Vec::new();
        for (id, node) in &tree.nodes {
            let Some(part) = node.string(property::PART) else {
                continue;
            };
            match part {
                MENU_POPUP_PART if declared_rows.len() < MAX_COMPONENT_INSTANCES => {
                    declared_rows.insert(component_key(*id, node), gather_rows(*id, tree));
                }
                MENU_TRIGGER_PART | MENU_SUBMENU_TRIGGER_PART
                    if triggers.len() < MAX_COMPONENT_INSTANCES =>
                {
                    triggers.push((
                        component_key(*id, node),
                        *id,
                        part == MENU_SUBMENU_TRIGGER_PART,
                    ));
                }
                MENU_RADIO_GROUP_PART => {
                    if let Some(value) = bounded_text(node, property::ACTIVE_VALUE) {
                        radio_values.insert(*id, value);
                    }
                }
                _ => {}
            }
        }

        // 2. Rebuild each level's retained row model. The highlighted stable identity survives an
        //    atomic replacement, so a re-render never loses the roving highlight.
        let mut rows = HashMap::new();
        for (key, level) in &declared_rows {
            for (index, row) in level.iter().enumerate() {
                rows.insert(row.node, MenuRowBinding { menu: *key, index });
            }
            let root = ElementId::new(*key);
            let instance = self.menu_compound.menus.entry(*key).or_insert_with(|| {
                NativeMenuInstance::new(menu_trigger_id(root), menu_popup_id(root))
            });
            let declaration = rows_declaration(level);
            if instance.declaration != declaration
                && instance.menu.set_items(menu_items(level)).is_ok()
            {
                instance.declaration = declaration;
            }
            instance.rows.clone_from(level);
            // A controlled `Menu.RadioGroup` value wins: exactly one row in the group stays
            // checked and the core unchecks every other one in the same update.
            for row in level {
                if row.kind != MenuRowKind::Radio {
                    continue;
                }
                let Some(group) = row.group.as_deref() else {
                    continue;
                };
                let Some(value) = row.group_node.and_then(|node| radio_values.get(&node)) else {
                    continue;
                };
                instance
                    .menu
                    .set_radio_value(ElementId::named(group), ElementId::named(value.as_ref()));
            }
        }
        self.menu_compound.rows = rows;

        // 3. Reseed each level's surface from its trigger, which is the one part mounted whether
        //    the level is open or closed and therefore carries the whole declaration.
        let mut live = HashSet::new();
        for (key, id, submenu) in &triggers {
            live.insert(*key);
            let Some(node) = tree.nodes.get(id) else {
                continue;
            };
            let trigger_id = if *submenu {
                self.menu_compound
                    .row_element_id(*id)
                    .unwrap_or_else(|| menu_trigger_id(ElementId::new(*key)))
            } else {
                menu_trigger_id(ElementId::new(*key))
            };
            self.sync_menu(*key, *id, trigger_id, node);
        }
        self.menu_compound
            .menus
            .retain(|key, _| live.contains(key) || declared_rows.contains_key(key));
        // A level whose popup is unmounted keeps no rows: the nodes that declared them are gone,
        // so reporting their state would address a node the hosted tree no longer holds.
        for (key, instance) in &mut self.menu_compound.menus {
            if !declared_rows.contains_key(key) {
                instance.rows.clear();
                instance.reported_rows.clear();
            }
        }
        self.report_menus(window, events);
    }

    fn sync_menu(&mut self, key: u64, id: u32, trigger_id: ElementId, node: &NativeNode) {
        let root = ElementId::new(key);
        let popup_id = menu_popup_id(root);
        let instance = self
            .menu_compound
            .menus
            .entry(key)
            .or_insert_with(|| NativeMenuInstance::new(trigger_id, popup_id));
        // A submenu trigger's identity is a row of its parent level, so it only becomes final once
        // the parent model exists; rebuilding the surface on that edge keeps the anchor correct.
        if instance.state.trigger_id() != trigger_id {
            instance.state = MenuState::new(trigger_id, popup_id);
            instance.reported_open = false;
            instance.reported_placement = None;
        }
        instance.trigger_node = id;
        instance.listens = declares_change(node);

        let declared_open = node.boolean(property::OPEN).unwrap_or(false);
        let mut state =
            std::mem::replace(&mut instance.state, MenuState::new(trigger_id, popup_id));
        state = state
            .modal(node.boolean(property::MODAL).unwrap_or(false))
            .loop_focus(node.boolean(property::LOOP_FOCUS).unwrap_or(true))
            .close_parent_on_esc(node.boolean(property::CLOSE_PARENT_ON_ESC).unwrap_or(false))
            .disabled(node.boolean(property::DISABLED).unwrap_or(false))
            .open_on_hover(node.boolean(property::OPEN_ON_HOVER).unwrap_or(false))
            .hoverable_popup(node.boolean(property::HOVERABLE).unwrap_or(true))
            .sticky(node.boolean(property::STICKY).unwrap_or(true))
            .side(declared_side(node))
            .align(declared_align(node));
        if let Some(delay) = optional_deadline(node, property::DELAY, MAX_POPOVER_HOVER_DELAY) {
            state = state.delay(delay);
        }
        if let Some(close_delay) =
            optional_deadline(node, property::CLOSE_DELAY, MAX_POPOVER_HOVER_DELAY)
        {
            state = state.close_delay(close_delay);
        }
        if let Some(offset) = declared_length(node, property::SIDE_OFFSET, MAX_POPOVER_SIDE_OFFSET)
        {
            state = state.side_offset(offset);
        }
        if let Some(offset) =
            declared_length(node, property::ALIGN_OFFSET, MAX_POPOVER_ALIGN_OFFSET)
        {
            state = state.align_offset(offset);
        }
        if let Some(padding) = declared_length(
            node,
            property::COLLISION_PADDING,
            MAX_POPOVER_COLLISION_PADDING,
        ) {
            state = state.collision_padding(padding);
        }
        // A controlled `open` the application committed wins over an outstanding hover deadline;
        // the core cancels the pending task so a click and a hover can never fight.
        if instance.declared_open != declared_open {
            instance.declared_open = declared_open;
            instance.reported_open = declared_open;
            if declared_open {
                state.open_now();
            } else {
                state.close_now();
            }
        }
        let orientation = match node.string(property::ORIENTATION) {
            Some("horizontal") => MenuOrientation::Horizontal,
            _ => MenuOrientation::Vertical,
        };
        state = state.orientation(orientation);
        instance.state = state;
        instance.menu.set_orientation(orientation);
        let loop_focus = node.boolean(property::LOOP_FOCUS).unwrap_or(true);
        if instance.menu.loops_focus() != loop_focus {
            let menu = std::mem::replace(
                &mut instance.menu,
                PopoverMenu::new(Vec::new()).expect("an empty menu is always valid"),
            );
            instance.menu = menu.loop_focus(loop_focus);
        }
    }

    /// Enqueue one asynchronous change event per level and row the core moved.
    fn report_menus(&mut self, window: u32, events: &EventQueue) {
        // A submenu trigger's `open` is the nested level's own open flag, so the levels are read
        // once before the report walk mutates them.
        let open_levels = self
            .menu_compound
            .menus
            .iter()
            .map(|(key, instance)| (*key, instance.state.is_open()))
            .collect::<HashMap<u64, bool>>();
        for instance in self.menu_compound.menus.values_mut() {
            let core = instance.state.state();
            let open = core.open;
            let placement = serde_json::json!({
                "open": open,
                "side": side_name(core.side),
                "align": align_name(core.align),
                "anchorHidden": core.anchor_hidden,
            });
            if instance.reported_open != open
                || instance.reported_placement.as_ref() != Some(&placement)
            {
                instance.reported_open = open;
                instance.reported_placement = Some(placement.clone());
                if instance.listens {
                    enqueue_component_change(events, window, instance.trigger_node, placement);
                }
            }
            let mut reported = HashMap::with_capacity(instance.rows.len());
            for (index, row) in instance.rows.iter().enumerate() {
                let submenu_open = row
                    .submenu
                    .and_then(|child| open_levels.get(&child).copied())
                    .unwrap_or(false);
                let Some(state) = instance.menu.item_part_state(index, submenu_open) else {
                    continue;
                };
                let value = serde_json::json!({
                    "highlighted": state.highlighted,
                    "disabled": state.disabled,
                    "checked": state.checked,
                    "open": state.open,
                });
                if row.listens && instance.reported_rows.get(&row.node) != Some(&value) {
                    enqueue_component_change(events, window, row.node, value.clone());
                }
                reported.insert(row.node, value);
            }
            instance.reported_rows = reported;
        }
    }
}

fn side_name(side: AnchorSide) -> &'static str {
    match side {
        AnchorSide::Top => "top",
        AnchorSide::Bottom => "bottom",
        AnchorSide::Left => "left",
        AnchorSide::Right => "right",
    }
}

fn align_name(align: AnchorAlign) -> &'static str {
    match align {
        AnchorAlign::Start => "start",
        AnchorAlign::Center => "center",
        AnchorAlign::End => "end",
    }
}

/// The core-derived identity one declared menu part mounts under.
pub(super) fn menu_part_element_id(
    part: &str,
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
) -> Option<ElementId> {
    let key = component_key(id, node);
    let root = ElementId::new(key);
    let compound = &components.menu_compound;
    let instance = compound.menus.get(&key);
    Some(match part {
        // A root, group, or radio group is a caller-owned wrapper the core decorates without
        // claiming an identity, so it keeps the ordinary node identity and two of them declared
        // under one scope stay distinct.
        MENU_PART | MENU_SUBMENU_ROOT_PART | MENU_GROUP_PART | MENU_RADIO_GROUP_PART => {
            return None;
        }
        MENU_TRIGGER_PART => menu_trigger_id(root),
        MENU_POPUP_PART => menu_popup_id(root),
        MENU_PORTAL_PART | MENU_POSITIONER_PART => {
            instance.map_or(root, |instance| instance.state.positioner_id())
        }
        MENU_BACKDROP_PART => instance.map_or(root, |instance| instance.state.backdrop_id()),
        MENU_ARROW_PART => instance.map_or(root, |instance| instance.state.arrow_id()),
        MENU_SUBMENU_TRIGGER_PART => compound.row_element_id(id).unwrap_or(menu_trigger_id(root)),
        part if is_menu_row_part(part) => compound.row_element_id(id)?,
        _ => return None,
    })
}

/// Apply one declared menu part.
///
/// `None` means the core decided this part is not mounted at all — every surface part of a closed
/// level, and every row whose declaration the core refused.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_menu_part(
    element: Element,
    part: &str,
    id: u32,
    window: u32,
    node: &NativeNode,
    events: &EventQueue,
    components: &NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Option<Element> {
    let key = component_key(id, node);
    match part {
        // Base UI's `Menu.Root` and `Menu.SubmenuRoot` are logical coordinators, so a caller-owned
        // wrapper declared under either name is decorated but never unmounted.
        MENU_PART | MENU_SUBMENU_ROOT_PART => {
            Some(match components.menu_compound.menus.get(&key) {
                Some(instance) => instance.state.root_part(element),
                None => element,
            })
        }
        MENU_GROUP_PART => Some(PopoverMenu::group_part(element)),
        MENU_RADIO_GROUP_PART => Some(PopoverMenu::radio_group_part(element)),
        MENU_CHECKBOX_ITEM_INDICATOR_PART => {
            Some(PopoverMenu::checkbox_item_indicator_part(element))
        }
        MENU_RADIO_ITEM_INDICATOR_PART => Some(PopoverMenu::radio_item_indicator_part(element)),
        MENU_TRIGGER_PART => {
            let Some(instance) = components.menu_compound.menus.get(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(element.id(instance.state.trigger_id()));
            }
            Some(instance.state.trigger_part_with(
                cx,
                menu_accessor(key),
                open_change(Rc::clone(events), window, instance.trigger_node),
                element,
            ))
        }
        MENU_PORTAL_PART | MENU_POSITIONER_PART => {
            let instance = components.menu_compound.menus.get(&key)?;
            if !instance.state.is_open() {
                return None;
            }
            Some(instance.state.positioner_part(element))
        }
        MENU_BACKDROP_PART => {
            let instance = components.menu_compound.menus.get(&key)?;
            if !instance.state.is_open() {
                return None;
            }
            Some(instance.state.backdrop_part(element))
        }
        MENU_ARROW_PART => {
            let instance = components.menu_compound.menus.get(&key)?;
            if !instance.state.is_open() {
                return None;
            }
            Some(instance.state.arrow_part(element))
        }
        MENU_POPUP_PART => {
            let instance = components.menu_compound.menus.get(&key)?;
            if !instance.state.is_open() {
                return None;
            }
            let popup = if listeners_enabled {
                instance.state.popup_part_with(
                    cx,
                    menu_accessor(key),
                    open_change(Rc::clone(events), window, instance.trigger_node),
                    element,
                )
            } else {
                element.id(instance.state.popup_id())
            };
            let popup = instance.menu.root_part(instance.state.popup_id(), popup);
            if !listeners_enabled {
                return Some(popup);
            }
            Some(menu_popup_keyboard(popup, key, window, events, cx))
        }
        part if is_menu_row_part(part) => {
            let binding = *components.menu_compound.rows.get(&id)?;
            let instance = components.menu_compound.menus.get(&binding.menu)?;
            let row = instance.row_at(binding.index)?;
            let popup = menu_popup_id(ElementId::new(binding.menu));
            // A submenu trigger is both a row of its parent level and the trigger of its own, so
            // the core's submenu-trigger decorator owns it: it already carries `menuitem`,
            // `has-popup`, and the expanded state the row would otherwise declare twice.
            if row.kind == MenuRowKind::Submenu {
                let Some(child) = row.submenu.and_then(|child| {
                    components
                        .menu_compound
                        .menus
                        .get(&child)
                        .map(|instance| (child, instance))
                }) else {
                    return Some(element);
                };
                let (child_key, child_instance) = child;
                if !listeners_enabled {
                    return Some(element.id(child_instance.state.trigger_id()));
                }
                let decorated = child_instance.state.submenu_trigger_part_with(
                    cx,
                    menu_accessor(child_key),
                    open_change(Rc::clone(events), window, id),
                    element,
                );
                return Some(highlight_on_hover(
                    decorated,
                    binding,
                    child_instance.state.trigger_id(),
                    cx,
                ));
            }
            let decorated = instance.menu.item_part(popup, binding.index, element)?;
            if !listeners_enabled {
                return Some(decorated);
            }
            let Some(row_id) = instance.menu.item_element_id(popup, binding.index) else {
                return Some(decorated);
            };
            if !instance
                .menu
                .items()
                .get(binding.index)
                .is_some_and(PopoverMenuItem::is_interactive)
            {
                return Some(decorated);
            }
            let activate_events = Rc::clone(events);
            let click = cx.listener(row_id, move |view, cx| {
                activate_menu_row(view, binding, window, &activate_events, cx);
            });
            Some(highlight_on_hover(
                decorated.on_click(click),
                binding,
                row_id,
                cx,
            ))
        }
        _ => Some(element),
    }
}

/// Report one level's controlled open value back to JavaScript.
fn open_change(
    events: EventQueue,
    window: u32,
    target: u32,
) -> impl Fn(&mut NativeView, bool, &mut EventContext) + Clone + 'static {
    move |view: &mut NativeView, open: bool, _cx: &mut EventContext| {
        let listens = view
            .tree
            .borrow()
            .nodes
            .get(&target)
            .is_some_and(declares_change);
        if !listens {
            return;
        }
        enqueue_component_change(&events, window, target, serde_json::json!({ "open": open }));
    }
}

/// Follow one row's hover state without clearing a highlight the keyboard has since moved.
fn highlight_on_hover(
    element: Element,
    binding: MenuRowBinding,
    row_id: ElementId,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    let hover = cx.hover_listener(row_id, move |view: &mut NativeView, hovered, cx| {
        let Some(instance) = view.components.menu_compound.menus.get_mut(&binding.menu) else {
            return;
        };
        let changed = if *hovered {
            instance.menu.highlight(binding.index)
        } else {
            instance.menu.unhighlight(binding.index)
        };
        if changed {
            cx.invalidate();
        }
    });
    element.on_hover(hover)
}

/// Activate one row through the retained model and publish everything the core decided.
fn activate_menu_row(
    view: &mut NativeView,
    binding: MenuRowBinding,
    window: u32,
    events: &EventQueue,
    cx: &mut EventContext,
) {
    let (row, close, checked, link) = {
        let Some(instance) = view.components.menu_compound.menus.get_mut(&binding.menu) else {
            return;
        };
        let Some(row) = instance.rows.get(binding.index).cloned() else {
            return;
        };
        let Some(activation) = instance.menu.activate(binding.index) else {
            return;
        };
        let close = match &activation {
            PopoverMenuActivation::Command { close_menu, .. } => *close_menu,
            PopoverMenuActivation::Submenu { .. } => false,
        };
        let checked = instance
            .menu
            .item_part_state(binding.index, false)
            .and_then(|state| state.checked);
        let link = match &activation {
            PopoverMenuActivation::Command { action, .. } => action
                .downcast_ref::<quickgui::OpenMenuLink>()
                .map(|link| Arc::clone(&link.url)),
            PopoverMenuActivation::Submenu { .. } => None,
        };
        (row, close, checked, link)
    };
    if close {
        let chain = view
            .components
            .menu_compound
            .menu_chain_to_root(binding.menu);
        for key in chain {
            let Some(instance) = view.components.menu_compound.menus.get_mut(&key) else {
                continue;
            };
            if instance.state.close_now() && instance.listens {
                enqueue_component_change(
                    events,
                    window,
                    instance.trigger_node,
                    serde_json::json!({ "open": false }),
                );
            }
        }
    }
    if row.clicks {
        enqueue_event(
            events,
            QueuedEvent {
                kind: "click",
                window,
                target: row.node,
                value: None,
            },
        );
    }
    if row.listens {
        let mut payload = serde_json::Map::new();
        if let Some(checked) = checked {
            payload.insert("checked".to_owned(), serde_json::Value::Bool(checked));
        }
        if let Some(url) = link.as_deref() {
            payload.insert("href".to_owned(), serde_json::Value::String(url.to_owned()));
        }
        payload.insert(
            "activated".to_owned(),
            serde_json::Value::String(row.id.to_string()),
        );
        enqueue_component_change(events, window, row.node, serde_json::Value::Object(payload));
    }
    // A radio row's new value belongs to its group, exactly as Base UI reports `onValueChange`.
    if row.kind == MenuRowKind::Radio
        && let Some(group_node) = row.group_node
    {
        let listens = view
            .tree
            .borrow()
            .nodes
            .get(&group_node)
            .is_some_and(declares_change);
        if listens {
            enqueue_component_change(
                events,
                window,
                group_node,
                serde_json::json!({ "value": row.id.as_ref() }),
            );
        }
    }
    // QuickGUI has no document to navigate, so a link item's destination reaches the platform
    // through the core's own open-URL path and is reported alongside it.
    if let Some(url) = link {
        let _ = cx.open_url(url);
    }
    cx.invalidate();
}

/// Attach the core's own contextual navigation, typeahead, activation, and dismissal to a popup.
fn menu_popup_keyboard(
    popup: Element,
    key: u64,
    window: u32,
    events: &EventQueue,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    let popup_id = menu_popup_id(ElementId::new(key));
    let mut popup = popup;
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuPrevious, cx| {
            update_menu(view, key, cx, PopoverMenu::select_previous);
        },
    ));
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuNext, cx| {
            update_menu(view, key, cx, PopoverMenu::select_next);
        },
    ));
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuFirst, cx| {
            update_menu(view, key, cx, PopoverMenu::select_first);
        },
    ));
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuLast, cx| {
            update_menu(view, key, cx, PopoverMenu::select_last);
        },
    ));
    let activate_events = Rc::clone(events);
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuActivate, cx| {
            let Some(index) = view
                .components
                .menu_compound
                .menus
                .get(&key)
                .and_then(|instance| instance.menu.active_index())
            else {
                return;
            };
            activate_menu_row(
                view,
                MenuRowBinding { menu: key, index },
                window,
                &activate_events,
                cx,
            );
        },
    ));
    let submenu_events = Rc::clone(events);
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuOpenSubmenu, cx| {
            open_highlighted_submenu(view, key, window, &submenu_events, cx);
        },
    ));
    let close_events = Rc::clone(events);
    popup = popup.on_action(cx.action_listener(
        popup_id,
        move |view: &mut NativeView, _: &quickgui::PopoverMenuClose, cx| {
            close_menu_level(view, key, window, &close_events, cx);
        },
    ));
    popup.on_key_down(
        cx.key_down_listener(popup_id, move |view: &mut NativeView, event, cx| {
            if event.modifiers.intersects(
                quickgui::Modifiers::CONTROL
                    | quickgui::Modifiers::ALT
                    | quickgui::Modifiers::SUPER,
            ) {
                return;
            }
            let pressed = event.key_char.as_ref().unwrap_or(&event.key);
            let quickgui::Key::Character(value) = pressed else {
                return;
            };
            if value.trim().is_empty() {
                return;
            }
            if let Some(instance) = view.components.menu_compound.menus.get_mut(&key)
                && instance.menu.typeahead(value, std::time::Instant::now())
            {
                cx.invalidate();
            }
            cx.prevent_default();
            cx.stop_propagation();
        }),
    )
}

fn update_menu(
    view: &mut NativeView,
    key: u64,
    cx: &mut EventContext,
    apply: fn(&mut PopoverMenu) -> bool,
) {
    if let Some(instance) = view.components.menu_compound.menus.get_mut(&key)
        && apply(&mut instance.menu)
    {
        cx.invalidate();
    }
}

/// Close the focused level and return focus to its trigger on the parent menu's focus path.
fn close_menu_level(
    view: &mut NativeView,
    key: u64,
    window: u32,
    events: &EventQueue,
    cx: &mut EventContext,
) {
    let Some(instance) = view.components.menu_compound.menus.get_mut(&key) else {
        return;
    };
    let trigger = instance.state.trigger_id();
    if !instance.state.close_now() {
        return;
    }
    if instance.listens {
        enqueue_component_change(
            events,
            window,
            instance.trigger_node,
            serde_json::json!({ "open": false }),
        );
    }
    cx.focus(quickgui::FocusHandle::new(trigger));
    cx.invalidate();
}

/// Open the nested level the highlighted row declares, Base UI's Right-arrow behavior.
fn open_highlighted_submenu(
    view: &mut NativeView,
    key: u64,
    window: u32,
    events: &EventQueue,
    cx: &mut EventContext,
) {
    let Some((child, node)) = view
        .components
        .menu_compound
        .menus
        .get(&key)
        .and_then(|instance| {
            let index = instance.menu.active_index()?;
            let row = instance.rows.get(index)?;
            Some((row.submenu?, row.node))
        })
    else {
        return;
    };
    let Some(instance) = view.components.menu_compound.menus.get_mut(&child) else {
        return;
    };
    let changed = instance.state.open_now();
    let popup = instance.state.popup_id();
    let listens = view
        .tree
        .borrow()
        .nodes
        .get(&node)
        .is_some_and(declares_change);
    if changed && listens {
        enqueue_component_change(events, window, node, serde_json::json!({ "open": true }));
    }
    cx.focus(quickgui::FocusHandle::new(popup));
    cx.invalidate();
}
