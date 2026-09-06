//! Declared select, constrained combobox, and free-form autocomplete bound to the Rust core.
//!
//! These three components own a native popover window of their own. The rows in that window are
//! rendered by the core from a bounded `appearance` declaration exactly the way
//! [`super::popover_menu`] renders declared menu rows: the row renderer the core retains is a
//! `'static` closure, so it can never reach back into the hosted JavaScript tree while the core is
//! deciding what a keystroke means.
//!
//! Everything else is a declaration too. JavaScript declares the option source, the controlled
//! value, the controlled input text, and the filter mode ahead of time; the core owns filtering,
//! highlight movement, typeahead, native surface placement and lifetime, accessibility semantics,
//! and commit policy; and every result the core decides travels back as one asynchronous
//! `componentchange` or `commit` event.

use super::*;
use serde::Deserialize;

/// Declared part name of a select trigger.
pub(super) const SELECT_PART: &str = "select";
/// Declared part name of a constrained combobox input.
pub(super) const COMBOBOX_PART: &str = "combobox";
/// Declared part name of a free-form autocomplete input.
pub(super) const AUTOCOMPLETE_PART: &str = "autocomplete";
/// Declared part name of one child option node.
pub(super) const OPTION_PART: &str = "option";

/// Declared part name of a select's visible label.
pub(super) const SELECT_LABEL_PART: &str = "select-label";
/// Declared part name of a select's value text.
pub(super) const SELECT_VALUE_PART: &str = "select-value";
/// Declared part name of a select's trigger affordance.
pub(super) const SELECT_ICON_PART: &str = "select-icon";
/// Declared part name of a select's owner-window backdrop.
pub(super) const SELECT_BACKDROP_PART: &str = "select-backdrop";
/// Declared part name of a select's portal boundary.
pub(super) const SELECT_PORTAL_PART: &str = "select-portal";
/// Declared part name of a select's positioner.
pub(super) const SELECT_POSITIONER_PART: &str = "select-positioner";
/// Declared part name of a select's option surface.
pub(super) const SELECT_POPUP_PART: &str = "select-popup";
/// Declared part name of a select's decorative arrow.
pub(super) const SELECT_ARROW_PART: &str = "select-arrow";
/// Declared part name of a select's option list.
pub(super) const SELECT_LIST_PART: &str = "select-list";
/// Declared part name of one declared option row.
pub(super) const SELECT_ITEM_PART: &str = "select-item";
/// Declared part name of one option row's text.
pub(super) const SELECT_ITEM_TEXT_PART: &str = "select-item-text";
/// Declared part name of one option row's selected mark.
pub(super) const SELECT_ITEM_INDICATOR_PART: &str = "select-item-indicator";
/// Declared part name of an option group.
pub(super) const SELECT_GROUP_PART: &str = "select-group";
/// Declared part name of an option group's label.
pub(super) const SELECT_GROUP_LABEL_PART: &str = "select-group-label";
/// Declared part name of a divider between option groups.
pub(super) const SELECT_SEPARATOR_PART: &str = "select-separator";
/// Declared part name of a select's upward scroll affordance.
pub(super) const SELECT_SCROLL_UP_ARROW_PART: &str = "select-scroll-up-arrow";
/// Declared part name of a select's downward scroll affordance.
pub(super) const SELECT_SCROLL_DOWN_ARROW_PART: &str = "select-scroll-down-arrow";

/// Declared part name of a combobox's visible label.
pub(super) const COMBOBOX_LABEL_PART: &str = "combobox-label";
/// Declared part name of a combobox's committed-value text.
pub(super) const COMBOBOX_VALUE_PART: &str = "combobox-value";
/// Declared part name of a combobox's affordance glyph.
pub(super) const COMBOBOX_ICON_PART: &str = "combobox-icon";
/// Declared part name of a combobox's input group.
pub(super) const COMBOBOX_INPUT_GROUP_PART: &str = "combobox-input-group";
/// Declared part name of a combobox's clear control.
pub(super) const COMBOBOX_CLEAR_PART: &str = "combobox-clear";
/// Declared part name of a combobox's surface trigger.
pub(super) const COMBOBOX_TRIGGER_PART: &str = "combobox-trigger";
/// Declared part name of a combobox's chip container.
pub(super) const COMBOBOX_CHIPS_PART: &str = "combobox-chips";
/// Declared part name of one combobox chip.
pub(super) const COMBOBOX_CHIP_PART: &str = "combobox-chip";
/// Declared part name of one combobox chip's remove control.
pub(super) const COMBOBOX_CHIP_REMOVE_PART: &str = "combobox-chip-remove";
/// Declared part name of a combobox's owner-window backdrop.
pub(super) const COMBOBOX_BACKDROP_PART: &str = "combobox-backdrop";
/// Declared part name of a combobox's portal boundary.
pub(super) const COMBOBOX_PORTAL_PART: &str = "combobox-portal";
/// Declared part name of a combobox's positioner.
pub(super) const COMBOBOX_POSITIONER_PART: &str = "combobox-positioner";
/// Declared part name of a combobox's suggestion surface.
pub(super) const COMBOBOX_POPUP_PART: &str = "combobox-popup";
/// Declared part name of a combobox's decorative arrow.
pub(super) const COMBOBOX_ARROW_PART: &str = "combobox-arrow";
/// Declared part name of a combobox's polite live region.
pub(super) const COMBOBOX_STATUS_PART: &str = "combobox-status";
/// Declared part name of a combobox's no-results part.
pub(super) const COMBOBOX_EMPTY_PART: &str = "combobox-empty";
/// Declared part name of a combobox's result list.
pub(super) const COMBOBOX_LIST_PART: &str = "combobox-list";
/// Declared part name of a combobox's grid-shaped result row.
pub(super) const COMBOBOX_ROW_PART: &str = "combobox-row";
/// Declared part name of one declared combobox result.
pub(super) const COMBOBOX_ITEM_PART: &str = "combobox-item";
/// Declared part name of one result's selected mark.
pub(super) const COMBOBOX_ITEM_INDICATOR_PART: &str = "combobox-item-indicator";
/// Declared part name of a combobox result group.
pub(super) const COMBOBOX_GROUP_PART: &str = "combobox-group";
/// Declared part name of a combobox group's label.
pub(super) const COMBOBOX_GROUP_LABEL_PART: &str = "combobox-group-label";
/// Declared part name of a combobox's mounted-row wrapper.
pub(super) const COMBOBOX_COLLECTION_PART: &str = "combobox-collection";
/// Declared part name of a divider between combobox groups.
pub(super) const COMBOBOX_SEPARATOR_PART: &str = "combobox-separator";

/// Whether one declared part belongs to this module.
pub(super) fn owns_picker_part(part: &str) -> bool {
    matches!(
        part,
        SELECT_PART
            | COMBOBOX_PART
            | AUTOCOMPLETE_PART
            | OPTION_PART
            | SELECT_LABEL_PART
            | SELECT_VALUE_PART
            | SELECT_ICON_PART
            | SELECT_BACKDROP_PART
            | SELECT_PORTAL_PART
            | SELECT_POSITIONER_PART
            | SELECT_POPUP_PART
            | SELECT_ARROW_PART
            | SELECT_LIST_PART
            | SELECT_ITEM_PART
            | SELECT_ITEM_TEXT_PART
            | SELECT_ITEM_INDICATOR_PART
            | SELECT_GROUP_PART
            | SELECT_GROUP_LABEL_PART
            | SELECT_SEPARATOR_PART
            | SELECT_SCROLL_UP_ARROW_PART
            | SELECT_SCROLL_DOWN_ARROW_PART
            | COMBOBOX_LABEL_PART
            | COMBOBOX_VALUE_PART
            | COMBOBOX_ICON_PART
            | COMBOBOX_INPUT_GROUP_PART
            | COMBOBOX_CLEAR_PART
            | COMBOBOX_TRIGGER_PART
            | COMBOBOX_CHIPS_PART
            | COMBOBOX_CHIP_PART
            | COMBOBOX_CHIP_REMOVE_PART
            | COMBOBOX_BACKDROP_PART
            | COMBOBOX_PORTAL_PART
            | COMBOBOX_POSITIONER_PART
            | COMBOBOX_POPUP_PART
            | COMBOBOX_ARROW_PART
            | COMBOBOX_STATUS_PART
            | COMBOBOX_EMPTY_PART
            | COMBOBOX_LIST_PART
            | COMBOBOX_ROW_PART
            | COMBOBOX_ITEM_PART
            | COMBOBOX_ITEM_INDICATOR_PART
            | COMBOBOX_GROUP_PART
            | COMBOBOX_GROUP_LABEL_PART
            | COMBOBOX_COLLECTION_PART
            | COMBOBOX_SEPARATOR_PART
    )
}

/// Whether one declared part only contributes to the core-painted surface, mounting nothing.
///
/// QuickGUI paints a select's option list and a combobox's result list in a separate native
/// window from a bounded appearance declaration, so the popup-side parts are declarations rather
/// than owner-window elements: they name the surface the core builds and carry the option, group,
/// and scroll-arrow facts it needs.
fn picker_part_is_declaration(part: &str) -> bool {
    matches!(
        part,
        OPTION_PART
            | SELECT_PORTAL_PART
            | SELECT_POSITIONER_PART
            | SELECT_POPUP_PART
            | SELECT_ARROW_PART
            | SELECT_LIST_PART
            | SELECT_ITEM_PART
            | SELECT_ITEM_TEXT_PART
            | SELECT_ITEM_INDICATOR_PART
            | SELECT_GROUP_PART
            | SELECT_GROUP_LABEL_PART
            | SELECT_SEPARATOR_PART
            | SELECT_SCROLL_UP_ARROW_PART
            | SELECT_SCROLL_DOWN_ARROW_PART
            | COMBOBOX_PORTAL_PART
            | COMBOBOX_POSITIONER_PART
            | COMBOBOX_POPUP_PART
            | COMBOBOX_ARROW_PART
            | COMBOBOX_LIST_PART
            | COMBOBOX_ROW_PART
            | COMBOBOX_ITEM_PART
            | COMBOBOX_ITEM_INDICATOR_PART
            | COMBOBOX_GROUP_PART
            | COMBOBOX_GROUP_LABEL_PART
            | COMBOBOX_COLLECTION_PART
            | COMBOBOX_SEPARATOR_PART
    )
}

/// Whether one declared part is a child option declaration of its picker.
fn is_option_part(part: &str) -> bool {
    matches!(part, OPTION_PART | SELECT_ITEM_PART | COMBOBOX_ITEM_PART)
}

/// Whether one declared part groups child option declarations.
fn is_option_group_part(part: &str) -> bool {
    matches!(part, SELECT_GROUP_PART | COMBOBOX_GROUP_PART)
}

/// Longest bounded option declaration accepted from one `options` property.
pub(super) const MAX_OPTIONS_JSON_BYTES: usize = 512 * 1024;
/// Most declared options retained by one picker instance.
pub(super) const MAX_DECLARED_OPTIONS: usize = 4_096;

const DEFAULT_PICKER_WIDTH: f32 = 240.0;
const DEFAULT_PICKER_ROW_HEIGHT: f32 = 28.0;
const DEFAULT_PICKER_VISIBLE_ROWS: usize = 8;
const DEFAULT_PICKER_ANCHOR_GAP: f32 = 4.0;
const DEFAULT_PICKER_FONT_SIZE: f32 = 13.0;
const DEFAULT_PICKER_RADIUS: f32 = 6.0;
const DEFAULT_PICKER_PADDING: f32 = 10.0;
const DEFAULT_PICKER_VERTICAL_PADDING: f32 = 4.0;
const DISABLED_ROW_OPACITY: f32 = 0.4;

/// One declared option. Absent fields fall back to the core's own defaults.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeclaredOption {
    #[serde(default)]
    value: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    keywords: Option<String>,
    #[serde(default)]
    disabled: bool,
}

/// Structural geometry and paint for the rows the core renders in its own window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativePickerStyle {
    width: f32,
    row_height: f32,
    max_visible_rows: usize,
    anchor_gap: f32,
    font_size: f32,
    radius: f32,
    padding: f32,
    vertical_padding: f32,
    background: Option<Color>,
    color: Option<Color>,
    highlight_background: Option<Color>,
    highlight_color: Option<Color>,
    selected_background: Option<Color>,
    muted_color: Option<Color>,
}

impl Default for NativePickerStyle {
    fn default() -> Self {
        Self {
            width: DEFAULT_PICKER_WIDTH,
            row_height: DEFAULT_PICKER_ROW_HEIGHT,
            max_visible_rows: DEFAULT_PICKER_VISIBLE_ROWS,
            anchor_gap: DEFAULT_PICKER_ANCHOR_GAP,
            font_size: DEFAULT_PICKER_FONT_SIZE,
            radius: DEFAULT_PICKER_RADIUS,
            padding: DEFAULT_PICKER_PADDING,
            vertical_padding: DEFAULT_PICKER_VERTICAL_PADDING,
            background: None,
            color: None,
            highlight_background: None,
            highlight_color: None,
            selected_background: None,
            muted_color: None,
        }
    }
}

/// One bounded appearance declaration shared by all three pickers.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativePickerAppearance {
    width: Option<f32>,
    row_height: Option<f32>,
    max_visible_rows: Option<f32>,
    anchor_gap: Option<f32>,
    font_size: Option<f32>,
    radius: Option<f32>,
    padding: Option<f32>,
    vertical_padding: Option<f32>,
    background: Option<u32>,
    color: Option<u32>,
    highlight_background: Option<u32>,
    highlight_color: Option<u32>,
    selected_background: Option<u32>,
    muted_color: Option<u32>,
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

/// Decode the declared appearance. Malformed or oversized JSON falls back to the defaults.
fn picker_style(node: &NativeNode) -> NativePickerStyle {
    let default = NativePickerStyle::default();
    let Some(source) = node
        .string(property::APPEARANCE)
        .filter(|value| value.len() <= MAX_COMPONENT_JSON_BYTES)
    else {
        return default;
    };
    let Ok(declared) = serde_json::from_str::<NativePickerAppearance>(source) else {
        return default;
    };
    NativePickerStyle {
        width: positive(declared.width, default.width),
        row_height: positive(declared.row_height, default.row_height),
        max_visible_rows: declared
            .max_visible_rows
            .filter(|rows| rows.is_finite() && *rows >= 1.0)
            .map_or(default.max_visible_rows, |rows| {
                (rows as usize).min(quickgui::MAX_SELECT_VISIBLE_ROWS)
            }),
        anchor_gap: non_negative(declared.anchor_gap, default.anchor_gap),
        font_size: positive(declared.font_size, default.font_size),
        radius: non_negative(declared.radius, default.radius),
        padding: non_negative(declared.padding, default.padding),
        vertical_padding: non_negative(declared.vertical_padding, default.vertical_padding),
        background: declared.background.map(unpack_color),
        color: declared.color.map(unpack_color),
        highlight_background: declared.highlight_background.map(unpack_color),
        highlight_color: declared.highlight_color.map(unpack_color),
        selected_background: declared.selected_background.map(unpack_color),
        muted_color: declared.muted_color.map(unpack_color),
    }
}

impl NativePickerStyle {
    fn select_layout(self) -> SelectPopoverLayout {
        SelectPopoverLayout::new(self.width, self.row_height)
            .max_visible_rows(self.max_visible_rows)
            .anchor_gap(self.anchor_gap)
    }

    fn suggestion_layout(self) -> AutocompletePopoverLayout {
        AutocompletePopoverLayout::new(self.width, self.row_height)
            .max_visible_rows(self.max_visible_rows)
            .anchor_gap(self.anchor_gap)
    }

    /// The appearance-only popover container the core mounts its rows in.
    fn surface(self) -> Element {
        let mut root = div().flex_col().py(self.vertical_padding);
        if let Some(background) = self.background {
            root = root.bg(background);
        }
        if let Some(color) = self.color {
            root = root.text_color(color);
        }
        root.rounded(self.radius)
    }

    /// One appearance-only scroll affordance the core decorates and drives.
    fn scroll_arrow(self, glyph: &'static str) -> Element {
        let mut row = div()
            .w_full()
            .flex_none()
            .h(self.row_height * 0.6)
            .flex_row()
            .items_center()
            .justify_center();
        if let Some(color) = self.muted_color.or(self.color) {
            row = row.text_color(color);
        }
        row.child(text(glyph).text_size(self.font_size))
    }

    /// One appearance-only option row.
    ///
    /// The core wraps whatever this returns with the exact option semantics, identity, and
    /// active-descendant relationship, so this function contributes no behavior at all.
    fn row(
        self,
        item: &PickerItem<Arc<str>>,
        active: bool,
        selected: bool,
        disabled: bool,
    ) -> Element {
        let mut row = div()
            .w_full()
            .flex_none()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(12.0)
            .px(self.padding)
            .rounded(self.radius.min(self.row_height / 2.0));
        if let Some(color) = self.color {
            row = row.text_color(color);
        }
        if selected
            && !active
            && let Some(background) = self.selected_background
        {
            row = row.bg(background);
        }
        if active && !disabled {
            if let Some(background) = self.highlight_background {
                row = row.bg(background);
            }
            if let Some(color) = self.highlight_color {
                row = row.text_color(color);
            }
        }
        if disabled {
            row = row.opacity(DISABLED_ROW_OPACITY);
        }
        row = row.child(
            text(item.label().clone())
                .text_size(self.font_size)
                .truncate(),
        );
        if let Some(detail) = item.detail_text() {
            let mut hint = text(Arc::clone(detail))
                .text_size(self.font_size)
                .whitespace_nowrap();
            if let Some(color) = self.muted_color {
                hint = hint.text_color(color);
            }
            row = row.child(hint);
        }
        row
    }
}

/// Decode the declared option source into validated core picker items.
///
/// A declaration may carry the options as one bounded JSON array or as child `option` nodes. An
/// empty value, an oversized declaration, or a duplicate value is dropped rather than retained, so
/// the core never sees an item list it would reject.
fn declared_options(
    node: &NativeNode,
    key: u64,
    options: &HashMap<u64, Vec<DeclaredOption>>,
) -> Vec<PickerItem<Arc<str>>> {
    let declared = match node
        .string(property::OPTIONS)
        .filter(|value| value.len() <= MAX_OPTIONS_JSON_BYTES)
    {
        Some(source) => declared_option_source(source),
        None => options.get(&key).cloned().unwrap_or_default(),
    };
    let mut seen = HashSet::new();
    declared
        .into_iter()
        .filter(|option| {
            !option.value.is_empty() && option.value.len() <= MAX_COMPONENT_VALUE_BYTES
        })
        .filter(|option| seen.insert(option.value.clone()))
        .take(MAX_DECLARED_OPTIONS)
        .map(|option| {
            let label = option.label.unwrap_or_else(|| option.value.clone());
            let mut item = PickerItem::new(
                bounded_option_text(&label),
                Arc::<str>::from(option.value.as_str()),
            )
            .id(ElementId::named(option.value.as_str()))
            .disabled(option.disabled);
            if let Some(detail) = option.detail.as_deref().filter(|detail| !detail.is_empty()) {
                item = item.detail(bounded_option_text(detail));
            }
            // A declared group is searchable metadata, not a separate row: the core's picker model
            // has no group rows, so the group name joins the item's own search keywords.
            let keywords = match (option.keywords.as_deref(), option.group.as_deref()) {
                (Some(keywords), Some(group)) => Some(format!("{keywords} {group}")),
                (Some(keywords), None) => Some(keywords.to_owned()),
                (None, Some(group)) => Some(group.to_owned()),
                (None, None) => None,
            };
            if let Some(keywords) = keywords.as_deref().filter(|value| !value.is_empty()) {
                item = item.keywords(bounded_option_text(keywords));
            }
            item
        })
        .collect()
}

/// Decode a declared option source in either of Base UI's two shapes.
///
/// `items` is an array of option objects, or the map form -- one entry per value and its label,
/// which is exactly what [`SelectState::from_labels`] takes. A malformed declaration yields no
/// options rather than reaching a constructor that would reject the whole source.
fn declared_option_source(source: &str) -> Vec<DeclaredOption> {
    if let Ok(options) = serde_json::from_str::<Vec<DeclaredOption>>(source) {
        return options;
    }
    let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(source) else {
        return Vec::new();
    };
    map.into_iter()
        .map(|(value, label)| DeclaredOption {
            label: Some(match label {
                serde_json::Value::String(label) => label,
                other => other.to_string(),
            }),
            value,
            ..DeclaredOption::default()
        })
        .collect()
}

fn bounded_option_text(value: &str) -> Arc<str> {
    if value.len() <= quickgui::MAX_PICKER_ITEM_TEXT_BYTES {
        return Arc::from(value);
    }
    let mut end = quickgui::MAX_PICKER_ITEM_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&value[..end])
}

/// Collect the declared child `option` nodes of one picker, in declaration order.
pub(super) fn gather_picker_options(tree: &NativeTree) -> HashMap<u64, Vec<DeclaredOption>> {
    let mut options: HashMap<u64, Vec<DeclaredOption>> = HashMap::new();
    let Some(root) = tree.nodes.get(&ROOT_NODE) else {
        return options;
    };
    let mut total = 0usize;
    // One depth-first walk in declaration order, carrying the enclosing picker and the enclosing
    // `Select.Group` / `Combobox.Group` name down with it. A grouped option therefore declares its
    // group without repeating it on every row, an option nested inside its own picker needs no
    // scope at all, and a row's position never depends on hash iteration order.
    let mut pending = root
        .children
        .iter()
        .rev()
        .map(|id| (*id, None, None))
        .collect::<Vec<(u32, Option<String>, Option<u64>)>>();
    while let Some((id, group, owner)) = pending.pop() {
        if total >= MAX_DECLARED_OPTIONS * 4 {
            break;
        }
        let Some(child) = tree.nodes.get(&id) else {
            continue;
        };
        let part = child.string(property::PART).unwrap_or("");
        if is_option_part(part) {
            let key = declared_scope_key(child).or(owner).unwrap_or(u64::from(id));
            let bucket = options.entry(key).or_default();
            if bucket.len() >= MAX_DECLARED_OPTIONS {
                continue;
            }
            total += 1;
            bucket.push(DeclaredOption {
                value: child.string(property::PART_VALUE).unwrap_or("").to_owned(),
                label: child
                    .string(property::VALUE)
                    .map(str::to_owned)
                    .or_else(|| option_text_content(child, tree)),
                detail: child.string(property::VALUE_TEXT).map(str::to_owned),
                group: child
                    .string(property::GROUP)
                    .map(str::to_owned)
                    .or_else(|| group.clone()),
                keywords: None,
                disabled: child.boolean(property::DISABLED).unwrap_or(false),
            });
            continue;
        }
        let group = if is_option_group_part(part) {
            child
                .string(property::PART_VALUE)
                .or_else(|| child.string(property::GROUP))
                .map(str::to_owned)
                .or_else(|| group_label_text(child, tree))
                .or(group)
        } else {
            group
        };
        let owner = if matches!(part, SELECT_PART | COMBOBOX_PART | AUTOCOMPLETE_PART) {
            Some(component_key(id, child))
        } else {
            owner
        };
        pending.extend(
            child
                .children
                .iter()
                .rev()
                .map(|entry| (*entry, group.clone(), owner)),
        );
    }
    options
}

/// The scope key one node declared, or `None` when it named no scope at all.
fn declared_scope_key(node: &NativeNode) -> Option<u64> {
    node.string(property::SCOPE)
        .filter(|scope| !scope.is_empty() && scope.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(|scope| ElementId::named(scope).as_u64())
}

/// The visible text of one declared option, used when no explicit label was declared.
///
/// Base UI derives an item's label from its `Select.ItemText`, so the binding does the same: the
/// declared text is the label the core matches, ranks, and announces.
fn option_text_content(node: &NativeNode, tree: &NativeTree) -> Option<String> {
    let content = declared_text_content(node, tree);
    (!content.is_empty()).then_some(content)
}

/// The visible text of a declared group label, used as the group name when none was declared.
fn group_label_text(node: &NativeNode, tree: &NativeTree) -> Option<String> {
    let mut pending = node.children.iter().copied().rev().collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(id) = pending.pop() {
        visited += 1;
        if visited > MAX_DECLARED_OPTIONS {
            break;
        }
        let Some(child) = tree.nodes.get(&id) else {
            continue;
        };
        let part = child.string(property::PART).unwrap_or("");
        if matches!(part, SELECT_GROUP_LABEL_PART | COMBOBOX_GROUP_LABEL_PART) {
            let content = declared_text_content(child, tree);
            if !content.is_empty() {
                return Some(content);
            }
            continue;
        }
        if is_option_part(part) {
            continue;
        }
        pending.extend(child.children.iter().copied().rev());
    }
    None
}

/// The core offers exactly two filter modes: its own bounded fuzzy matcher, or none at all for a
/// source an application or service already filtered. An unknown declaration keeps the default.
fn declared_filter_mode(node: &NativeNode, default: PickerFilterMode) -> PickerFilterMode {
    match node.string(property::FILTER_MODE) {
        Some("none" | "manual") => PickerFilterMode::None,
        Some("contains") => PickerFilterMode::Contains,
        Some("startsWith" | "starts-with") => PickerFilterMode::StartsWith,
        Some("fuzzy") => PickerFilterMode::Fuzzy,
        _ => default,
    }
}

fn declared_label(node: &NativeNode) -> Arc<str> {
    Arc::from(
        node.string(property::ACCESSIBILITY_LABEL)
            .filter(|label| label.len() <= MAX_COMPONENT_VALUE_BYTES)
            .unwrap_or(""),
    )
}

fn declared_text(node: &NativeNode, key: u16) -> Option<Arc<str>> {
    node.string(key)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(Arc::from)
}

/// Everything a picker's surface parts declare about the popup the core paints in its own window.
///
/// The option list lives in a separate native child window, so these parts are declarations rather
/// than owner-window elements: they name the placement the core resolves against the display work
/// area and the scroll affordances the surface renderer mounts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct NativePickerSurface {
    pub(super) scroll_up_arrow: bool,
    pub(super) scroll_down_arrow: bool,
    pub(super) side: Option<AnchorSide>,
    pub(super) align: Option<AnchorAlign>,
    pub(super) side_offset: Option<f32>,
    pub(super) trigger_height: Option<f32>,
}

impl NativePickerSurface {
    fn fingerprint(self) -> String {
        format!(
            "{}|{}|{:?}|{:?}|{:?}|{:?}",
            u8::from(self.scroll_up_arrow),
            u8::from(self.scroll_down_arrow),
            self.side,
            self.align,
            self.side_offset,
            self.trigger_height,
        )
    }
}

/// Collect every declared surface part, keyed by the scope its picker was declared under.
pub(super) fn gather_picker_surfaces(tree: &NativeTree) -> HashMap<u64, NativePickerSurface> {
    let mut surfaces: HashMap<u64, NativePickerSurface> = HashMap::new();
    for (id, node) in &tree.nodes {
        let Some(part) = node.string(property::PART) else {
            continue;
        };
        if !matches!(
            part,
            SELECT_SCROLL_UP_ARROW_PART
                | SELECT_SCROLL_DOWN_ARROW_PART
                | SELECT_PORTAL_PART
                | SELECT_POSITIONER_PART
                | COMBOBOX_PORTAL_PART
                | COMBOBOX_POSITIONER_PART
        ) {
            continue;
        }
        if surfaces.len() >= MAX_COMPONENT_INSTANCES
            && !surfaces.contains_key(&component_key(*id, node))
        {
            continue;
        }
        let entry = surfaces.entry(component_key(*id, node)).or_default();
        match part {
            SELECT_SCROLL_UP_ARROW_PART => entry.scroll_up_arrow = true,
            SELECT_SCROLL_DOWN_ARROW_PART => entry.scroll_down_arrow = true,
            _ => {
                entry.side = match node.string(property::SIDE) {
                    Some("top") => Some(AnchorSide::Top),
                    Some("bottom") => Some(AnchorSide::Bottom),
                    Some("left") => Some(AnchorSide::Left),
                    Some("right") => Some(AnchorSide::Right),
                    _ => entry.side,
                };
                entry.align = match node.string(property::ALIGN) {
                    Some("start") => Some(AnchorAlign::Start),
                    Some("center") => Some(AnchorAlign::Center),
                    Some("end") => Some(AnchorAlign::End),
                    _ => entry.align,
                };
                entry.side_offset = node
                    .number(property::SIDE_OFFSET)
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .map(|value| value.min(MAX_POPOVER_SIDE_OFFSET))
                    .or(entry.side_offset);
                // Base UI's `alignItemWithTrigger` needs the trigger's own height, which a hosted
                // renderer measures itself and declares; the core owns everything derived from it.
                entry.trigger_height = node
                    .number(property::HEIGHT)
                    .filter(|value| value.is_finite() && *value > 0.0)
                    .or(entry.trigger_height);
            }
        }
    }
    surfaces
}

/// A fingerprint of everything a picker's retained core state is built from.
///
/// The core's replacement mutators all need an `EventContext` because they close a live native
/// popover, and a render pass has none. The binding therefore rebuilds a picker's state whenever
/// this fingerprint changes and the popover is closed, which is exactly when a rebuild is free.
fn picker_declaration(node: &NativeNode, surface: NativePickerSurface) -> Arc<str> {
    let mut fingerprint = String::new();
    for key in [
        property::OPTIONS,
        property::APPEARANCE,
        property::FILTER_MODE,
        property::ACTIVE_VALUE,
        property::VALUES,
        property::INPUT_VALUE,
    ] {
        fingerprint.push('\u{1f}');
        fingerprint.push_str(node.string(key).unwrap_or(""));
    }
    // Every prop below is a consuming core builder, so a change to any of them needs the rebuild
    // this fingerprint schedules for the next moment the surface is closed.
    for key in [
        property::MULTIPLE,
        property::READ_ONLY,
        property::REQUIRED,
        property::MODAL,
        property::ALIGN_ITEM_WITH_TRIGGER,
        property::AUTO_HIGHLIGHT,
        property::OPEN_ON_INPUT_CLICK,
        property::HIGHLIGHT_ITEM_ON_HOVER,
        property::LOOP_FOCUS,
    ] {
        fingerprint.push('\u{1f}');
        fingerprint.push_str(match node.boolean(key) {
            Some(true) => "1",
            Some(false) => "0",
            None => "",
        });
    }
    fingerprint.push('\u{1f}');
    fingerprint.push_str(&surface.fingerprint());
    Arc::from(fingerprint.as_str())
}

/// Whether a picker's child option declarations changed shape.
///
/// Child `Select.Item` / `Combobox.Item` nodes are not properties of the picker node, so they are
/// not part of the property fingerprint; their declaration is summarized here instead.
fn child_option_declaration(
    node: &NativeNode,
    key: u64,
    options: &HashMap<u64, Vec<DeclaredOption>>,
) -> Arc<str> {
    if node.string(property::OPTIONS).is_some() {
        return Arc::from("");
    }
    let Some(declared) = options.get(&key) else {
        return Arc::from("");
    };
    let mut fingerprint = String::new();
    for option in declared {
        fingerprint.push('\u{1f}');
        fingerprint.push_str(&option.value);
        fingerprint.push('\u{1e}');
        fingerprint.push_str(option.label.as_deref().unwrap_or(""));
        fingerprint.push('\u{1e}');
        fingerprint.push_str(option.detail.as_deref().unwrap_or(""));
        fingerprint.push('\u{1e}');
        fingerprint.push_str(option.group.as_deref().unwrap_or(""));
        fingerprint.push('\u{1e}');
        fingerprint.push_str(if option.disabled { "1" } else { "0" });
    }
    Arc::from(fingerprint.as_str())
}

/// One retained select instance.
pub(super) struct NativeSelectState {
    pub(super) state: SelectState<Arc<str>>,
    /// The surface parts this instance declared, read back by the trigger's own renderer.
    pub(super) surface: NativePickerSurface,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Option<Arc<str>>,
    reported_open: bool,
    reported_state: Option<serde_json::Value>,
}

impl Default for NativeSelectState {
    fn default() -> Self {
        Self {
            state: SelectState::new(Vec::new()).expect("an empty select source is always valid"),
            surface: NativePickerSurface::default(),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: None,
            reported_open: false,
            reported_state: None,
        }
    }
}

/// One retained constrained-combobox instance.
pub(super) struct NativeComboboxState {
    pub(super) state: ComboboxState<Arc<str>>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported: Option<Arc<str>>,
    reported_input: Arc<str>,
    reported_open: bool,
    reported_state: Option<serde_json::Value>,
}

impl Default for NativeComboboxState {
    fn default() -> Self {
        Self {
            state: ComboboxState::new(Vec::new())
                .expect("an empty combobox source is always valid"),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported: None,
            reported_input: Arc::from(""),
            reported_open: false,
            reported_state: None,
        }
    }
}

/// One retained free-form autocomplete instance.
pub(super) struct NativeAutocompleteState {
    pub(super) state: AutocompleteState<Arc<str>>,
    owner: u32,
    listens: bool,
    declaration: Arc<str>,
    reported_input: Arc<str>,
    reported_open: bool,
}

impl Default for NativeAutocompleteState {
    fn default() -> Self {
        Self {
            state: AutocompleteState::new(Vec::new())
                .expect("an empty autocomplete source is always valid"),
            owner: 0,
            listens: false,
            declaration: Arc::from(""),
            reported_input: Arc::from(""),
            reported_open: false,
        }
    }
}

/// A per-instance accessor from the hosted view to one declared select's retained state.
fn select_accessor(key: u64) -> StateAccessor<NativeView, SelectState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.selects.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared combobox's retained state.
fn combobox_accessor(key: u64) -> StateAccessor<NativeView, ComboboxState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.comboboxes.entry(key).or_default().state
    })
}

/// A per-instance accessor from the hosted view to one declared autocomplete's retained state.
fn autocomplete_accessor(key: u64) -> StateAccessor<NativeView, AutocompleteState<Arc<str>>> {
    StateAccessor::new(move |view: &mut NativeView| {
        &mut view.components.autocompletes.entry(key).or_default().state
    })
}

impl NativeComponentStates {
    /// Reseed one declared select from this frame's declaration.
    pub(super) fn sync_select(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        options: &HashMap<u64, Vec<DeclaredOption>>,
        surface: NativePickerSurface,
    ) {
        let mut declaration = picker_declaration(node, surface).to_string();
        declaration.push_str(&child_option_declaration(node, key, options));
        let declaration: Arc<str> = Arc::from(declaration.as_str());
        let listens = declares_change(node);
        let retained = self.selects.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        retained.surface = surface;
        // Rebuilding while a native popover is open would orphan that window, so an in-flight
        // declaration waits for the core to close it; the close itself schedules another frame.
        if retained.declaration == declaration || retained.state.is_open() {
            return;
        }
        let style = picker_style(node);
        let mut layout = style.select_layout();
        if let Some(side) = surface.side {
            layout = layout.placement(anchor_placement(
                side,
                surface.align.unwrap_or(AnchorAlign::Start),
            ));
        }
        if let Some(offset) = surface.side_offset {
            layout = layout.anchor_gap(offset);
        }
        if let Some(height) = surface.trigger_height {
            layout = layout.trigger_height(height);
        }
        let multiple = node.boolean(property::MULTIPLE).unwrap_or(false);
        let mut state = SelectState::new(declared_options(node, key, options))
            .unwrap_or_else(|_| {
                SelectState::new(Vec::new()).expect("an empty select source is always valid")
            })
            .with_layout(layout)
            .multiple(multiple)
            .modal(node.boolean(property::MODAL).unwrap_or(false))
            .align_item_with_trigger(
                node.boolean(property::ALIGN_ITEM_WITH_TRIGGER)
                    .unwrap_or(false),
            )
            .required(node.boolean(property::REQUIRED).unwrap_or(false));
        // The controlled value is seeded before `read_only`, which refuses every later change.
        if multiple {
            for value in declared_strings(node, property::VALUES) {
                state.select_id(ElementId::named(value.as_str()));
            }
        } else if let Some(value) = declared_text(node, property::ACTIVE_VALUE) {
            state.select_id(ElementId::named(value.as_ref()));
        }
        // Seeding is a declaration, not an interaction, so the core's own dirty and touched edges
        // start clean; only what the user really does moves them afterwards.
        state.reset_dirty();
        state.reset_touched();
        retained.state = state.read_only(node.boolean(property::READ_ONLY).unwrap_or(false));
        retained.declaration = declaration;
        retained.reported = retained.state.selected_value().cloned();
        retained.reported_open = false;
        retained.reported_state = None;
    }

    /// Reseed one declared constrained combobox from this frame's declaration.
    pub(super) fn sync_combobox(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        options: &HashMap<u64, Vec<DeclaredOption>>,
        surface: NativePickerSurface,
    ) {
        let mut declaration = picker_declaration(node, surface).to_string();
        declaration.push_str(&child_option_declaration(node, key, options));
        let declaration: Arc<str> = Arc::from(declaration.as_str());
        let listens = declares_change(node);
        let retained = self.comboboxes.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration || retained.state.is_open() {
            return;
        }
        let style = picker_style(node);
        let mut layout = style.suggestion_layout();
        if let Some(offset) = surface.side_offset {
            layout = layout.anchor_gap(offset);
        }
        if let Some(side) = surface.side {
            layout = layout.placement(anchor_placement(
                side,
                surface.align.unwrap_or(AnchorAlign::Start),
            ));
        }
        let multiple = node.boolean(property::MULTIPLE).unwrap_or(false);
        let mut state = ComboboxState::new(declared_options(node, key, options))
            .unwrap_or_else(|_| {
                ComboboxState::new(Vec::new()).expect("an empty combobox source is always valid")
            })
            .with_layout(layout)
            .with_filter_mode(declared_filter_mode(node, PickerFilterMode::Contains))
            .multiple(multiple)
            .auto_highlight(node.boolean(property::AUTO_HIGHLIGHT).unwrap_or(false))
            .open_on_input_click(node.boolean(property::OPEN_ON_INPUT_CLICK).unwrap_or(true))
            .highlight_item_on_hover(
                node.boolean(property::HIGHLIGHT_ITEM_ON_HOVER)
                    .unwrap_or(true),
            )
            .loop_focus(node.boolean(property::LOOP_FOCUS).unwrap_or(true))
            .required(node.boolean(property::REQUIRED).unwrap_or(false));
        if let Some(value) = declared_text(node, property::ACTIVE_VALUE) {
            state = state.with_selected_id(ElementId::named(value.as_ref()));
        }
        // A multiple combobox's declared value set becomes its chips, bounded by the core's own
        // `MAX_COMBOBOX_VALUES`; a value that names no declared option adds no chip at all.
        if multiple {
            for value in declared_strings(node, property::VALUES) {
                let id = ElementId::named(value.as_str());
                let Some(index) = state
                    .items()
                    .iter()
                    .position(|item| item.stable_id() == Some(id))
                else {
                    continue;
                };
                state.add_chip_source(index);
            }
        }
        state.reset_dirty();
        state.reset_touched();
        retained.state = state.read_only(node.boolean(property::READ_ONLY).unwrap_or(false));
        retained.declaration = declaration;
        retained.reported = retained.state.selected_value().cloned();
        retained.reported_input = retained.state.input_value().clone();
        retained.reported_open = false;
        retained.reported_state = None;
    }

    /// Reseed one declared free-form autocomplete from this frame's declaration.
    pub(super) fn sync_autocomplete(
        &mut self,
        key: u64,
        id: u32,
        node: &NativeNode,
        options: &HashMap<u64, Vec<DeclaredOption>>,
        surface: NativePickerSurface,
    ) {
        let mut declaration = picker_declaration(node, surface).to_string();
        declaration.push_str(&child_option_declaration(node, key, options));
        let declaration: Arc<str> = Arc::from(declaration.as_str());
        let listens = declares_change(node);
        let retained = self.autocompletes.entry(key).or_default();
        retained.owner = id;
        retained.listens = listens;
        if retained.declaration == declaration || retained.state.is_open() {
            return;
        }
        let style = picker_style(node);
        let state = AutocompleteState::new(declared_options(node, key, options))
            .unwrap_or_else(|_| {
                AutocompleteState::new(Vec::new())
                    .expect("an empty autocomplete source is always valid")
            })
            .with_layout(style.suggestion_layout())
            .with_filter_mode(declared_filter_mode(node, PickerFilterMode::Fuzzy))
            .with_value(node.string(property::INPUT_VALUE).unwrap_or(""));
        retained.state = state;
        retained.declaration = declaration;
        retained.reported_input = retained.state.value().clone();
        retained.reported_open = false;
    }

    /// Enqueue one asynchronous change event per picker the core moved since the last pass.
    pub(super) fn report_pickers(&mut self, window: u32, events: &EventQueue) {
        for retained in self.selects.values_mut() {
            let value = retained.state.selected_value().cloned();
            let open = retained.state.is_open();
            let part_state = select_part_state(&retained.state);
            if value == retained.reported
                && open == retained.reported_open
                && retained.reported_state.as_ref() == Some(&part_state)
            {
                continue;
            }
            let mut payload = serde_json::Map::new();
            if value != retained.reported {
                payload.insert("value".to_owned(), optional_text(value.as_deref()));
            }
            payload.insert(
                "selectedValues".to_owned(),
                serde_json::Value::Array(
                    retained
                        .state
                        .selected_values()
                        .map(|value| serde_json::Value::String(value.to_string()))
                        .collect(),
                ),
            );
            payload.insert(
                "valueText".to_owned(),
                optional_text(retained.state.value_text().as_deref()),
            );
            payload.insert("open".to_owned(), serde_json::Value::Bool(open));
            payload.insert("state".to_owned(), part_state.clone());
            retained.reported.clone_from(&value);
            retained.reported_open = open;
            retained.reported_state = Some(part_state);
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::Value::Object(payload),
                );
            }
        }
        for retained in self.comboboxes.values_mut() {
            let value = retained.state.selected_value().cloned();
            let input = retained.state.input_value().clone();
            let open = retained.state.is_open();
            let part_state = combobox_part_state(&retained.state);
            if value == retained.reported
                && input == retained.reported_input
                && open == retained.reported_open
                && retained.reported_state.as_ref() == Some(&part_state)
            {
                continue;
            }
            let mut payload = serde_json::Map::new();
            if value != retained.reported {
                payload.insert("value".to_owned(), optional_text(value.as_deref()));
            }
            if input != retained.reported_input {
                payload.insert(
                    "inputValue".to_owned(),
                    serde_json::Value::String(input.to_string()),
                );
            }
            payload.insert(
                "chipValues".to_owned(),
                serde_json::Value::Array(
                    retained
                        .state
                        .chip_values()
                        .map(|value| serde_json::Value::String(value.to_string()))
                        .collect(),
                ),
            );
            payload.insert(
                "chipLabels".to_owned(),
                serde_json::Value::Array(
                    retained
                        .state
                        .chip_labels()
                        .map(|label| serde_json::Value::String(label.to_string()))
                        .collect(),
                ),
            );
            payload.insert("open".to_owned(), serde_json::Value::Bool(open));
            payload.insert("state".to_owned(), part_state.clone());
            retained.reported.clone_from(&value);
            retained.reported_input = input;
            retained.reported_open = open;
            retained.reported_state = Some(part_state);
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::Value::Object(payload),
                );
            }
        }
        for retained in self.autocompletes.values_mut() {
            let input = retained.state.value().clone();
            let open = retained.state.is_open();
            if input == retained.reported_input && open == retained.reported_open {
                continue;
            }
            let mut payload = serde_json::Map::new();
            if input != retained.reported_input {
                payload.insert(
                    "inputValue".to_owned(),
                    serde_json::Value::String(input.to_string()),
                );
            }
            payload.insert("open".to_owned(), serde_json::Value::Bool(open));
            retained.reported_input = input;
            retained.reported_open = open;
            if retained.listens {
                enqueue_component_change(
                    events,
                    window,
                    retained.owner,
                    serde_json::Value::Object(payload),
                );
            }
        }
    }
}

/// The core's own `SelectPartState`, exactly as Base UI publishes it on the trigger.
fn select_part_state(state: &SelectState<Arc<str>>) -> serde_json::Value {
    let part = state.state();
    serde_json::json!({
        "popupOpen": part.popup_open,
        "popupSide": match part.popup_side {
            AnchorSide::Top => "top",
            AnchorSide::Bottom => "bottom",
            AnchorSide::Left => "left",
            AnchorSide::Right => "right",
        },
        "pressed": part.pressed,
        "placeholder": part.placeholder,
        "valid": part.valid,
        "invalid": part.invalid,
        "dirty": part.dirty,
        "touched": part.touched,
        "filled": part.filled,
        "focused": part.focused,
        "readOnly": part.read_only,
        "required": part.required,
    })
}

/// The core's own `ComboboxPartState`, plus the Status text and Empty edge it derives.
fn combobox_part_state(state: &ComboboxState<Arc<str>>) -> serde_json::Value {
    let part = state.state();
    serde_json::json!({
        "popupOpen": part.popup_open,
        "pressed": part.pressed,
        "placeholder": part.placeholder,
        "valid": part.valid,
        "invalid": part.invalid,
        "dirty": part.dirty,
        "touched": part.touched,
        "filled": part.filled,
        "focused": part.focused,
        "readOnly": part.read_only,
        "required": part.required,
        "status": state.status_text().to_string(),
        "empty": state.is_empty_result(),
        "resultCount": state.result_count(),
    })
}

/// Record trigger and input focus so the core owns `focused` and the `touched` edge it implies.
pub(super) fn sync_picker_focus(
    components: &mut NativeComponentStates,
    focused: Option<ElementId>,
) -> bool {
    let mut changed = false;
    for (key, retained) in &mut components.selects {
        changed |= retained
            .state
            .set_focused(focused == Some(ElementId::new(*key)));
    }
    for (key, retained) in &mut components.comboboxes {
        changed |= retained
            .state
            .set_focused(focused == Some(ElementId::new(*key)));
    }
    changed
}

fn optional_text(value: Option<&str>) -> serde_json::Value {
    match value {
        Some(value) => serde_json::Value::String(value.to_owned()),
        None => serde_json::Value::Null,
    }
}

/// Publish one committed picker value as its own asynchronous `commit` event.
///
/// Commit is an edge, not a value: committing the same option twice reports twice while the
/// controlled value never moves, so it cannot travel on the `componentchange` value channel.
fn enqueue_commit(events: &EventQueue, window: u32, target: u32, value: serde_json::Value) {
    enqueue_event(
        events,
        QueuedEvent {
            kind: "commit",
            window,
            target,
            value: Some(Arc::from(value.to_string().as_str())),
        },
    );
}

/// Apply one declared picker part.
///
/// The trigger or input element the caller declared is handed straight to the core, which returns
/// it decorated with the complete interaction. Nothing here filters, highlights, or commits.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_picker_part(
    element: Element,
    part: &str,
    id: u32,
    window: u32,
    node: &NativeNode,
    events: &EventQueue,
    components: &mut NativeComponentStates,
    cx: &mut ViewContext<'_, NativeView>,
    listeners_enabled: bool,
) -> Option<Element> {
    // Every other declared part reaches this function too, so the untouched path leaves before it
    // hashes a scope, decodes an appearance, or allocates a label.
    if !owns_picker_part(part) {
        return Some(element);
    }
    // The core paints the option and result lists in its own native window, so the popup-side
    // parts contribute a declaration and no element at all.
    if picker_part_is_declaration(part) {
        return None;
    }
    let key = component_key(id, node);
    let root = ElementId::new(key);
    let style = picker_style(node);
    let label = declared_label(node);
    let commits = node.boolean(property::COMMIT_LISTENER).unwrap_or(false);
    match part {
        SELECT_PART => {
            let Some(retained) = components.selects.get(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(retained.state.trigger_part(root, label, element));
            }
            let arrows = retained.surface;
            // Base UI's `Select.ScrollUpArrow` and `Select.ScrollDownArrow` live inside the option
            // surface, so the core's own popup-part decorators carry their hovered scrolling.
            let popup = move |list: SelectListState, parts: &quickgui::SelectPopupParts<'_>| {
                let mut root = style.surface();
                if arrows.scroll_up_arrow && list.can_scroll_up {
                    root = root.child(parts.scroll_up_arrow_part(style.scroll_arrow("\u{25b2}")));
                }
                root = root.child(div().flex_col().flex_1().min_h(0.0));
                if arrows.scroll_down_arrow && list.can_scroll_down {
                    root = root.child(parts.scroll_down_arrow_part(style.scroll_arrow("\u{25bc}")));
                }
                root
            };
            let plain = move |_list: SelectListState| style.surface();
            let option = move |item: &PickerItem<Arc<str>>, state: SelectOptionState| {
                style
                    .row(item, state.active, state.selected, state.disabled)
                    .h(style.row_height)
            };
            let commit_events = Rc::clone(events);
            // The core has already committed the value into the retained state; the binding only
            // publishes the edge and asks for one more frame so the report pass runs.
            let change = move |_view: &mut NativeView, value: Arc<str>, cx: &mut EventContext| {
                if commits {
                    enqueue_commit(
                        &commit_events,
                        window,
                        id,
                        serde_json::json!({ "value": value.as_ref() }),
                    );
                }
                cx.invalidate();
            };
            let trigger = if arrows.scroll_up_arrow || arrows.scroll_down_arrow {
                retained.state.element_with_parts_with(
                    cx,
                    root,
                    label,
                    select_accessor(key),
                    element,
                    popup,
                    option,
                    change,
                )
            } else {
                retained.state.element_with(
                    cx,
                    root,
                    label,
                    select_accessor(key),
                    element,
                    plain,
                    option,
                    change,
                )
            };
            Some(pressed_listeners(trigger, root, select_accessor(key), cx))
        }
        COMBOBOX_PART => {
            let Some(retained) = components.comboboxes.get_mut(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(element.id(root));
            }
            let surface = move |_list: ComboboxListState| style.surface();
            let option = move |item: &PickerItem<Arc<str>>, state: ComboboxOptionState| {
                style
                    .row(item, state.active, state.selected, state.disabled)
                    .h(style.row_height)
            };
            let commit_events = Rc::clone(events);
            Some(retained.state.element_with(
                cx,
                root,
                label,
                combobox_accessor(key),
                element,
                surface,
                option,
                move |_view: &mut NativeView, _query: Arc<str>, cx: &mut EventContext| {
                    cx.invalidate();
                },
                move |_view: &mut NativeView, value: Arc<str>, cx: &mut EventContext| {
                    if commits {
                        enqueue_commit(
                            &commit_events,
                            window,
                            id,
                            serde_json::json!({ "value": value.as_ref() }),
                        );
                    }
                    cx.invalidate();
                },
            ))
        }
        AUTOCOMPLETE_PART => {
            let Some(retained) = components.autocompletes.get_mut(&key) else {
                return Some(element);
            };
            if !listeners_enabled {
                return Some(element.id(root));
            }
            let surface = move |_list: AutocompleteListState| style.surface();
            let option = move |item: &PickerItem<Arc<str>>, state: AutocompleteOptionState| {
                style
                    .row(item, state.active, false, state.disabled)
                    .h(style.row_height)
            };
            let commit_events = Rc::clone(events);
            Some(retained.state.element_with(
                cx,
                root,
                label,
                autocomplete_accessor(key),
                element,
                surface,
                option,
                move |_view: &mut NativeView, _value: Arc<str>, cx: &mut EventContext| {
                    cx.invalidate();
                },
                move |_view: &mut NativeView, value: Arc<str>, cx: &mut EventContext| {
                    if commits {
                        enqueue_commit(
                            &commit_events,
                            window,
                            id,
                            serde_json::json!({ "value": value.as_ref() }),
                        );
                    }
                    cx.invalidate();
                },
            ))
        }
        // -------------------------------------------------------------------
        // Owner-window select parts
        // -------------------------------------------------------------------
        SELECT_LABEL_PART => Some(SelectState::<Arc<str>>::label_part(root, element)),
        SELECT_VALUE_PART => Some(SelectState::<Arc<str>>::value_part(root, element)),
        SELECT_ICON_PART => Some(SelectState::<Arc<str>>::icon_part(root, element)),
        SELECT_BACKDROP_PART => {
            let retained = components.selects.get(&key)?;
            if !retained.state.is_open() {
                return None;
            }
            Some(SelectState::<Arc<str>>::backdrop_part(root, element))
        }

        // -------------------------------------------------------------------
        // Owner-window combobox and autocomplete parts
        // -------------------------------------------------------------------
        COMBOBOX_LABEL_PART => Some(ComboboxState::<Arc<str>>::label_part(root, element)),
        COMBOBOX_VALUE_PART => Some(ComboboxState::<Arc<str>>::value_part(root, element)),
        COMBOBOX_ICON_PART => Some(ComboboxState::<Arc<str>>::icon_part(root, element)),
        COMBOBOX_INPUT_GROUP_PART => {
            Some(ComboboxState::<Arc<str>>::input_group_part(root, element))
        }
        COMBOBOX_CHIPS_PART => Some(ComboboxState::<Arc<str>>::chips_part(root, element)),
        COMBOBOX_CHIP_PART => {
            let retained = components.comboboxes.get(&key)?;
            let index = declared_index(node);
            let chip_label = retained
                .state
                .chip_labels()
                .nth(index)
                .cloned()
                .or_else(|| declared_text(node, property::VALUE))
                .unwrap_or_else(|| Arc::from(""));
            Some(ComboboxState::<Arc<str>>::chip_part(
                root, index, chip_label, element,
            ))
        }
        COMBOBOX_CHIP_REMOVE_PART => {
            let retained = components.comboboxes.get(&key)?;
            let index = declared_index(node);
            if index >= retained.state.chip_count() {
                return None;
            }
            let remove = ComboboxState::<Arc<str>>::chip_remove_part(
                root,
                index,
                node.string(property::ACCESSIBILITY_LABEL)
                    .unwrap_or("Remove"),
                element,
            );
            if !listeners_enabled {
                return Some(remove);
            }
            let remove_id = ComboboxState::<Arc<str>>::chip_remove_id(root, index);
            Some(remove.on_click(cx.listener(
                remove_id,
                move |view: &mut NativeView, cx: &mut EventContext| {
                    if let Some(retained) = view.components.comboboxes.get_mut(&key)
                        && retained.state.remove_chip(index)
                    {
                        cx.invalidate();
                    }
                },
            )))
        }
        COMBOBOX_CLEAR_PART => {
            let retained = components.comboboxes.get(&key)?;
            let clear = ComboboxState::<Arc<str>>::clear_part(
                root,
                node.string(property::ACCESSIBILITY_LABEL)
                    .unwrap_or("Clear"),
                element,
            );
            if !listeners_enabled {
                return Some(clear);
            }
            let _ = retained;
            let clear_id = ComboboxState::<Arc<str>>::clear_id(root);
            Some(clear.on_click(cx.listener(
                clear_id,
                move |view: &mut NativeView, cx: &mut EventContext| {
                    let Some(retained) = view.components.comboboxes.get_mut(&key) else {
                        return;
                    };
                    let mut changed = retained.state.clear_chips();
                    changed |= retained.state.clear_selection(cx);
                    if changed {
                        cx.invalidate();
                    }
                },
            )))
        }
        COMBOBOX_TRIGGER_PART => {
            let retained = components.comboboxes.get(&key)?;
            Some(retained.state.trigger_part(root, label, element))
        }
        COMBOBOX_BACKDROP_PART => {
            let retained = components.comboboxes.get(&key)?;
            if !retained.state.is_open() {
                return None;
            }
            Some(ComboboxState::<Arc<str>>::backdrop_part(root, element))
        }
        COMBOBOX_STATUS_PART => Some(ComboboxState::<Arc<str>>::status_part(root, element)),
        COMBOBOX_EMPTY_PART => {
            let retained = components.comboboxes.get(&key)?;
            // Base UI mounts `Combobox.Empty` only while the query really matched nothing, and the
            // core is the one that knows.
            if !retained.state.is_empty_result() {
                return None;
            }
            Some(ComboboxState::<Arc<str>>::empty_part(root, element))
        }
        _ => Some(element),
    }
}

/// Report the trigger's held-down state, Base UI's `data-pressed`, from the core's own flag.
fn pressed_listeners(
    element: Element,
    root: ElementId,
    access: StateAccessor<NativeView, SelectState<Arc<str>>>,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    let down_access = access.clone();
    let down = cx.mouse_down_listener(root, move |view: &mut NativeView, _event, cx| {
        if down_access.get(view).set_pressed(true) {
            cx.invalidate();
        }
    });
    let up = cx.mouse_up_listener(root, move |view: &mut NativeView, _event, cx| {
        if access.get(view).set_pressed(false) {
            cx.invalidate();
        }
    });
    element
        .on_mouse_down(quickgui::MouseButton::Left, down)
        .on_mouse_up(quickgui::MouseButton::Left, up)
}

/// The core-derived identity one declared picker part mounts under.
pub(super) fn picker_part_element_id(part: &str, id: u32, node: &NativeNode) -> Option<ElementId> {
    let root = ElementId::new(component_key(id, node));
    Some(match part {
        SELECT_PART | COMBOBOX_PART | AUTOCOMPLETE_PART => root,
        SELECT_LABEL_PART => SelectState::<Arc<str>>::label_id(root),
        SELECT_VALUE_PART => SelectState::<Arc<str>>::value_id(root),
        SELECT_ICON_PART => SelectState::<Arc<str>>::icon_id(root),
        SELECT_BACKDROP_PART => SelectState::<Arc<str>>::backdrop_id(root),
        COMBOBOX_LABEL_PART => ComboboxState::<Arc<str>>::label_id(root),
        COMBOBOX_VALUE_PART => ComboboxState::<Arc<str>>::value_id(root),
        COMBOBOX_ICON_PART => ComboboxState::<Arc<str>>::icon_id(root),
        COMBOBOX_INPUT_GROUP_PART => ComboboxState::<Arc<str>>::input_group_id(root),
        COMBOBOX_CLEAR_PART => ComboboxState::<Arc<str>>::clear_id(root),
        COMBOBOX_TRIGGER_PART => ComboboxState::<Arc<str>>::trigger_id(root),
        COMBOBOX_CHIPS_PART => ComboboxState::<Arc<str>>::chips_id(root),
        COMBOBOX_CHIP_PART => ComboboxState::<Arc<str>>::chip_id(root, declared_index(node)),
        COMBOBOX_CHIP_REMOVE_PART => {
            ComboboxState::<Arc<str>>::chip_remove_id(root, declared_index(node))
        }
        COMBOBOX_BACKDROP_PART => ComboboxState::<Arc<str>>::backdrop_id(root),
        COMBOBOX_STATUS_PART => ComboboxState::<Arc<str>>::status_id(root),
        COMBOBOX_EMPTY_PART => ComboboxState::<Arc<str>>::empty_id(root),
        _ => return None,
    })
}
