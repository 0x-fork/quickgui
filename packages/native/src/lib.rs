use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt,
    rc::Rc,
    sync::{
        Arc, Condvar, LazyLock, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use napi::{
    Env, Error, Result, Task,
    bindgen_prelude::{AsyncTask, Buffer, Function},
};
use napi_derive::napi;
use quickgui::{
    AccessibilityRole, AnchorPlacement, AnchoredPopover, App as QuickGuiApp, AppConfig, AppRegion,
    AppRunStatus, AppRunner, AppRunnerWaker, Color, CursorStyle, Element, ElementId, FontWeight,
    IntoElement, Markdown, MarkdownStyle, QuitMode, TextAlign, TitleBarStyle, View, ViewContext,
    WindowBackgroundAppearance, WindowHandle, button, div, text, text_area, text_input,
};

const PROTOCOL_MAGIC: &[u8; 4] = b"QGMB";
const PROTOCOL_VERSION: u16 = 3;
const ROOT_NODE: u32 = 0;
const ROOT_ELEMENT_ID: u64 = u64::MAX - 1;
const MAX_BATCH_BYTES: usize = 16 * 1024 * 1024;
const MAX_MUTATIONS: usize = 131_072;
const MAX_NODES: usize = 262_144;
const MAX_TREE_DEPTH: usize = 512;
const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAX_QUEUED_EVENTS: usize = 8_192;
const MAX_HOST_COMMANDS: usize = 8_192;
const MAX_WINDOWS: usize = 256;
const NO_ANCHOR: u32 = u32::MAX;

mod property {
    pub const DISPLAY: u16 = 1;
    pub const FLEX_DIRECTION: u16 = 2;
    pub const FLEX_WRAP: u16 = 3;
    pub const FLEX_GROW: u16 = 4;
    pub const FLEX_SHRINK: u16 = 5;
    pub const FLEX_BASIS: u16 = 6;
    pub const ALIGN_ITEMS: u16 = 7;
    pub const ALIGN_SELF: u16 = 8;
    pub const JUSTIFY_CONTENT: u16 = 9;
    pub const ALIGN_CONTENT: u16 = 10;
    pub const GAP: u16 = 11;
    pub const COLUMN_GAP: u16 = 12;
    pub const ROW_GAP: u16 = 13;
    pub const WIDTH: u16 = 14;
    pub const HEIGHT: u16 = 15;
    pub const MIN_WIDTH: u16 = 16;
    pub const MIN_HEIGHT: u16 = 17;
    pub const MAX_WIDTH: u16 = 18;
    pub const MAX_HEIGHT: u16 = 19;
    pub const PADDING: u16 = 20;
    pub const PADDING_TOP: u16 = 21;
    pub const PADDING_RIGHT: u16 = 22;
    pub const PADDING_BOTTOM: u16 = 23;
    pub const PADDING_LEFT: u16 = 24;
    pub const MARGIN: u16 = 25;
    pub const MARGIN_TOP: u16 = 26;
    pub const MARGIN_RIGHT: u16 = 27;
    pub const MARGIN_BOTTOM: u16 = 28;
    pub const MARGIN_LEFT: u16 = 29;
    pub const BACKGROUND_COLOR: u16 = 30;
    pub const COLOR: u16 = 31;
    pub const OPACITY: u16 = 32;
    pub const BORDER_WIDTH: u16 = 33;
    pub const BORDER_COLOR: u16 = 34;
    pub const BORDER_RADIUS: u16 = 35;
    pub const FONT_SIZE: u16 = 36;
    pub const FONT_WEIGHT: u16 = 37;
    pub const LINE_HEIGHT: u16 = 38;
    pub const TEXT_ALIGN: u16 = 39;
    pub const WHITE_SPACE: u16 = 40;
    pub const TEXT_OVERFLOW: u16 = 41;
    pub const LINE_CLAMP: u16 = 42;
    pub const OVERFLOW: u16 = 43;
    pub const OVERFLOW_X: u16 = 44;
    pub const OVERFLOW_Y: u16 = 45;
    pub const CURSOR: u16 = 46;
    pub const APP_REGION: u16 = 47;
    pub const DISABLED: u16 = 48;
    pub const ACCESSIBILITY_LABEL: u16 = 49;
    pub const ROLE: u16 = 50;
    pub const TAB_INDEX: u16 = 51;
    pub const POSITION: u16 = 52;
    pub const TOP: u16 = 53;
    pub const RIGHT: u16 = 54;
    pub const BOTTOM: u16 = 55;
    pub const LEFT: u16 = 56;
    pub const USER_SELECT: u16 = 57;
    pub const CLICK_LISTENER: u16 = 58;
    pub const HOVER_LISTENER: u16 = 59;
    pub const VISIBILITY: u16 = 60;
    pub const ASPECT_RATIO: u16 = 61;
    pub const VALUE: u16 = 62;
    pub const PLACEHOLDER: u16 = 63;
    pub const MULTILINE: u16 = 64;
    pub const INPUT_LISTENER: u16 = 65;
    pub const SUBMIT_LISTENER: u16 = 66;
    pub const STREAMING: u16 = 67;
    pub const MARKDOWN_CODE_BACKGROUND: u16 = 68;
    pub const MARKDOWN_BORDER_COLOR: u16 = 69;
    pub const MARKDOWN_MUTED_COLOR: u16 = 70;
    pub const MARKDOWN_LINK_COLOR: u16 = 71;
    pub const MARKDOWN_CODE_TEXT_COLOR: u16 = 72;
    pub const MARKDOWN_BLOCK_GAP: u16 = 73;
    pub const MARKDOWN_CODE_FONT_SIZE: u16 = 74;
    pub const SCROLL_TO_END_REVISION: u16 = 75;
    pub const PASSWORD: u16 = 76;
    pub const LAST: u16 = PASSWORD;
}

#[derive(Clone, Default)]
#[napi(object)]
pub struct NativeWindowOptions {
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub minimum_width: Option<f64>,
    pub minimum_height: Option<f64>,
    pub background: Option<u32>,
    pub title_bar_style: Option<String>,
    pub traffic_light_x: Option<f64>,
    pub traffic_light_y: Option<f64>,
    pub transparent: Option<bool>,
    pub blur: Option<bool>,
    pub popup_placement: Option<String>,
    pub popup_gap: Option<f64>,
    pub popup_offset_x: Option<f64>,
    pub popup_offset_y: Option<f64>,
    pub popup_grab: Option<bool>,
    pub popup_accepts_key_focus: Option<bool>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeEvent {
    pub kind: String,
    pub window: u32,
    pub target: u32,
    pub value: Option<String>,
}

#[derive(Clone)]
#[napi(object)]
pub struct HostedAppUpdate {
    pub events: Vec<NativeEvent>,
    pub exit_code: Option<i32>,
}

struct SyncReply<T> {
    value: Mutex<Option<std::result::Result<T, String>>>,
    ready: Condvar,
}

impl<T> SyncReply<T> {
    fn new() -> Self {
        Self {
            value: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    fn complete(&self, value: std::result::Result<T, String>) {
        *lock(&self.value) = Some(value);
        self.ready.notify_all();
    }

    fn wait(&self) -> std::result::Result<T, String> {
        let mut value = lock(&self.value);
        while value.is_none() {
            value = wait(&self.ready, value);
        }
        value
            .take()
            .expect("a completed QuickGUI host reply must contain a value")
    }
}

enum HostCommand {
    CreateApp {
        app: u32,
        reply: Arc<SyncReply<()>>,
    },
    CreateWindow {
        app: u32,
        options: NativeWindowOptions,
        reply: Arc<SyncReply<u32>>,
    },
    CreateAnchoredWindow {
        app: u32,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        reply: Arc<SyncReply<u32>>,
    },
    ApplyBatch {
        app: u32,
        window: u32,
        batch: Vec<u8>,
    },
    CloseWindow {
        app: u32,
        window: u32,
        reply: Arc<SyncReply<bool>>,
    },
    FocusNode {
        app: u32,
        window: u32,
        node: u32,
        reply: Arc<SyncReply<bool>>,
    },
    StartApp {
        app: u32,
        reply: Arc<SyncReply<()>>,
    },
    DestroyApp {
        app: u32,
        reply: Arc<SyncReply<bool>>,
    },
}

#[derive(Default)]
struct HostState {
    running: bool,
    app: Option<u32>,
    commands: VecDeque<HostCommand>,
    events: VecDeque<NativeEvent>,
    exit_code: Option<i32>,
    failure: Option<String>,
    waker: Option<AppRunnerWaker>,
}

struct HostCoordinator {
    next_app: AtomicU32,
    state: Mutex<HostState>,
    changed: Condvar,
}

impl HostCoordinator {
    fn new() -> Self {
        Self {
            next_app: AtomicU32::new(1),
            state: Mutex::new(HostState::default()),
            changed: Condvar::new(),
        }
    }

    fn allocate_app(&self) -> std::result::Result<u32, String> {
        let app = self
            .next_app
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                id.checked_add(1).filter(|next| *next != 0)
            })
            .map_err(|_| "QuickGUI hosted app id space exhausted".to_owned())?
            .max(1);
        Ok(app)
    }

    fn enqueue(&self, command: HostCommand) -> std::result::Result<(), String> {
        let waker = {
            let mut state = lock(&self.state);
            if let Some(error) = state.failure.clone() {
                return Err(error);
            }
            if state.commands.len() >= MAX_HOST_COMMANDS {
                return Err("QuickGUI host command queue is full".to_owned());
            }
            state.commands.push_back(command);
            state.waker.clone()
        };
        self.changed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
        Ok(())
    }

    fn begin(&self) -> std::result::Result<(), String> {
        let mut state = lock(&self.state);
        if state.running {
            return Err("the QuickGUI native host is already running".to_owned());
        }
        state.running = true;
        Ok(())
    }

    fn finish(&self) {
        let mut state = lock(&self.state);
        state.running = false;
        state.waker = None;
        self.changed.notify_all();
    }

    fn wait_for_commands(&self) -> std::result::Result<VecDeque<HostCommand>, String> {
        let mut state = lock(&self.state);
        while state.commands.is_empty() && state.failure.is_none() {
            state = wait(&self.changed, state);
        }
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.commands))
    }

    fn take_commands(&self) -> std::result::Result<VecDeque<HostCommand>, String> {
        let mut state = lock(&self.state);
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.commands))
    }

    fn set_app(&self, app: u32) -> std::result::Result<(), String> {
        let mut state = lock(&self.state);
        if state.app.is_some() {
            return Err("a QuickGUI hosted app is already active".to_owned());
        }
        state.app = Some(app);
        state.events.clear();
        state.exit_code = None;
        state.failure = None;
        Ok(())
    }

    fn set_waker(&self, waker: AppRunnerWaker) {
        lock(&self.state).waker = Some(waker);
    }

    fn publish_events(&self, events: impl IntoIterator<Item = NativeEvent>) {
        let mut state = lock(&self.state);
        for event in events {
            if state.events.len() >= MAX_QUEUED_EVENTS {
                break;
            }
            state.events.push_back(event);
        }
        drop(state);
        self.changed.notify_all();
    }

    fn publish_exit(&self, code: i32) {
        let mut state = lock(&self.state);
        state.exit_code = Some(code.max(0));
        state.waker = None;
        drop(state);
        self.changed.notify_all();
    }

    fn fail(&self, error: String) {
        let waker = {
            let mut state = lock(&self.state);
            if state.failure.is_none() {
                state.failure = Some(error);
            }
            state.waker.clone()
        };
        self.changed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn wait_for_update(&self, app: u32) -> std::result::Result<HostedAppUpdate, String> {
        let mut state = lock(&self.state);
        loop {
            if state.app != Some(app) {
                return Err(format!("unknown QuickGUI hosted app {app}"));
            }
            if let Some(error) = state.failure.clone() {
                return Err(error);
            }
            if !state.events.is_empty() || state.exit_code.is_some() {
                return Ok(HostedAppUpdate {
                    events: state.events.drain(..).collect(),
                    exit_code: state.exit_code,
                });
            }
            state = wait(&self.changed, state);
        }
    }
}

static HOST: LazyLock<HostCoordinator> = LazyLock::new(HostCoordinator::new);

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait<'a, T>(
    condvar: &Condvar,
    guard: std::sync::MutexGuard<'a, T>,
) -> std::sync::MutexGuard<'a, T> {
    condvar
        .wait(guard)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeTag {
    Root,
    View,
    Button,
    Text,
    Sentinel,
    Input,
    Markdown,
}

impl NodeTag {
    fn decode(value: u8) -> std::result::Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::View),
            2 => Ok(Self::Button),
            3 => Ok(Self::Text),
            4 => Ok(Self::Sentinel),
            5 => Ok(Self::Input),
            6 => Ok(Self::Markdown),
            _ => Err(ProtocolError::new(format!("unknown node tag {value}"))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PropertyValue {
    Bool(bool),
    Number(f32),
    Color(u32),
    String(Arc<str>),
}

#[derive(Clone, Debug)]
struct NativeNode {
    tag: NodeTag,
    parent: Option<u32>,
    children: Vec<u32>,
    text: Arc<str>,
    properties: Vec<(u16, PropertyValue)>,
}

impl NativeNode {
    fn new(tag: NodeTag) -> Self {
        Self {
            tag,
            parent: None,
            children: Vec::new(),
            text: Arc::from(""),
            properties: Vec::new(),
        }
    }

    fn property(&self, key: u16) -> Option<&PropertyValue> {
        self.properties
            .binary_search_by_key(&key, |(property, _)| *property)
            .ok()
            .map(|index| &self.properties[index].1)
    }

    fn number(&self, key: u16) -> Option<f32> {
        match self.property(key) {
            Some(PropertyValue::Number(value)) if value.is_finite() => Some(*value),
            _ => None,
        }
    }

    fn boolean(&self, key: u16) -> Option<bool> {
        match self.property(key) {
            Some(PropertyValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    fn string(&self, key: u16) -> Option<&str> {
        match self.property(key) {
            Some(PropertyValue::String(value)) => Some(value),
            _ => None,
        }
    }

    fn color(&self, key: u16) -> Option<Color> {
        match self.property(key) {
            Some(PropertyValue::Color(value)) => Some(unpack_color(*value)),
            _ => None,
        }
    }

    fn set_property(&mut self, key: u16, value: Option<PropertyValue>) {
        match self
            .properties
            .binary_search_by_key(&key, |(property, _)| *property)
        {
            Ok(index) => match value {
                Some(value) => self.properties[index].1 = value,
                None => {
                    self.properties.remove(index);
                }
            },
            Err(index) => {
                if let Some(value) = value {
                    self.properties.insert(index, (key, value));
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct NativeTree {
    nodes: HashMap<u32, NativeNode>,
    revision: u32,
}

impl Default for NativeTree {
    fn default() -> Self {
        let mut nodes = HashMap::with_capacity(64);
        nodes.insert(ROOT_NODE, NativeNode::new(NodeTag::Root));
        Self { nodes, revision: 0 }
    }
}

#[derive(Clone, Debug)]
enum Mutation {
    Create {
        id: u32,
        tag: NodeTag,
        text: Arc<str>,
    },
    SetProperty {
        id: u32,
        key: u16,
        value: Option<PropertyValue>,
    },
    ReplaceText {
        id: u32,
        text: Arc<str>,
    },
    Insert {
        parent: u32,
        child: u32,
        before: Option<u32>,
    },
    Remove {
        parent: u32,
        child: u32,
    },
    Cleanup {
        parent: u32,
        children: Vec<u32>,
    },
}

struct TreeTransaction<'a> {
    base: &'a NativeTree,
    overlay: HashMap<u32, Option<NativeNode>>,
}

impl<'a> TreeTransaction<'a> {
    fn new(base: &'a NativeTree) -> Self {
        Self {
            base,
            overlay: HashMap::new(),
        }
    }

    fn node(&self, id: u32) -> Option<&NativeNode> {
        match self.overlay.get(&id) {
            Some(node) => node.as_ref(),
            None => self.base.nodes.get(&id),
        }
    }

    fn edit(&mut self, id: u32) -> std::result::Result<&mut NativeNode, ProtocolError> {
        if !self.overlay.contains_key(&id) {
            let node = self
                .base
                .nodes
                .get(&id)
                .cloned()
                .ok_or_else(|| ProtocolError::new(format!("node {id} does not exist")))?;
            self.overlay.insert(id, Some(node));
        }
        self.overlay
            .get_mut(&id)
            .and_then(Option::as_mut)
            .ok_or_else(|| ProtocolError::new(format!("node {id} was already removed")))
    }

    fn create(
        &mut self,
        id: u32,
        tag: NodeTag,
        text: Arc<str>,
    ) -> std::result::Result<(), ProtocolError> {
        if id == ROOT_NODE {
            return Err(ProtocolError::new("node id 0 is reserved for the root"));
        }
        if self.node(id).is_some() {
            return Err(ProtocolError::new(format!("node {id} already exists")));
        }
        let mut node = NativeNode::new(tag);
        node.text = text;
        self.overlay.insert(id, Some(node));
        Ok(())
    }

    fn set_property(
        &mut self,
        id: u32,
        key: u16,
        value: Option<PropertyValue>,
    ) -> std::result::Result<(), ProtocolError> {
        if !(1..=property::LAST).contains(&key) {
            return Err(ProtocolError::new(format!("unknown property code {key}")));
        }
        self.edit(id)?.set_property(key, value);
        Ok(())
    }

    fn replace_text(&mut self, id: u32, text: Arc<str>) -> std::result::Result<(), ProtocolError> {
        let node = self.edit(id)?;
        if node.tag != NodeTag::Text {
            return Err(ProtocolError::new(format!("node {id} is not a text node")));
        }
        node.text = text;
        Ok(())
    }

    fn insert(
        &mut self,
        parent: u32,
        child: u32,
        before: Option<u32>,
    ) -> std::result::Result<(), ProtocolError> {
        if parent == child {
            return Err(ProtocolError::new("a node cannot contain itself"));
        }
        if self.node(parent).is_none() || self.node(child).is_none() {
            return Err(ProtocolError::new(format!(
                "cannot insert missing node {child} into {parent}"
            )));
        }
        if before == Some(child) && self.node(child).and_then(|node| node.parent) == Some(parent) {
            return Ok(());
        }
        if let Some(anchor) = before
            && self.node(anchor).and_then(|node| node.parent) != Some(parent)
        {
            return Err(ProtocolError::new(format!(
                "anchor {anchor} is not a child of {parent}"
            )));
        }

        let mut ancestor = Some(parent);
        for _ in 0..MAX_TREE_DEPTH {
            let Some(id) = ancestor else {
                break;
            };
            if id == child {
                return Err(ProtocolError::new("insertion would create a cycle"));
            }
            ancestor = self.node(id).and_then(|node| node.parent);
        }
        if ancestor.is_some() {
            return Err(ProtocolError::new(format!(
                "tree depth exceeds {MAX_TREE_DEPTH}"
            )));
        }

        if let Some(previous_parent) = self.node(child).and_then(|node| node.parent) {
            self.edit(previous_parent)?
                .children
                .retain(|candidate| *candidate != child);
        }
        let index = before
            .and_then(|anchor| {
                self.node(parent)?
                    .children
                    .iter()
                    .position(|candidate| *candidate == anchor)
            })
            .unwrap_or_else(|| self.node(parent).map_or(0, |node| node.children.len()));
        self.edit(parent)?.children.insert(index, child);
        self.edit(child)?.parent = Some(parent);
        Ok(())
    }

    fn remove(&mut self, parent: u32, child: u32) -> std::result::Result<(), ProtocolError> {
        if child == ROOT_NODE {
            return Err(ProtocolError::new("the root node cannot be removed"));
        }
        if self.node(child).and_then(|node| node.parent) != Some(parent) {
            return Err(ProtocolError::new(format!(
                "node {child} is not a child of {parent}"
            )));
        }
        self.edit(parent)?
            .children
            .retain(|candidate| *candidate != child);

        let mut pending = vec![child];
        let mut removed = 0_usize;
        while let Some(id) = pending.pop() {
            removed += 1;
            if removed > MAX_NODES {
                return Err(ProtocolError::new("removed subtree exceeds the node limit"));
            }
            let children = self
                .node(id)
                .map(|node| node.children.clone())
                .unwrap_or_default();
            pending.extend(children);
            self.overlay.insert(id, None);
        }
        Ok(())
    }

    fn finish(self) -> std::result::Result<HashMap<u32, Option<NativeNode>>, ProtocolError> {
        let removed = self.overlay.values().filter(|node| node.is_none()).count();
        let inserted = self
            .overlay
            .iter()
            .filter(|(id, node)| node.is_some() && !self.base.nodes.contains_key(id))
            .count();
        let final_len = self
            .base
            .nodes
            .len()
            .saturating_sub(removed)
            .saturating_add(inserted);
        if final_len > MAX_NODES + 1 {
            return Err(ProtocolError::new(format!(
                "tree cannot contain more than {MAX_NODES} application nodes"
            )));
        }
        Ok(self.overlay)
    }
}

fn commit_overlay(tree: &mut NativeTree, overlay: HashMap<u32, Option<NativeNode>>) -> u32 {
    for (id, node) in overlay {
        match node {
            Some(node) => {
                tree.nodes.insert(id, node);
            }
            None => {
                tree.nodes.remove(&id);
            }
        }
    }
    tree.revision = tree.revision.wrapping_add(1).max(1);
    tree.revision
}

fn apply_mutations(
    tree: &mut NativeTree,
    mutations: Vec<Mutation>,
) -> std::result::Result<u32, ProtocolError> {
    let mut transaction = TreeTransaction::new(tree);
    for mutation in mutations {
        match mutation {
            Mutation::Create { id, tag, text } => transaction.create(id, tag, text)?,
            Mutation::SetProperty { id, key, value } => transaction.set_property(id, key, value)?,
            Mutation::ReplaceText { id, text } => transaction.replace_text(id, text)?,
            Mutation::Insert {
                parent,
                child,
                before,
            } => transaction.insert(parent, child, before)?,
            Mutation::Remove { parent, child } => transaction.remove(parent, child)?,
            Mutation::Cleanup { parent, children } => {
                for child in children {
                    if transaction.node(child).is_some() {
                        transaction.remove(parent, child)?;
                    }
                }
            }
        }
    }
    let overlay = transaction.finish()?;
    Ok(commit_overlay(tree, overlay))
}

#[derive(Debug)]
struct ProtocolError(String);

impl ProtocolError {
    fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> std::result::Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| ProtocolError::new("truncated mutation batch"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> std::result::Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> std::result::Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> std::result::Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> std::result::Result<f32, ProtocolError> {
        let value = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ProtocolError::new("property numbers must be finite"))
        }
    }

    fn string(&mut self) -> std::result::Result<Arc<str>, ProtocolError> {
        let length = self.u32()? as usize;
        if length > MAX_STRING_BYTES {
            return Err(ProtocolError::new(format!(
                "strings cannot exceed {MAX_STRING_BYTES} bytes"
            )));
        }
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| ProtocolError::new("strings must contain valid UTF-8"))?;
        Ok(Arc::from(value))
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn decode_batch(bytes: &[u8]) -> std::result::Result<Vec<Mutation>, ProtocolError> {
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(ProtocolError::new(format!(
            "one mutation batch cannot exceed {MAX_BATCH_BYTES} bytes"
        )));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != PROTOCOL_MAGIC {
        return Err(ProtocolError::new("invalid mutation batch magic"));
    }
    let version = reader.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::new(format!(
            "unsupported mutation protocol version {version}"
        )));
    }
    let count = reader.u32()? as usize;
    if count > MAX_MUTATIONS {
        return Err(ProtocolError::new(format!(
            "one batch cannot contain more than {MAX_MUTATIONS} mutations"
        )));
    }
    let mut mutations = Vec::with_capacity(count.min(4_096));
    for _ in 0..count {
        let mutation = match reader.u8()? {
            1 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::decode(reader.u8()?)?,
                text: Arc::from(""),
            },
            2 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::Text,
                text: reader.string()?,
            },
            3 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::Sentinel,
                text: Arc::from(""),
            },
            4 => {
                let id = reader.u32()?;
                let key = reader.u16()?;
                let value = match reader.u8()? {
                    0 => None,
                    1 => Some(PropertyValue::Bool(match reader.u8()? {
                        0 => false,
                        1 => true,
                        value => {
                            return Err(ProtocolError::new(format!(
                                "invalid boolean byte {value}"
                            )));
                        }
                    })),
                    2 => Some(PropertyValue::Number(reader.f32()?)),
                    3 => Some(PropertyValue::Color(reader.u32()?)),
                    4 => Some(PropertyValue::String(reader.string()?)),
                    value => {
                        return Err(ProtocolError::new(format!(
                            "unknown property value tag {value}"
                        )));
                    }
                };
                Mutation::SetProperty { id, key, value }
            }
            5 => Mutation::ReplaceText {
                id: reader.u32()?,
                text: reader.string()?,
            },
            6 => {
                let parent = reader.u32()?;
                let child = reader.u32()?;
                let anchor = reader.u32()?;
                Mutation::Insert {
                    parent,
                    child,
                    before: (anchor != NO_ANCHOR).then_some(anchor),
                }
            }
            7 => Mutation::Remove {
                parent: reader.u32()?,
                child: reader.u32()?,
            },
            8 => {
                let parent = reader.u32()?;
                let count = reader.u32()? as usize;
                if count > MAX_MUTATIONS {
                    return Err(ProtocolError::new(
                        "cleanup list exceeds the mutation limit",
                    ));
                }
                let mut children = Vec::with_capacity(count.min(4_096));
                for _ in 0..count {
                    children.push(reader.u32()?);
                }
                Mutation::Cleanup { parent, children }
            }
            opcode => {
                return Err(ProtocolError::new(format!(
                    "unknown mutation opcode {opcode}"
                )));
            }
        };
        mutations.push(mutation);
    }
    if !reader.finished() {
        return Err(ProtocolError::new("mutation batch has trailing bytes"));
    }
    Ok(mutations)
}

#[derive(Clone, Debug)]
struct QueuedEvent {
    kind: &'static str,
    window: u32,
    target: u32,
    value: Option<Arc<str>>,
}

type EventQueue = Rc<RefCell<VecDeque<QueuedEvent>>>;

struct NativeView {
    window: u32,
    tree: Rc<RefCell<NativeTree>>,
    events: EventQueue,
    markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
}

impl View for NativeView {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let tree = self.tree.borrow();
        let mut markdown = self.markdown.borrow_mut();
        markdown.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Markdown)
        });
        let mut root = div()
            .id(ElementId::new(ROOT_ELEMENT_ID))
            .size_full()
            .min_w(0.0)
            .min_h(0.0);
        if let Some(node) = tree.nodes.get(&ROOT_NODE) {
            root = root.children(node.children.iter().filter_map(|id| {
                build_element(*id, self.window, &tree, &self.events, &mut markdown, cx, 0)
            }));
        }
        root
    }
}

fn build_element(
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    markdown: &mut HashMap<u32, Markdown>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Option<Element> {
    if depth >= MAX_TREE_DEPTH {
        return None;
    }
    let node = tree.nodes.get(&id)?;
    let element_id = ElementId::new(id as u64);
    let mut element = match node.tag {
        NodeTag::Root => return None,
        NodeTag::View => div(),
        NodeTag::Button => button().cursor_default(),
        NodeTag::Text => text(node.text.clone()),
        NodeTag::Sentinel => div().hidden(),
        NodeTag::Input => {
            let multiline = node.boolean(property::MULTILINE).unwrap_or(false);
            let mut input = if multiline {
                text_area(node.string(property::VALUE).unwrap_or_default())
            } else {
                text_input(node.string(property::VALUE).unwrap_or_default())
            }
            .bg(Color::TRANSPARENT)
            .border(0.0, Color::TRANSPARENT)
            .rounded(0.0);
            if let Some(placeholder) = node.string(property::PLACEHOLDER) {
                input = input.placeholder(placeholder);
            }
            if !multiline && node.boolean(property::PASSWORD).unwrap_or(false) {
                input = input.password(true);
            }
            if node.boolean(property::INPUT_LISTENER).unwrap_or(false) {
                let events = Rc::clone(events);
                // The retained input state already schedules its paint. Rebuilding here would read
                // the previous JavaScript-controlled value before Bun drains this queued event,
                // resetting every keystroke before Solid can commit the matching mutation batch.
                let listener = cx.input_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "input",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_input(listener);
            }
            if !multiline && node.boolean(property::SUBMIT_LISTENER).unwrap_or(false) {
                let events = Rc::clone(events);
                // Submit has the same controlled-state boundary as input: JavaScript must consume
                // the queued value before a render can safely read the controlled property again.
                let listener = cx.submit_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "submit",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_submit(listener);
            }
            input
        }
        NodeTag::Markdown => {
            let mut markdown_style = MarkdownStyle::default();
            markdown_style.text_color = node.color(property::COLOR);
            markdown_style.font_size = node
                .number(property::FONT_SIZE)
                .unwrap_or(markdown_style.font_size);
            markdown_style.line_height = node
                .number(property::LINE_HEIGHT)
                .unwrap_or(markdown_style.line_height);
            markdown_style.code_background = node.color(property::MARKDOWN_CODE_BACKGROUND);
            markdown_style.border_color = node.color(property::MARKDOWN_BORDER_COLOR);
            markdown_style.muted_color = node.color(property::MARKDOWN_MUTED_COLOR);
            markdown_style.link_color = node.color(property::MARKDOWN_LINK_COLOR);
            markdown_style.code_text_color = node.color(property::MARKDOWN_CODE_TEXT_COLOR);
            markdown_style.block_gap = node
                .number(property::MARKDOWN_BLOCK_GAP)
                .unwrap_or(markdown_style.block_gap);
            markdown_style.code_font_size = node
                .number(property::MARKDOWN_CODE_FONT_SIZE)
                .unwrap_or(markdown_style.code_font_size);
            let state = markdown.entry(id).or_default();
            state.set_streaming(node.boolean(property::STREAMING).unwrap_or(false));
            state.set_style(markdown_style);
            state.set_text(node.string(property::VALUE).unwrap_or_default());
            state.element(element_id)
        }
    }
    .id(element_id);

    element = apply_properties(element, node);

    if node.boolean(property::CLICK_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.listener(element_id, move |_view, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "click",
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        });
        element = element.on_click(listener);
    }
    if node.boolean(property::HOVER_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.hover_listener(element_id, move |_view, hovered, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: if *hovered { "mouseenter" } else { "mouseleave" },
                    window,
                    target: id,
                    value: None,
                },
            );
            cx.invalidate();
        });
        element = element.on_hover(listener);
    }

    if !matches!(
        node.tag,
        NodeTag::Text | NodeTag::Sentinel | NodeTag::Input | NodeTag::Markdown
    ) {
        element = element.children(node.children.iter().filter_map(|child| {
            build_element(*child, window, tree, events, markdown, cx, depth + 1)
        }));
    }
    Some(element)
}

fn enqueue_event(events: &EventQueue, event: QueuedEvent) {
    let mut events = events.borrow_mut();
    if events.len() < MAX_QUEUED_EVENTS {
        events.push_back(event);
    }
}

fn apply_properties(mut element: Element, node: &NativeNode) -> Element {
    if let Some(display) = node.string(property::DISPLAY) {
        element = match display {
            "none" => element.hidden(),
            "flex" => match node.string(property::FLEX_DIRECTION) {
                Some("row") => element.flex_row(),
                Some("row-reverse") => element.flex_row_reverse(),
                Some("column-reverse") => element.flex_col_reverse(),
                _ => element.flex_col(),
            },
            "grid" => element.grid(),
            _ => element.block(),
        };
    }
    if let Some(wrap) = node.string(property::FLEX_WRAP) {
        element = match wrap {
            "wrap" => element.flex_wrap(),
            "wrap-reverse" => element.flex_wrap_reverse(),
            _ => element.flex_nowrap(),
        };
    }
    if let Some(value) = node.number(property::FLEX_GROW) {
        element = element.flex_grow(value);
    }
    if let Some(value) = node.number(property::FLEX_SHRINK) {
        element = element.flex_shrink(value);
    }
    if let Some(value) = node.number(property::FLEX_BASIS) {
        element = element.flex_basis(value);
    } else if node.string(property::FLEX_BASIS) == Some("auto") {
        element = element.flex_basis_auto();
    }
    if let Some(value) = node.string(property::ALIGN_ITEMS) {
        element = match value {
            "center" => element.items_center(),
            "flex-end" | "end" => element.items_end(),
            "baseline" => element.items_baseline(),
            "stretch" => element.items_stretch(),
            _ => element.items_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_SELF) {
        element = match value {
            "center" => element.self_center(),
            "flex-end" => element.self_flex_end(),
            "end" => element.self_end(),
            "baseline" => element.self_baseline(),
            "stretch" => element.self_stretch(),
            "start" => element.self_start(),
            _ => element.self_flex_start(),
        };
    }
    if let Some(value) = node.string(property::JUSTIFY_CONTENT) {
        element = match value {
            "center" => element.justify_center(),
            "flex-end" | "end" => element.justify_end(),
            "space-between" => element.justify_between(),
            "space-around" => element.justify_around(),
            "space-evenly" => element.justify_evenly(),
            _ => element.justify_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_CONTENT) {
        element = match value {
            "center" => element.content_center(),
            "flex-end" | "end" => element.content_end(),
            "space-between" => element.content_between(),
            "space-around" => element.content_around(),
            "space-evenly" => element.content_evenly(),
            "stretch" => element.content_stretch(),
            "normal" => element.content_normal(),
            _ => element.content_start(),
        };
    }
    if let Some(value) = node.number(property::GAP) {
        element = element.gap(value);
    }
    if let Some(value) = node.number(property::COLUMN_GAP) {
        element = element.gap_x(value);
    }
    if let Some(value) = node.number(property::ROW_GAP) {
        element = element.gap_y(value);
    }

    element = apply_dimension(element, node, property::WIDTH, DimensionKind::Width);
    element = apply_dimension(element, node, property::HEIGHT, DimensionKind::Height);
    if let Some(value) = node.number(property::MIN_WIDTH) {
        element = element.min_w(value);
    }
    if let Some(value) = node.number(property::MIN_HEIGHT) {
        element = element.min_h(value);
    }
    if let Some(value) = node.number(property::MAX_WIDTH) {
        element = element.max_w(value);
    }
    if let Some(value) = node.number(property::MAX_HEIGHT) {
        element = element.max_h(value);
    }

    let padding = node.number(property::PADDING).unwrap_or(0.0);
    let padding_top = node.number(property::PADDING_TOP).unwrap_or(padding);
    let padding_right = node.number(property::PADDING_RIGHT).unwrap_or(padding);
    let padding_bottom = node.number(property::PADDING_BOTTOM).unwrap_or(padding);
    let padding_left = node.number(property::PADDING_LEFT).unwrap_or(padding);
    if [padding_top, padding_right, padding_bottom, padding_left]
        .iter()
        .any(|value| *value != 0.0)
    {
        element = element.padding(padding_top, padding_right, padding_bottom, padding_left);
    }
    let margin = node.number(property::MARGIN).unwrap_or(0.0);
    let margin_top = node.number(property::MARGIN_TOP).unwrap_or(margin);
    let margin_right = node.number(property::MARGIN_RIGHT).unwrap_or(margin);
    let margin_bottom = node.number(property::MARGIN_BOTTOM).unwrap_or(margin);
    let margin_left = node.number(property::MARGIN_LEFT).unwrap_or(margin);
    if [margin_top, margin_right, margin_bottom, margin_left]
        .iter()
        .any(|value| *value != 0.0)
    {
        element = element.margin(margin_top, margin_right, margin_bottom, margin_left);
    }

    if let Some(color) = node.color(property::BACKGROUND_COLOR) {
        element = element.bg(color);
    }
    if let Some(color) = node.color(property::COLOR) {
        element = element.text_color(color);
    }
    if let Some(value) = node.number(property::OPACITY) {
        element = element.opacity(value);
    }
    let border_width = node.number(property::BORDER_WIDTH).unwrap_or(0.0);
    if border_width > 0.0 {
        element = element.border(
            border_width,
            node.color(property::BORDER_COLOR)
                .unwrap_or(Color::TRANSPARENT),
        );
    }
    if let Some(value) = node.number(property::BORDER_RADIUS) {
        element = element.rounded(value);
    }
    if let Some(value) = node.number(property::FONT_SIZE) {
        element = element.text_size(value);
    }
    if let Some(value) = node.number(property::LINE_HEIGHT) {
        element = element.line_height(value);
    }
    if let Some(weight) = font_weight(node.property(property::FONT_WEIGHT)) {
        element = element.font_weight(weight);
    }
    if let Some(value) = node.string(property::TEXT_ALIGN) {
        element = element.text_align(match value {
            "center" => TextAlign::Center,
            "right" | "end" => TextAlign::Right,
            "justify" => TextAlign::Justify,
            _ => TextAlign::Left,
        });
    }
    if let Some(value) = node.string(property::WHITE_SPACE) {
        element = match value {
            "nowrap" => element.whitespace_nowrap(),
            _ => element.whitespace_normal(),
        };
    }
    if node.string(property::TEXT_OVERFLOW) == Some("ellipsis") {
        element = element.text_ellipsis();
    }
    if let Some(value) = node.number(property::LINE_CLAMP) {
        element = element.line_clamp(value.max(1.0) as usize);
    }
    if [
        node.string(property::OVERFLOW),
        node.string(property::OVERFLOW_X),
        node.string(property::OVERFLOW_Y),
    ]
    .into_iter()
    .flatten()
    .any(|value| value == "hidden")
    {
        element = element.overflow_hidden();
    }
    if node
        .string(property::OVERFLOW_Y)
        .is_some_and(|value| matches!(value, "auto" | "scroll"))
        || node
            .string(property::OVERFLOW)
            .is_some_and(|value| matches!(value, "auto" | "scroll"))
    {
        element = element.overflow_y_scroll();
    }
    if let Some(value) = node.number(property::SCROLL_TO_END_REVISION) {
        element = element.scroll_to_end(value.max(0.0) as u64);
    }
    if let Some(value) = node.string(property::CURSOR) {
        element = element.cursor(cursor(value));
    }
    if let Some(value) = node.string(property::APP_REGION) {
        element = element.app_region(if value == "drag" {
            AppRegion::Drag
        } else {
            AppRegion::NoDrag
        });
    }
    if let Some(value) = node.boolean(property::DISABLED) {
        element = element.disabled(value);
    }
    if let Some(value) = node.string(property::ACCESSIBILITY_LABEL) {
        element = element.accessibility_label(value.to_owned());
    }
    if let Some(value) = node.string(property::ROLE).and_then(accessibility_role) {
        element = element.accessibility_role(value);
    }
    if let Some(value) = node.number(property::TAB_INDEX) {
        element = element.tab_index(value.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
    }
    if let Some(value) = node.string(property::POSITION) {
        element = if value == "absolute" {
            element.absolute()
        } else {
            element.relative()
        };
    }
    if let Some(value) = node.number(property::TOP) {
        element = element.top(value);
    }
    if let Some(value) = node.number(property::RIGHT) {
        element = element.right(value);
    }
    if let Some(value) = node.number(property::BOTTOM) {
        element = element.bottom(value);
    }
    if let Some(value) = node.number(property::LEFT) {
        element = element.left(value);
    }
    if let Some(value) = node.string(property::USER_SELECT) {
        element = match value {
            "none" => element.user_select_none(),
            "text" => element.user_select_text(),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::VISIBILITY) {
        element = if value == "hidden" {
            element.invisible()
        } else {
            element.visible()
        };
    }
    if let Some(value) = node.number(property::ASPECT_RATIO) {
        element = element.aspect_ratio(value);
    }
    element
}

enum DimensionKind {
    Width,
    Height,
}

fn apply_dimension(
    element: Element,
    node: &NativeNode,
    property: u16,
    kind: DimensionKind,
) -> Element {
    match node.property(property) {
        Some(PropertyValue::Number(value)) => match kind {
            DimensionKind::Width => element.w(*value),
            DimensionKind::Height => element.h(*value),
        },
        Some(PropertyValue::String(value)) if value.as_ref() == "100%" => match kind {
            DimensionKind::Width => element.w_full(),
            DimensionKind::Height => element.h_full(),
        },
        _ => element,
    }
}

fn unpack_color(value: u32) -> Color {
    Color::rgba8(
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}

fn font_weight(value: Option<&PropertyValue>) -> Option<FontWeight> {
    let weight = match value {
        Some(PropertyValue::Number(value)) => *value,
        Some(PropertyValue::String(value)) => match value.as_ref() {
            "thin" => 100.0,
            "extralight" | "extra-light" => 200.0,
            "light" => 300.0,
            "medium" => 500.0,
            "semibold" | "semi-bold" => 600.0,
            "bold" => 700.0,
            "extrabold" | "extra-bold" => 800.0,
            "black" => 900.0,
            _ => 400.0,
        },
        _ => return None,
    };
    Some(if weight < 150.0 {
        FontWeight::THIN
    } else if weight < 250.0 {
        FontWeight::EXTRA_LIGHT
    } else if weight < 350.0 {
        FontWeight::LIGHT
    } else if weight < 450.0 {
        FontWeight::NORMAL
    } else if weight < 550.0 {
        FontWeight::MEDIUM
    } else if weight < 650.0 {
        FontWeight::SEMIBOLD
    } else if weight < 750.0 {
        FontWeight::BOLD
    } else if weight < 850.0 {
        FontWeight::EXTRA_BOLD
    } else {
        FontWeight::BLACK
    })
}

fn cursor(value: &str) -> CursorStyle {
    match value {
        "text" => CursorStyle::IBeam,
        "pointer" => CursorStyle::PointingHand,
        "crosshair" => CursorStyle::Crosshair,
        "grab" => CursorStyle::OpenHand,
        "grabbing" => CursorStyle::ClosedHand,
        "not-allowed" | "no-drop" => CursorStyle::OperationNotAllowed,
        "copy" => CursorStyle::DragCopy,
        "alias" => CursorStyle::DragLink,
        "context-menu" => CursorStyle::ContextualMenu,
        "ew-resize" => CursorStyle::ResizeLeftRight,
        "ns-resize" => CursorStyle::ResizeUpDown,
        "nwse-resize" => CursorStyle::ResizeUpLeftDownRight,
        "nesw-resize" => CursorStyle::ResizeUpRightDownLeft,
        "col-resize" => CursorStyle::ResizeColumn,
        "row-resize" => CursorStyle::ResizeRow,
        "n-resize" => CursorStyle::ResizeUp,
        "e-resize" => CursorStyle::ResizeRight,
        "s-resize" => CursorStyle::ResizeDown,
        "w-resize" => CursorStyle::ResizeLeft,
        "vertical-text" => CursorStyle::IBeamCursorForVerticalLayout,
        _ => CursorStyle::Arrow,
    }
}

fn accessibility_role(value: &str) -> Option<AccessibilityRole> {
    Some(match value {
        "button" => AccessibilityRole::Button,
        "link" => AccessibilityRole::Link,
        "img" | "image" => AccessibilityRole::Image,
        "list" => AccessibilityRole::List,
        "listitem" => AccessibilityRole::ListItem,
        "heading" => AccessibilityRole::Heading,
        "checkbox" => AccessibilityRole::CheckBox,
        "radio" => AccessibilityRole::RadioButton,
        "radiogroup" => AccessibilityRole::RadioGroup,
        "switch" => AccessibilityRole::Switch,
        "dialog" => AccessibilityRole::Dialog,
        "alertdialog" => AccessibilityRole::AlertDialog,
        "menu" => AccessibilityRole::Menu,
        "menuitem" => AccessibilityRole::MenuItem,
        "separator" => AccessibilityRole::Separator,
        "group" => AccessibilityRole::Group,
        "region" => AccessibilityRole::Region,
        "listbox" => AccessibilityRole::ListBox,
        "option" => AccessibilityRole::ListBoxOption,
        "combobox" => AccessibilityRole::ComboBox,
        "table" => AccessibilityRole::Table,
        "tree" => AccessibilityRole::Tree,
        "grid" => AccessibilityRole::Grid,
        "row" => AccessibilityRole::Row,
        "columnheader" => AccessibilityRole::ColumnHeader,
        "rowheader" => AccessibilityRole::RowHeader,
        "gridcell" => AccessibilityRole::GridCell,
        "treeitem" => AccessibilityRole::TreeItem,
        "tab" => AccessibilityRole::Tab,
        "tablist" => AccessibilityRole::TabList,
        "tabpanel" => AccessibilityRole::TabPanel,
        "tooltip" => AccessibilityRole::Tooltip,
        "form" => AccessibilityRole::Form,
        "label" => AccessibilityRole::Label,
        _ => return None,
    })
}

struct NativeWindowRuntime {
    config: AppConfig,
    tree: Rc<RefCell<NativeTree>>,
    markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
    handle: Option<WindowHandle>,
}

impl NativeWindowRuntime {
    fn view(&self, window: u32, events: &EventQueue) -> NativeView {
        NativeView {
            window,
            tree: Rc::clone(&self.tree),
            events: Rc::clone(events),
            markdown: Rc::clone(&self.markdown),
        }
    }
}

struct NativeRuntime {
    next_window_id: u32,
    windows: HashMap<u32, NativeWindowRuntime>,
    window_order: Vec<u32>,
    events: EventQueue,
    handles: Rc<RefCell<HashMap<WindowHandle, u32>>>,
    closed_windows: Rc<RefCell<Vec<u32>>>,
    runner: Option<AppRunner>,
}

impl NativeRuntime {
    fn new() -> Self {
        Self {
            next_window_id: 1,
            windows: HashMap::new(),
            window_order: Vec::with_capacity(2),
            events: Rc::new(RefCell::new(VecDeque::with_capacity(32))),
            handles: Rc::new(RefCell::new(HashMap::with_capacity(2))),
            closed_windows: Rc::new(RefCell::new(Vec::with_capacity(2))),
            runner: None,
        }
    }

    fn create_window(&mut self, options: NativeWindowOptions) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        if self.windows.len() >= MAX_WINDOWS {
            return Err(format!(
                "an application cannot own more than {MAX_WINDOWS} windows"
            ));
        }
        let id = self.next_window_id.max(1);
        self.next_window_id = id
            .checked_add(1)
            .ok_or_else(|| "QuickGUI window id space exhausted".to_owned())?;
        let config = window_config(&options)?;
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(NativeTree::default())),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            handle: None,
        };
        if let Some(runner) = &mut self.runner {
            let handle = runner
                .open_window(window.view(id, &self.events), window.config.clone())
                .map_err(|error| error.to_string())?;
            self.handles.borrow_mut().insert(handle, id);
            window.handle = Some(handle);
        }
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    fn create_anchored_window(
        &mut self,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
    ) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        if self.windows.len() >= MAX_WINDOWS {
            return Err(format!(
                "an application cannot own more than {MAX_WINDOWS} windows"
            ));
        }
        let parent_handle = {
            let parent_window = self
                .windows
                .get(&parent)
                .ok_or_else(|| format!("unknown QuickGUI parent window {parent}"))?;
            if !parent_window.tree.borrow().nodes.contains_key(&anchor) {
                return Err(format!(
                    "anchored popup trigger node {anchor} is not mounted in parent window {parent}"
                ));
            }
            parent_window
                .handle
                .ok_or_else(|| "an anchored popup requires a running parent window".to_owned())?
        };
        let id = self.next_window_id.max(1);
        self.next_window_id = id
            .checked_add(1)
            .ok_or_else(|| "QuickGUI window id space exhausted".to_owned())?;
        let config = anchored_window_config(&options)?;
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(NativeTree::default())),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            handle: None,
        };
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "an anchored popup requires a running application".to_owned())?;
        let handle = runner
            .open_anchored_popup(
                parent_handle,
                ElementId::new(anchor as u64),
                window.view(id, &self.events),
                window.config.clone(),
            )
            .map_err(|error| error.to_string())?;
        self.handles.borrow_mut().insert(handle, id);
        window.handle = Some(handle);
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    fn start(&mut self) -> std::result::Result<(), String> {
        if self.runner.is_some() {
            return Ok(());
        }
        let staged = self
            .window_order
            .iter()
            .filter_map(|id| {
                self.windows
                    .get(id)
                    .map(|window| (*id, window.config.clone(), window.view(*id, &self.events)))
            })
            .collect::<Vec<_>>();
        let mut staged = staged.into_iter();
        let Some((root_id, root_config, root_view)) = staged.next() else {
            return Err("createWindow must be called before starting the application".to_owned());
        };
        let handles = Rc::clone(&self.handles);
        let callback_handles = Rc::clone(&self.handles);
        let callback_events = Rc::clone(&self.events);
        let closed_windows = Rc::clone(&self.closed_windows);
        let mut runner = QuickGuiApp::new(root_view)
            .config(root_config)
            .quit_mode(QuitMode::LastWindowClosed)
            .on_window_closed(move |handle, _cx| {
                let Some(window) = callback_handles.borrow_mut().remove(&handle) else {
                    return;
                };
                enqueue_event(
                    &callback_events,
                    QueuedEvent {
                        kind: "close",
                        window,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
                let mut closed = closed_windows.borrow_mut();
                if closed.len() < MAX_WINDOWS {
                    closed.push(window);
                }
            })
            .into_runner()
            .map_err(|error| error.to_string())?;
        let mut mounted = Vec::with_capacity(self.windows.len());
        let root_handle = runner.root_window();
        handles.borrow_mut().insert(root_handle, root_id);
        mounted.push((root_id, root_handle));
        for (id, config, view) in staged {
            let handle = match runner.open_window(view, config) {
                Ok(handle) => handle,
                Err(error) => {
                    handles.borrow_mut().clear();
                    return Err(error.to_string());
                }
            };
            handles.borrow_mut().insert(handle, id);
            mounted.push((id, handle));
        }
        for (id, handle) in mounted {
            if let Some(window) = self.windows.get_mut(&id) {
                window.handle = Some(handle);
            }
        }
        self.runner = Some(runner);
        Ok(())
    }

    fn close_window(&mut self, window: u32) -> bool {
        self.sync_closed_windows();
        let Some(handle) = self.windows.get(&window).and_then(|window| window.handle) else {
            if self.windows.remove(&window).is_none() {
                return false;
            }
            self.window_order.retain(|id| *id != window);
            enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "close",
                    window,
                    target: ROOT_NODE,
                    value: None,
                },
            );
            return true;
        };
        let Some(runner) = &mut self.runner else {
            return false;
        };
        if !runner.close_window(handle) {
            return false;
        }
        self.handles.borrow_mut().remove(&handle);
        self.windows.remove(&window);
        self.window_order.retain(|id| *id != window);
        enqueue_event(
            &self.events,
            QueuedEvent {
                kind: "close",
                window,
                target: ROOT_NODE,
                value: None,
            },
        );
        true
    }

    fn apply_batch(
        &mut self,
        window: u32,
        batch: &[u8],
    ) -> std::result::Result<u32, String> {
        let mutations = decode_batch(batch).map_err(|error| error.to_string())?;
        self.sync_closed_windows();
        let native_window = self
            .windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?;
        let revision = apply_mutations(&mut native_window.tree.borrow_mut(), mutations)
            .map_err(|error| error.to_string())?;
        if let (Some(runner), Some(handle)) = (&mut self.runner, native_window.handle) {
            runner.invalidate_window(handle);
        }
        Ok(revision)
    }

    fn focus_node(&mut self, window: u32, node: u32) -> std::result::Result<bool, String> {
        self.sync_closed_windows();
        let native_window = self
            .windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?;
        if !native_window.tree.borrow().nodes.contains_key(&node) {
            return Ok(false);
        }
        let Some(handle) = native_window.handle else {
            return Ok(false);
        };
        let Some(runner) = &mut self.runner else {
            return Ok(false);
        };
        Ok(runner.focus_element(handle, ElementId::new(node as u64)))
    }

    fn sync_closed_windows(&mut self) {
        let closed = std::mem::take(&mut *self.closed_windows.borrow_mut());
        if closed.is_empty() {
            return;
        }
        for id in &closed {
            self.windows.remove(id);
        }
        self.window_order.retain(|id| !closed.contains(id));
    }

    fn drain_events(&mut self) -> Vec<NativeEvent> {
        self.events
            .borrow_mut()
            .drain(..)
            .map(|event| NativeEvent {
                kind: event.kind.to_owned(),
                window: event.window,
                target: event.target,
                value: event.value.map(|value| value.to_string()),
            })
            .collect()
    }
}

fn window_config(options: &NativeWindowOptions) -> std::result::Result<AppConfig, String> {
    let width = finite_dimension(options.width, 960.0);
    let height = finite_dimension(options.height, 640.0);
    let minimum_width = finite_dimension(options.minimum_width, 320.0);
    let minimum_height = finite_dimension(options.minimum_height, 240.0);
    let title_bar_style = match options.title_bar_style.as_deref() {
        Some("hiddenInset") | Some("hidden-inset") => TitleBarStyle::HiddenInset,
        Some("hidden") => TitleBarStyle::Hidden,
        Some("default") | None => TitleBarStyle::Default,
        Some(value) => return Err(format!("unknown titleBarStyle `{value}`")),
    };
    let background = options
        .background
        .map(unpack_color)
        .unwrap_or_else(|| Color::rgb8(18, 18, 20));
    let background_appearance = if options.blur.unwrap_or(false) {
        WindowBackgroundAppearance::Blurred
    } else if options.transparent.unwrap_or(false) {
        WindowBackgroundAppearance::Transparent
    } else {
        WindowBackgroundAppearance::Opaque
    };
    let mut config = AppConfig::new(
        options
            .title
            .clone()
            .unwrap_or_else(|| "QuickGUI".to_owned()),
    )
    .size(width, height)
    .minimum_size(minimum_width, minimum_height)
    .background(background)
    .window_background(background_appearance)
    .title_bar_style(title_bar_style);
    if let (Some(x), Some(y)) = (
        finite_number(options.traffic_light_x),
        finite_number(options.traffic_light_y),
    ) {
        config = config.traffic_light_position(x, y);
    }
    Ok(config)
}

fn anchored_window_config(
    options: &NativeWindowOptions,
) -> std::result::Result<AppConfig, String> {
    let placement = match options.popup_placement.as_deref() {
        Some("top-start") => AnchorPlacement::TopStart,
        Some("top") => AnchorPlacement::Top,
        Some("top-end") => AnchorPlacement::TopEnd,
        Some("bottom-start") | None => AnchorPlacement::BottomStart,
        Some("bottom") => AnchorPlacement::Bottom,
        Some("bottom-end") => AnchorPlacement::BottomEnd,
        Some("left-start") => AnchorPlacement::LeftStart,
        Some("left") => AnchorPlacement::Left,
        Some("left-end") => AnchorPlacement::LeftEnd,
        Some("right-start") => AnchorPlacement::RightStart,
        Some("right") => AnchorPlacement::Right,
        Some("right-end") => AnchorPlacement::RightEnd,
        Some(value) => return Err(format!("unknown popup placement `{value}`")),
    };
    let mut popover = AnchoredPopover::new(
        finite_dimension(options.width, 420.0),
        finite_dimension(options.height, 300.0),
    )
    .placement(placement)
    .gap(finite_number(options.popup_gap).unwrap_or(0.0))
    .offset(
        finite_number(options.popup_offset_x).unwrap_or(0.0),
        finite_number(options.popup_offset_y).unwrap_or(0.0),
    );
    if let Some(grab) = options.popup_grab {
        popover = popover.grab(grab);
    }
    if let Some(accepts_key_focus) = options.popup_accepts_key_focus {
        popover = popover.accepts_key_focus(accepts_key_focus);
    }
    Ok(popover.window_options(
        options
            .title
            .clone()
            .unwrap_or_else(|| "QuickGUI popover".to_owned()),
    ))
}

#[derive(Default)]
struct Registry {
    next_id: u32,
    apps: HashMap<u32, NativeRuntime>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry {
        next_id: 1,
        apps: HashMap::new(),
    });
}

fn with_app_mut<T>(
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> Result<T> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let runtime = registry
            .apps
            .get_mut(&app)
            .ok_or_else(|| Error::from_reason(format!("unknown QuickGUI app {app}")))?;
        callback(runtime).map_err(Error::from_reason)
    })
}

#[napi]
pub fn create_app() -> Result<u32> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let id = registry.next_id.max(1);
        registry.next_id = id
            .checked_add(1)
            .ok_or_else(|| Error::from_reason("QuickGUI app id space exhausted"))?;
        registry.apps.insert(id, NativeRuntime::new());
        Ok(id)
    })
}

#[napi]
pub fn create_window(app: u32, options: Option<NativeWindowOptions>) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_window(options.unwrap_or_default())
    })
}

#[napi]
pub fn create_anchored_window(
    app: u32,
    parent: u32,
    anchor: u32,
    options: Option<NativeWindowOptions>,
) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_anchored_window(parent, anchor, options.unwrap_or_default())
    })
}

#[napi]
pub fn apply_batch(app: u32, window: u32, batch: Buffer) -> Result<u32> {
    with_app_mut(app, |runtime| runtime.apply_batch(window, &batch))
}

#[napi]
pub fn close_window(app: u32, window: u32) -> Result<bool> {
    with_app_mut(app, |runtime| Ok(runtime.close_window(window)))
}

#[napi]
pub fn focus_node(app: u32, window: u32, node: u32) -> Result<bool> {
    with_app_mut(app, |runtime| runtime.focus_node(window, node))
}

#[napi]
pub fn start_app(app: u32) -> Result<()> {
    with_app_mut(app, NativeRuntime::start)
}

/// Return `-1` while running or the non-negative native exit code after termination.
#[napi]
pub fn pump_app(app: u32, timeout_ms: Option<f64>) -> Result<i32> {
    with_app_mut(app, |runtime| {
        let timeout = timeout_ms
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(16.0)
            .min(1_000.0);
        let status = runtime
            .runner
            .as_mut()
            .ok_or_else(|| "startApp must be called before pumpApp".to_owned())?
            .pump(Some(Duration::from_secs_f64(timeout / 1_000.0)))
            .map_err(|error| error.to_string())?;
        runtime.sync_closed_windows();
        match status {
            AppRunStatus::Continue => Ok(-1),
            AppRunStatus::Exited(code) => Ok(code.max(0)),
        }
    })
}

#[napi]
pub fn take_events(app: u32) -> Result<Vec<NativeEvent>> {
    with_app_mut(app, |runtime| Ok(runtime.drain_events()))
}

#[napi]
pub fn destroy_app(app: u32) -> Result<bool> {
    REGISTRY.with(|registry| Ok(registry.borrow_mut().apps.remove(&app).is_some()))
}

#[napi]
pub fn create_hosted_app() -> Result<u32> {
    let app = HOST.allocate_app().map_err(Error::from_reason)?;
    HOST.set_app(app).map_err(Error::from_reason)?;
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)?;
    Ok(app)
}

#[napi]
pub fn create_hosted_window(
    app: u32,
    options: Option<NativeWindowOptions>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateWindow {
        app,
        options: options.unwrap_or_default(),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn create_hosted_anchored_window(
    app: u32,
    parent: u32,
    anchor: u32,
    options: Option<NativeWindowOptions>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateAnchoredWindow {
        app,
        parent,
        anchor,
        options: options.unwrap_or_default(),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn apply_hosted_batch(app: u32, window: u32, batch: Buffer) -> Result<u32> {
    if batch.len() > MAX_BATCH_BYTES {
        return Err(Error::from_reason(format!(
            "mutation batch exceeds {MAX_BATCH_BYTES} bytes"
        )));
    }
    HOST.enqueue(HostCommand::ApplyBatch {
        app,
        window,
        batch: batch.to_vec(),
    })
    .map_err(Error::from_reason)?;
    Ok(0)
}

#[napi]
pub fn close_hosted_window(app: u32, window: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CloseWindow {
        app,
        window,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn focus_hosted_node(app: u32, window: u32, node: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::FocusNode {
        app,
        window,
        node,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn start_hosted_app(app: u32) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::StartApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn destroy_hosted_app(app: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::DestroyApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[doc(hidden)]
pub struct WaitForHostedEvents {
    app: u32,
}

impl Task for WaitForHostedEvents {
    type Output = HostedAppUpdate;
    type JsValue = HostedAppUpdate;

    fn compute(&mut self) -> Result<Self::Output> {
        HOST.wait_for_update(self.app).map_err(Error::from_reason)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[napi(ts_return_type = "Promise<HostedAppUpdate>")]
pub fn wait_for_hosted_events(app: u32) -> AsyncTask<WaitForHostedEvents> {
    AsyncTask::new(WaitForHostedEvents { app })
}

#[napi]
pub fn abort_app_host(message: String) {
    HOST.fail(message);
}

#[napi]
pub fn run_app_host(on_ready: Option<Function<'_, (), ()>>) -> Result<i32> {
    HOST.begin().map_err(Error::from_reason)?;
    let result = run_app_host_loop(on_ready.as_ref());
    if let Err(error) = &result {
        HOST.fail(error.clone());
    }
    HOST.finish();
    result.map_err(Error::from_reason)
}

fn run_app_host_loop(
    on_ready: Option<&Function<'_, (), ()>>,
) -> std::result::Result<i32, String> {
    let mut active_app = None;
    let mut runtime: Option<NativeRuntime> = None;
    let mut ready_reported = false;

    loop {
        let running = runtime
            .as_ref()
            .is_some_and(|runtime| runtime.runner.is_some());
        let mut commands = if running {
            HOST.take_commands()?
        } else {
            HOST.wait_for_commands()?
        };

        while let Some(command) = commands.pop_front() {
            match command {
                HostCommand::CreateApp { app, reply } => {
                    let result = if runtime.is_some() {
                        Err("a QuickGUI native host can own only one app".to_owned())
                    } else {
                        active_app = Some(app);
                        runtime = Some(NativeRuntime::new());
                        Ok(())
                    };
                    reply.complete(result);
                }
                HostCommand::CreateWindow {
                    app,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.create_window(options),
                    ));
                }
                HostCommand::CreateAnchoredWindow {
                    app,
                    parent,
                    anchor,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.create_anchored_window(parent, anchor, options),
                    ));
                }
                HostCommand::ApplyBatch { app, window, batch } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime.apply_batch(window, &batch)
                    })?;
                }
                HostCommand::CloseWindow {
                    app,
                    window,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| Ok(runtime.close_window(window)),
                    ));
                }
                HostCommand::FocusNode {
                    app,
                    window,
                    node,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.focus_node(window, node),
                    ));
                }
                HostCommand::StartApp { app, reply } => {
                    let result = with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        NativeRuntime::start,
                    );
                    if result.is_ok() {
                        let waker = runtime
                            .as_ref()
                            .and_then(|runtime| runtime.runner.as_ref())
                            .expect("a started QuickGUI host must own an AppRunner")
                            .waker();
                        HOST.set_waker(waker);
                    }
                    reply.complete(result);
                }
                HostCommand::DestroyApp { app, reply } => {
                    let result = if active_app == Some(app) && runtime.is_some() {
                        HOST.publish_exit(0);
                        Ok(true)
                    } else {
                        Ok(false)
                    };
                    reply.complete(result);
                    return Ok(0);
                }
            }
        }

        let Some(runtime) = runtime.as_mut() else {
            continue;
        };
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());

        let Some(runner) = runtime.runner.as_mut() else {
            continue;
        };
        let status = runner.pump(None).map_err(|error| error.to_string())?;
        if !ready_reported {
            if let Some(on_ready) = on_ready {
                on_ready.call(()).map_err(|error| error.to_string())?;
            }
            ready_reported = true;
        }
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());
        if let AppRunStatus::Exited(code) = status {
            let code = code.max(0);
            HOST.publish_exit(code);
            return Ok(code);
        }
    }
}

fn with_hosted_runtime<T>(
    active_app: Option<u32>,
    runtime: Option<&mut NativeRuntime>,
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> std::result::Result<T, String> {
    if active_app != Some(app) {
        return Err(format!("unknown QuickGUI hosted app {app}"));
    }
    callback(runtime.ok_or_else(|| format!("unknown QuickGUI hosted app {app}"))?)
}

#[napi]
pub fn protocol_version() -> u32 {
    PROTOCOL_VERSION as u32
}

fn finite_number(value: Option<f64>) -> Option<f32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(f32::MIN as f64, f32::MAX as f64) as f32)
}

fn finite_dimension(value: Option<f64>, fallback: f32) -> f32 {
    finite_number(value)
        .filter(|value| *value > 0.0)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BatchWriter {
        bytes: Vec<u8>,
        count: u32,
    }

    impl BatchWriter {
        fn new() -> Self {
            let mut bytes = PROTOCOL_MAGIC.to_vec();
            bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            Self { bytes, count: 0 }
        }

        fn op(&mut self, opcode: u8) {
            self.count += 1;
            self.bytes.push(opcode);
        }

        fn u32(&mut self, value: u32) {
            self.bytes.extend_from_slice(&value.to_le_bytes());
        }

        fn string(&mut self, value: &str) {
            self.u32(value.len() as u32);
            self.bytes.extend_from_slice(value.as_bytes());
        }

        fn finish(mut self) -> Vec<u8> {
            self.bytes[6..10].copy_from_slice(&self.count.to_le_bytes());
            self.bytes
        }
    }

    #[test]
    fn binary_batch_builds_and_removes_a_tree_atomically() {
        let mut writer = BatchWriter::new();
        writer.op(1);
        writer.u32(1);
        writer.bytes.push(1);
        writer.op(2);
        writer.u32(2);
        writer.string("Hello");
        writer.op(6);
        writer.u32(1);
        writer.u32(2);
        writer.u32(NO_ANCHOR);
        writer.op(6);
        writer.u32(ROOT_NODE);
        writer.u32(1);
        writer.u32(NO_ANCHOR);

        let mut tree = NativeTree::default();
        let revision = apply_mutations(&mut tree, decode_batch(&writer.finish()).unwrap()).unwrap();
        assert_eq!(revision, 1);
        assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
        assert_eq!(tree.nodes[&1].children, [2]);
        assert_eq!(tree.nodes[&2].text.as_ref(), "Hello");

        let mut remove = BatchWriter::new();
        remove.op(7);
        remove.u32(ROOT_NODE);
        remove.u32(1);
        apply_mutations(&mut tree, decode_batch(&remove.finish()).unwrap()).unwrap();
        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.nodes[&ROOT_NODE].children.is_empty());
    }

    #[test]
    fn failed_cycle_does_not_mutate_the_committed_tree() {
        let mut tree = NativeTree::default();
        let mut setup = BatchWriter::new();
        for id in [1, 2] {
            setup.op(1);
            setup.u32(id);
            setup.bytes.push(1);
        }
        setup.op(6);
        setup.u32(ROOT_NODE);
        setup.u32(1);
        setup.u32(NO_ANCHOR);
        setup.op(6);
        setup.u32(1);
        setup.u32(2);
        setup.u32(NO_ANCHOR);
        apply_mutations(&mut tree, decode_batch(&setup.finish()).unwrap()).unwrap();
        let before = tree.clone();

        let mut cycle = BatchWriter::new();
        cycle.op(6);
        cycle.u32(2);
        cycle.u32(1);
        cycle.u32(NO_ANCHOR);
        assert!(apply_mutations(&mut tree, decode_batch(&cycle.finish()).unwrap()).is_err());
        assert_eq!(tree.revision, before.revision);
        assert_eq!(
            tree.nodes[&ROOT_NODE].children,
            before.nodes[&ROOT_NODE].children
        );
        assert_eq!(tree.nodes[&1].children, before.nodes[&1].children);
    }

    #[test]
    fn malformed_batch_is_rejected_before_tree_mutation() {
        let error = decode_batch(b"not a batch").unwrap_err();
        assert!(error.to_string().contains("magic"));
    }

    #[test]
    fn queued_input_and_submit_survive_until_javascript_commits_the_controlled_value() {
        let input_id = 7;
        let mut tree = NativeTree::default();
        let mut input = NativeNode::new(NodeTag::Input);
        input.parent = Some(ROOT_NODE);
        input.set_property(property::INPUT_LISTENER, Some(PropertyValue::Bool(true)));
        input.set_property(property::SUBMIT_LISTENER, Some(PropertyValue::Bool(true)));
        tree.nodes.insert(input_id, input);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(input_id);

        let events = Rc::new(RefCell::new(VecDeque::new()));
        let view = NativeView {
            window: 3,
            tree: Rc::new(RefCell::new(tree)),
            events: Rc::clone(&events),
            markdown: Rc::new(RefCell::new(HashMap::new())),
        };
        let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
        let window = view.window_handle();

        cx.focus(window, ElementId::new(input_id as u64)).unwrap();
        cx.simulate_input(window, "hello").unwrap();

        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some("hello")
        );
        let event = events.borrow_mut().pop_front().unwrap();
        assert_eq!(event.kind, "input");
        assert_eq!(event.window, 3);
        assert_eq!(event.target, input_id);
        assert_eq!(event.value.as_deref(), Some("hello"));

        cx.simulate_keystrokes(window, "enter").unwrap();

        assert_eq!(
            cx.focused(window).unwrap(),
            Some(ElementId::new(input_id as u64))
        );
        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some("hello")
        );
        let event = events.borrow_mut().pop_front().unwrap();
        assert_eq!(event.kind, "submit");
        assert_eq!(event.window, 3);
        assert_eq!(event.target, input_id);
        assert_eq!(event.value.as_deref(), Some("hello"));
    }
}
