//! Declared popover and context menus bound to the Rust core's `PopoverMenu` model.
//!
//! A menu is declared ahead of time as one bounded JSON property so the hosted JavaScript boundary
//! never has to answer a synchronous question while the core is deciding what a secondary click,
//! an arrow key, or a typeahead prefix means. The core owns validation, highlighting, typeahead,
//! toggle policy, submenu models, accessibility semantics, and native surface lifetime; this module
//! only translates the declaration and turns the core's typed activation into an asynchronous
//! event.

use super::*;
use serde::Deserialize;

/// Declared part name of an in-window popover-menu surface.
pub(super) const POPOVER_MENU_POPUP_PART: &str = "popover-menu-popup";
/// Declared part name of a popover-menu trigger.
pub(super) const POPOVER_MENU_TRIGGER_PART: &str = "popover-menu-trigger";
/// Declared part name of a cursor-point context-menu target.
pub(super) const CONTEXT_MENU_TRIGGER_PART: &str = "context-menu-trigger";

/// Longest bounded menu declaration accepted from one `menu` property.
pub(super) const MAX_MENU_JSON_BYTES: usize = 512 * 1024;
/// Longest bounded stable item identifier accepted from one declared menu entry.
pub(super) const MAX_MENU_ITEM_ID_BYTES: usize = MAX_COMPONENT_VALUE_BYTES;

const DEFAULT_MENU_WIDTH: f32 = 224.0;
const DEFAULT_MENU_ITEM_HEIGHT: f32 = 28.0;
const DEFAULT_MENU_SEPARATOR_HEIGHT: f32 = 9.0;
const DEFAULT_MENU_GROUP_LABEL_HEIGHT: f32 = 22.0;
const DEFAULT_MENU_VERTICAL_PADDING: f32 = 4.0;
const DEFAULT_MENU_FONT_SIZE: f32 = 13.0;
const DEFAULT_MENU_RADIUS: f32 = 6.0;
const DEFAULT_MENU_PADDING: f32 = 10.0;
const DISABLED_ROW_OPACITY: f32 = 0.4;

/// Typed command the core dispatches when a declared menu entry is activated.
///
/// The payload keeps the application's own stable item identifier, so JavaScript receives the id
/// it declared instead of a positional index that atomic source replacement could invalidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NativeMenuSelect {
    pub(super) node: u32,
    pub(super) item: Arc<str>,
    pub(super) checked: Option<bool>,
}

impl NativeMenuSelect {
    pub(super) fn event_value(&self) -> Arc<str> {
        menu_select_value(&self.item, self.checked, false)
    }
}

fn menu_select_value(item: &str, checked: Option<bool>, submenu: bool) -> Arc<str> {
    let mut value = serde_json::Map::new();
    value.insert("id".to_owned(), serde_json::Value::String(item.to_owned()));
    if let Some(checked) = checked {
        value.insert("checked".to_owned(), serde_json::Value::Bool(checked));
    }
    if submenu {
        value.insert("submenu".to_owned(), serde_json::Value::Bool(true));
    }
    Arc::from(serde_json::Value::Object(value).to_string().as_str())
}

/// Structural geometry and appearance declared alongside the items.
///
/// The core renders the cursor-point context surface in its own window, so these measurements
/// travel with the model instead of being read back from JavaScript while a native menu is
/// deciding its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeMenuStyle {
    pub(super) width: f32,
    pub(super) item_height: f32,
    pub(super) separator_height: f32,
    pub(super) group_label_height: f32,
    pub(super) vertical_padding: f32,
    pub(super) font_size: f32,
    pub(super) radius: f32,
    pub(super) padding: f32,
    pub(super) background: Option<Color>,
    pub(super) color: Option<Color>,
    pub(super) highlight_background: Option<Color>,
    pub(super) highlight_color: Option<Color>,
    pub(super) muted_color: Option<Color>,
}

impl Default for NativeMenuStyle {
    fn default() -> Self {
        Self {
            width: DEFAULT_MENU_WIDTH,
            item_height: DEFAULT_MENU_ITEM_HEIGHT,
            separator_height: DEFAULT_MENU_SEPARATOR_HEIGHT,
            group_label_height: DEFAULT_MENU_GROUP_LABEL_HEIGHT,
            vertical_padding: DEFAULT_MENU_VERTICAL_PADDING,
            font_size: DEFAULT_MENU_FONT_SIZE,
            radius: DEFAULT_MENU_RADIUS,
            padding: DEFAULT_MENU_PADDING,
            background: None,
            color: None,
            highlight_background: None,
            highlight_color: None,
            muted_color: None,
        }
    }
}

impl NativeMenuStyle {
    /// Project the declaration onto the core's structural context-menu measurements.
    pub(super) fn layout(self) -> ContextMenuLayout {
        ContextMenuLayout::new(self.width, self.item_height)
            .separator_height(self.separator_height)
            .group_label_height(self.group_label_height)
            .vertical_padding(self.vertical_padding)
    }

    fn row_height(self, kind: PopoverMenuItemKind) -> f32 {
        self.layout().row_height(kind)
    }
}

/// One declared menu entry. Absent fields fall back to the core's own defaults.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeMenuEntry {
    #[serde(rename = "type")]
    kind: Option<String>,
    id: Option<String>,
    label: Option<String>,
    shortcut: Option<String>,
    group: Option<String>,
    checked: Option<bool>,
    disabled: Option<bool>,
    close_on_select: Option<bool>,
    typeahead_label: Option<String>,
    #[serde(default)]
    items: Vec<NativeMenuEntry>,
}

/// One complete bounded menu declaration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeMenuDeclaration {
    width: Option<f32>,
    item_height: Option<f32>,
    separator_height: Option<f32>,
    group_label_height: Option<f32>,
    vertical_padding: Option<f32>,
    font_size: Option<f32>,
    radius: Option<f32>,
    padding: Option<f32>,
    background: Option<u32>,
    color: Option<u32>,
    highlight_background: Option<u32>,
    highlight_color: Option<u32>,
    muted_color: Option<u32>,
    loop_focus: Option<bool>,
    #[serde(default)]
    items: Vec<NativeMenuEntry>,
}

impl NativeMenuDeclaration {
    /// Decode a bounded declaration. Malformed or oversized JSON declares no menu at all.
    pub(super) fn parse(value: &str) -> Option<Self> {
        if value.len() > MAX_MENU_JSON_BYTES {
            return None;
        }
        serde_json::from_str::<Self>(value).ok()
    }

    pub(super) fn style(&self) -> NativeMenuStyle {
        let default = NativeMenuStyle::default();
        NativeMenuStyle {
            width: positive(self.width, default.width),
            item_height: positive(self.item_height, default.item_height),
            separator_height: positive(self.separator_height, default.separator_height),
            group_label_height: positive(self.group_label_height, default.group_label_height),
            vertical_padding: non_negative(self.vertical_padding, default.vertical_padding),
            font_size: positive(self.font_size, default.font_size),
            radius: non_negative(self.radius, default.radius),
            padding: non_negative(self.padding, default.padding),
            background: self.background.map(unpack_color),
            color: self.color.map(unpack_color),
            highlight_background: self.highlight_background.map(unpack_color),
            highlight_color: self.highlight_color.map(unpack_color),
            muted_color: self.muted_color.map(unpack_color),
        }
    }

    pub(super) fn entries(&self) -> &[NativeMenuEntry] {
        &self.items
    }

    pub(super) fn loops_focus(&self) -> bool {
        self.loop_focus.unwrap_or(true)
    }
}

fn positive(value: Option<f32>, fallback: f32) -> f32 {
    match value {
        Some(value) if value.is_finite() && value > 0.0 => value,
        _ => fallback,
    }
}

fn non_negative(value: Option<f32>, fallback: f32) -> f32 {
    match value {
        Some(value) if value.is_finite() && value >= 0.0 => value,
        _ => fallback,
    }
}

fn bounded_id(value: &str) -> Option<&str> {
    (!value.is_empty() && value.len() <= MAX_MENU_ITEM_ID_BYTES).then_some(value)
}

fn bounded_text(value: &str) -> Option<&str> {
    (value.len() <= quickgui::MAX_POPOVER_MENU_ITEM_TEXT_BYTES).then_some(value)
}

/// Translate declared entries into validated core items paired with their declared identifiers.
///
/// Entries the core could not form are skipped, so the declaration of one malformed row never
/// discards the whole menu.
pub(super) fn menu_entries(
    node: u32,
    entries: &[NativeMenuEntry],
    depth: usize,
) -> Vec<(Option<Arc<str>>, PopoverMenuItem)> {
    if depth >= quickgui::MAX_POPOVER_MENU_DEPTH {
        return Vec::new();
    }
    let mut items = Vec::new();
    for entry in entries {
        if items.len() >= quickgui::MAX_POPOVER_MENU_ITEMS {
            break;
        }
        let kind = entry.kind.as_deref().unwrap_or(if entry.items.is_empty() {
            "action"
        } else {
            "submenu"
        });
        let built = match kind {
            "separator" | "divider" => Some((None, PopoverMenuItem::separator())),
            "group" | "group-label" | "header" => entry
                .label
                .as_deref()
                .and_then(bounded_text)
                .map(|label| (None, PopoverMenuItem::group_label(label))),
            _ => interactive_item(node, entry, kind, depth),
        };
        let Some((id, mut item)) = built else {
            continue;
        };
        if let Some(disabled) = entry.disabled {
            item = item.disabled(disabled);
        }
        if let Some(shortcut) = entry.shortcut.as_deref().and_then(bounded_text) {
            item = item.shortcut(shortcut);
        }
        if let Some(label) = entry.typeahead_label.as_deref().and_then(bounded_text) {
            item = item.typeahead_label(label);
        }
        if let Some(close) = entry.close_on_select {
            item = item.close_on_activate(close);
        }
        items.push((id, item));
    }
    items
}

/// Translate declared entries into validated core items.
pub(super) fn menu_items(
    node: u32,
    entries: &[NativeMenuEntry],
    depth: usize,
) -> Vec<PopoverMenuItem> {
    menu_entries(node, entries, depth)
        .into_iter()
        .map(|(_, item)| item)
        .collect()
}

fn interactive_item(
    node: u32,
    entry: &NativeMenuEntry,
    kind: &str,
    depth: usize,
) -> Option<(Option<Arc<str>>, PopoverMenuItem)> {
    let id = entry.id.as_deref().and_then(bounded_id)?;
    let label = entry.label.as_deref().and_then(bounded_text)?;
    let item: Arc<str> = Arc::from(id);
    let built = match kind {
        "checkbox" | "check" => {
            let toggled = Arc::clone(&item);
            PopoverMenuItem::checkbox_with(
                ElementId::named(id),
                label,
                entry.checked.unwrap_or(false),
                move |checked| NativeMenuSelect {
                    node,
                    item: Arc::clone(&toggled),
                    checked: Some(checked),
                },
            )
        }
        "radio" => PopoverMenuItem::radio(
            ElementId::named(id),
            label,
            ElementId::named(entry.group.as_deref().and_then(bounded_id).unwrap_or(id)),
            entry.checked.unwrap_or(false),
            NativeMenuSelect {
                node,
                item: Arc::clone(&item),
                checked: Some(true),
            },
        ),
        "submenu" => {
            let submenu = PopoverMenu::new(menu_items(node, &entry.items, depth + 1)).ok()?;
            PopoverMenuItem::submenu(ElementId::named(id), label, submenu)
        }
        _ => PopoverMenuItem::action(
            ElementId::named(id),
            label,
            NativeMenuSelect {
                node,
                item: Arc::clone(&item),
                checked: None,
            },
        ),
    };
    Some((Some(item), built))
}

/// Render one appearance-only row from the declaration.
///
/// The core wraps whatever this returns with the exact `menuitem`, checkable, submenu, separator,
/// or group-label semantics and identity, so this function contributes no behavior.
pub(super) fn menu_row(
    style: NativeMenuStyle,
    item: &PopoverMenuItem,
    state: PopoverMenuItemState,
) -> Element {
    match item.kind() {
        PopoverMenuItemKind::Separator => {
            div()
                .w_full()
                .flex_col()
                .justify_center()
                .child(match style.muted_color {
                    Some(color) => div().w_full().h(1.0).bg(color),
                    None => div().w_full().h(1.0).opacity(0.2),
                })
        }
        PopoverMenuItemKind::GroupLabel => {
            let mut row = div()
                .w_full()
                .flex_row()
                .items_center()
                .px(style.padding)
                .child(text(item.label().clone()).text_size(style.font_size * 0.85));
            if let Some(color) = style.muted_color.or(style.color) {
                row = row.text_color(color);
            }
            row
        }
        kind => {
            let mut row = div()
                .w_full()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(12.0)
                .px(style.padding)
                .rounded(style.radius.min(style.item_height / 2.0));
            if let Some(color) = style.color {
                row = row.text_color(color);
            }
            if state.highlighted && !state.disabled {
                if let Some(background) = style.highlight_background {
                    row = row.bg(background);
                }
                if let Some(color) = style.highlight_color {
                    row = row.text_color(color);
                }
            }
            if state.disabled {
                row = row.opacity(DISABLED_ROW_OPACITY);
            }
            let mark = match state.checked {
                Some(true) => "\u{2713} ",
                Some(false) => "  ",
                None => "",
            };
            row = row.child(
                text(format!("{mark}{}", item.label()))
                    .text_size(style.font_size)
                    .truncate(),
            );
            let trailing = match (kind, item.shortcut_text()) {
                (PopoverMenuItemKind::Submenu, _) => Some(Arc::<str>::from("\u{203a}")),
                (_, Some(shortcut)) => Some(Arc::clone(shortcut)),
                _ => None,
            };
            if let Some(trailing) = trailing {
                let mut hint = text(trailing)
                    .text_size(style.font_size)
                    .whitespace_nowrap();
                if let Some(color) = style.muted_color {
                    hint = hint.text_color(color);
                }
                row = row.child(hint);
            }
            row
        }
    }
}

/// Retained interaction state for one in-window popover-menu node.
pub(super) struct NativeMenuState {
    pub(super) source: Arc<str>,
    pub(super) menu: PopoverMenu,
    pub(super) style: NativeMenuStyle,
    /// Declared identifier per top-level row, aligned with the validated core items.
    pub(super) ids: Vec<Option<Arc<str>>>,
}

impl NativeMenuState {
    pub(super) fn new(node: u32, source: &str) -> Option<Self> {
        let declaration = NativeMenuDeclaration::parse(source)?;
        let entries = menu_entries(node, declaration.entries(), 0);
        let ids = entries.iter().map(|(id, _)| id.clone()).collect();
        let items = entries
            .into_iter()
            .map(|(_, item)| item)
            .collect::<Vec<_>>();
        Some(Self {
            source: Arc::from(source),
            menu: PopoverMenu::new(items)
                .ok()?
                .loop_focus(declaration.loops_focus()),
            style: declaration.style(),
            ids,
        })
    }

    /// Adopt a replaced declaration while the core preserves the highlighted stable identity.
    pub(super) fn sync(&mut self, node: u32, source: &str) {
        if &*self.source == source {
            return;
        }
        let Some(declaration) = NativeMenuDeclaration::parse(source) else {
            return;
        };
        let entries = menu_entries(node, declaration.entries(), 0);
        let ids = entries.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
        let items = entries
            .into_iter()
            .map(|(_, item)| item)
            .collect::<Vec<_>>();
        if self.menu.set_items(items).is_err() {
            return;
        }
        self.source = Arc::from(source);
        self.style = declaration.style();
        self.ids = ids;
    }

    fn declared_id(&self, index: usize) -> Option<Arc<str>> {
        self.ids.get(index).cloned().flatten()
    }
}

pub(super) type NativeMenuStates = Rc<RefCell<HashMap<u32, NativeMenuState>>>;

/// The single per-window context-menu state the core mutates through a plain accessor.
///
/// `ContextMenuState::element` takes a non-capturing accessor, so one window owns exactly one
/// context-menu state. That matches the native invariant: opening a context menu anywhere replaces
/// the menu already open and tears its root surface down first.
pub(super) fn native_context_menu_state(view: &mut NativeView) -> &mut ContextMenuState {
    &mut view.context_menu
}

/// Attach the core's cursor-point context menu to a caller-owned target.
pub(super) fn apply_context_menu(
    element: Element,
    element_id: ElementId,
    id: u32,
    node: &NativeNode,
    state: ContextMenuState,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    let Some(declaration) = node
        .string(property::MENU)
        .and_then(NativeMenuDeclaration::parse)
    else {
        return state.target_part(element_id, element);
    };
    let style = declaration.style();
    let entries: Rc<[NativeMenuEntry]> = Rc::from(declaration.entries().to_vec());
    let loops = declaration.loops_focus();
    state.element(
        cx,
        element_id,
        native_context_menu_state,
        element,
        style.layout(),
        move |view, _event| {
            view.context_menu_owner = Some(id);
            Some(
                PopoverMenu::new(menu_items(id, &entries, 0))
                    .ok()?
                    .loop_focus(loops),
            )
        },
        move || {
            let mut root = div().flex_col().py(style.vertical_padding);
            if let Some(background) = style.background {
                root = root.bg(background);
            }
            if let Some(color) = style.color {
                root = root.text_color(color);
            }
            root.rounded(style.radius)
        },
        move |item, item_state| menu_row(style, item, item_state),
    )
}

/// Decorate one in-window popover-menu surface and mount its declared rows.
///
/// Menu behavior comes from the core's own contextual key bindings, model mutators, and part
/// decorators; this function only routes them and turns an activation into an asynchronous event.
#[allow(clippy::too_many_arguments)]
pub(super) fn popover_menu_surface(
    element: Element,
    element_id: ElementId,
    id: u32,
    window: u32,
    events: &EventQueue,
    menus: &NativeMenuStates,
    node: &NativeNode,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    let Some((style, rows)) = ({
        let states = menus.borrow();
        states.get(&id).map(|state| {
            let rows = (0..state.menu.items().len())
                .filter_map(|index| {
                    let item = state.menu.items().get(index)?;
                    let item_state = state.menu.item_state(index)?;
                    let row = menu_row(state.style, item, item_state)
                        .h(state.style.row_height(item.kind()))
                        .flex_none();
                    let row = state.menu.item_part(element_id, index, row)?;
                    let interactive = item.is_interactive() && !item_state.disabled;
                    let row_id = interactive
                        .then(|| state.menu.item_element_id(element_id, index))
                        .flatten();
                    Some((index, row_id, row))
                })
                .collect::<Vec<_>>();
            (state.style, rows)
        })
    }) else {
        return element;
    };

    let selectable = node.boolean(property::SELECT_LISTENER).unwrap_or(false);
    let mut children = Vec::with_capacity(rows.len());
    for (index, row_id, row) in rows {
        let mut row = row;
        if let Some(row_id) = row_id {
            let activate_menus = Rc::clone(menus);
            let activate_events = Rc::clone(events);
            let click = cx.listener(row_id, move |_view, cx| {
                activate_row(
                    id,
                    window,
                    index,
                    selectable,
                    &activate_menus,
                    &activate_events,
                    cx,
                );
            });
            let hover_menus = Rc::clone(menus);
            let hover = cx.hover_listener(row_id, move |_view, hovered, cx| {
                if *hovered
                    && let Some(state) = hover_menus.borrow_mut().get_mut(&id)
                    && state.menu.highlight(index)
                {
                    cx.invalidate();
                }
            });
            row = row.on_click(click).on_hover(hover);
        }
        children.push(row);
    }

    let root = {
        let states = menus.borrow();
        match states.get(&id) {
            Some(state) => state.menu.root_part(element_id, element),
            None => return element,
        }
    };
    let mut element = root
        .flex_col()
        .py(style.vertical_padding)
        .children(children);

    element = element.on_action(cx.action_listener(element_id, {
        let menus = Rc::clone(menus);
        move |_view, _: &quickgui::PopoverMenuPrevious, cx| {
            update_menu(&menus, id, cx, PopoverMenu::select_previous);
        }
    }));
    element = element.on_action(cx.action_listener(element_id, {
        let menus = Rc::clone(menus);
        move |_view, _: &quickgui::PopoverMenuNext, cx| {
            update_menu(&menus, id, cx, PopoverMenu::select_next);
        }
    }));
    element = element.on_action(cx.action_listener(element_id, {
        let menus = Rc::clone(menus);
        move |_view, _: &quickgui::PopoverMenuFirst, cx| {
            update_menu(&menus, id, cx, PopoverMenu::select_first);
        }
    }));
    element = element.on_action(cx.action_listener(element_id, {
        let menus = Rc::clone(menus);
        move |_view, _: &quickgui::PopoverMenuLast, cx| {
            update_menu(&menus, id, cx, PopoverMenu::select_last);
        }
    }));
    element = element.on_action(cx.action_listener(element_id, {
        let menus = Rc::clone(menus);
        let events = Rc::clone(events);
        move |_view, _: &quickgui::PopoverMenuActivate, cx| {
            let Some(index) = menus
                .borrow()
                .get(&id)
                .and_then(|state| state.menu.active_index())
            else {
                return;
            };
            activate_row(id, window, index, selectable, &menus, &events, cx);
        }
    }));
    element = element.on_action(cx.action_listener(element_id, {
        let events = Rc::clone(events);
        // Escape and Left both close one menu level. JavaScript owns the controlled `open` value,
        // so the core's decision travels back as the ordinary asynchronous dismissal event.
        let dismissible = node.boolean(property::DISMISS_LISTENER).unwrap_or(false);
        move |_view, _: &quickgui::PopoverMenuClose, cx| {
            if !dismissible {
                return;
            }
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "dismiss",
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        }
    }));
    element = element.on_key_down(cx.key_down_listener(element_id, {
        let menus = Rc::clone(menus);
        move |_view, event, cx| {
            if event.modifiers.intersects(
                quickgui::Modifiers::CONTROL
                    | quickgui::Modifiers::ALT
                    | quickgui::Modifiers::SUPER,
            ) {
                return;
            }
            let key = event.key_char.as_ref().unwrap_or(&event.key);
            let quickgui::Key::Character(value) = key else {
                return;
            };
            if value.trim().is_empty() {
                return;
            }
            if let Some(state) = menus.borrow_mut().get_mut(&id)
                && state.menu.typeahead(value, std::time::Instant::now())
            {
                cx.invalidate();
            }
            cx.prevent_default();
            cx.stop_propagation();
        }
    }));
    element
}

fn update_menu(
    menus: &NativeMenuStates,
    id: u32,
    cx: &mut EventContext,
    apply: fn(&mut PopoverMenu) -> bool,
) {
    if let Some(state) = menus.borrow_mut().get_mut(&id)
        && apply(&mut state.menu)
    {
        cx.invalidate();
    }
}

/// Activate one row through the core model and publish the resulting typed command.
fn activate_row(
    id: u32,
    window: u32,
    index: usize,
    selectable: bool,
    menus: &NativeMenuStates,
    events: &EventQueue,
    cx: &mut EventContext,
) {
    let value = {
        let mut states = menus.borrow_mut();
        let Some(state) = states.get_mut(&id) else {
            return;
        };
        match state.menu.activate(index) {
            Some(PopoverMenuActivation::Command { action, .. }) => action
                .downcast_ref::<NativeMenuSelect>()
                .map(NativeMenuSelect::event_value),
            Some(PopoverMenuActivation::Submenu { item_index, .. }) => state
                .declared_id(item_index)
                .map(|item| menu_select_value(&item, None, true)),
            None => return,
        }
    };
    if let Some(value) = value.filter(|_| selectable) {
        enqueue_event(
            events,
            QueuedEvent {
                kind: "menuselect",
                window,
                target: id,
                value: Some(value),
            },
        );
    }
    cx.invalidate();
}
