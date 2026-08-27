#[cfg(not(target_os = "macos"))]
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{collections::HashSet, fmt, sync::Arc};

use thiserror::Error;
use winit::{event_loop::ActiveEventLoop, monitor::MonitorHandle};

use crate::{Point, Rect, Size};

/// Maximum active displays retained in one application snapshot.
pub const MAX_DISPLAYS: usize = 64;
/// Maximum UTF-8 bytes retained for one operating-system display name.
pub const MAX_DISPLAY_NAME_BYTES: usize = 4 * 1024;

const MAX_DISPLAY_LOGICAL_COORDINATE: f32 = 16_777_216.0;
const MAX_DISPLAY_LOGICAL_DIMENSION: f32 = 1_048_576.0;
const MIN_DISPLAY_SCALE_FACTOR: f32 = 0.25;
const MAX_DISPLAY_SCALE_FACTOR: f32 = 16.0;

/// Process-level display identity.
///
/// On macOS this is the current Core Graphics display identifier. It is suitable for selecting a
/// display during the current application run. Persist [`Display::uuid`] when a stable physical
/// display identity is required across launches.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DisplayId(u64);

impl DisplayId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable physical display identity when the platform exposes one.
///
/// QuickGUI currently supplies this on macOS. The byte representation avoids retaining native
/// Core Foundation objects and does not require a UUID allocation for reads.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DisplayUuid([u8; 16]);

impl DisplayUuid {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for DisplayUuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for DisplayUuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        write!(
            formatter,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15],
        )
    }
}

/// One immutable display description in global logical desktop coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    id: DisplayId,
    uuid: Option<DisplayUuid>,
    name: Arc<str>,
    bounds: Rect,
    visible_bounds: Rect,
    scale_factor: f32,
    refresh_rate_millihertz: Option<u32>,
    primary: bool,
}

impl Display {
    /// Build a validated display value, primarily for deterministic platform adapters and tests.
    pub fn new(
        id: DisplayId,
        name: impl Into<Arc<str>>,
        bounds: Rect,
        visible_bounds: Rect,
        scale_factor: f32,
    ) -> Result<Self, DisplayError> {
        let name = name.into();
        if name.len() > MAX_DISPLAY_NAME_BYTES || name.contains('\0') {
            return Err(DisplayError::InvalidName);
        }
        if !valid_display_bounds(bounds)
            || !valid_display_bounds(visible_bounds)
            || bounds.intersection(visible_bounds) != Some(visible_bounds)
        {
            return Err(DisplayError::InvalidBounds);
        }
        if !scale_factor.is_finite()
            || !(MIN_DISPLAY_SCALE_FACTOR..=MAX_DISPLAY_SCALE_FACTOR).contains(&scale_factor)
        {
            return Err(DisplayError::InvalidScaleFactor);
        }
        Ok(Self {
            id,
            uuid: None,
            name,
            bounds,
            visible_bounds,
            scale_factor,
            refresh_rate_millihertz: None,
            primary: false,
        })
    }

    pub const fn id(&self) -> DisplayId {
        self.id
    }

    pub const fn uuid(&self) -> Option<DisplayUuid> {
        self.uuid
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Complete display rectangle in global logical desktop coordinates.
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Work area excluding persistent native chrome such as the Dock, menu bar, or taskbar.
    pub const fn visible_bounds(&self) -> Rect {
        self.visible_bounds
    }

    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub const fn refresh_rate_millihertz(&self) -> Option<u32> {
        self.refresh_rate_millihertz
    }

    pub const fn is_primary(&self) -> bool {
        self.primary
    }

    pub fn with_uuid(mut self, uuid: DisplayUuid) -> Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn with_refresh_rate_millihertz(mut self, refresh_rate: u32) -> Self {
        self.refresh_rate_millihertz = (refresh_rate > 0).then_some(refresh_rate);
        self
    }

    /// Center and, if necessary, shrink a window rectangle into this display's visible work area.
    pub fn centered_bounds(&self, size: Size) -> Rect {
        let area = self.visible_bounds;
        let width = sane_window_dimension(size.width, area.width);
        let height = sane_window_dimension(size.height, area.height);
        Rect::new(
            area.x + (area.width - width) * 0.5,
            area.y + (area.height - height) * 0.5,
            width,
            height,
        )
    }

    /// Keep a window rectangle fully inside this display's visible work area.
    ///
    /// Oversized or non-finite dimensions are replaced by the available dimension. This helper is
    /// intentionally pure and does not move a native window by itself.
    pub fn constrain_bounds(&self, bounds: Rect) -> Rect {
        let area = self.visible_bounds;
        let width = sane_window_dimension(bounds.width, area.width);
        let height = sane_window_dimension(bounds.height, area.height);
        let x = if bounds.x.is_finite() {
            bounds.x.clamp(area.x, area.right() - width)
        } else {
            area.x
        };
        let y = if bounds.y.is_finite() {
            bounds.y.clamp(area.y, area.bottom() - height)
        } else {
            area.y
        };
        Rect::new(x, y, width, height)
    }
}

/// A bounded, cheaply cloned snapshot of all active displays.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Displays {
    displays: Arc<[Display]>,
    primary: Option<DisplayId>,
}

impl Displays {
    /// Build a deterministic display snapshot.
    pub fn new(
        mut displays: Vec<Display>,
        primary: Option<DisplayId>,
    ) -> Result<Self, DisplayError> {
        if displays.len() > MAX_DISPLAYS {
            return Err(DisplayError::TooManyDisplays);
        }
        let mut ids = HashSet::with_capacity(displays.len());
        if displays.iter().any(|display| !ids.insert(display.id)) {
            return Err(DisplayError::DuplicateId);
        }
        if primary.is_some_and(|primary| !ids.contains(&primary)) {
            return Err(DisplayError::UnknownPrimary);
        }
        for display in &mut displays {
            display.primary = Some(display.id) == primary;
        }
        displays.sort_unstable_by_key(|display| display.id);
        Ok(Self {
            displays: displays.into(),
            primary,
        })
    }

    pub fn all(&self) -> &[Display] {
        &self.displays
    }

    pub fn primary(&self) -> Option<&Display> {
        self.primary.and_then(|id| self.find(id))
    }

    pub const fn primary_id(&self) -> Option<DisplayId> {
        self.primary
    }

    pub fn find(&self, id: DisplayId) -> Option<&Display> {
        self.displays
            .binary_search_by_key(&id, |display| display.id)
            .ok()
            .map(|index| &self.displays[index])
    }

    pub fn is_empty(&self) -> bool {
        self.displays.is_empty()
    }

    pub fn len(&self) -> usize {
        self.displays.len()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn test_default() -> Self {
        let id = DisplayId::new(1);
        Self::new(
            vec![
                Display::new(
                    id,
                    "Test display",
                    Rect::new(0.0, 0.0, 1_440.0, 900.0),
                    Rect::new(0.0, 24.0, 1_440.0, 876.0),
                    2.0,
                )
                .expect("the built-in test display is valid"),
            ],
            Some(id),
        )
        .expect("the built-in test display snapshot is valid")
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum DisplayError {
    #[error("a display name is too large or contains a null byte")]
    InvalidName,
    #[error("display bounds must be finite, positive, and the visible bounds must be contained")]
    InvalidBounds,
    #[error("a display scale factor must be finite and between 0.25 and 16")]
    InvalidScaleFactor,
    #[error("a display snapshot cannot retain more than {MAX_DISPLAYS} displays")]
    TooManyDisplays,
    #[error("display identifiers must be unique within one snapshot")]
    DuplicateId,
    #[error("the primary display identifier must occur in the snapshot")]
    UnknownPrimary,
}

pub(crate) fn native_displays(event_loop: &ActiveEventLoop) -> Displays {
    let primary_monitor = event_loop.primary_monitor();
    let mut monitors = Vec::with_capacity(4);
    if let Some(primary) = primary_monitor.clone() {
        monitors.push(primary);
    }
    for monitor in event_loop.available_monitors() {
        if monitors.len() == MAX_DISPLAYS {
            break;
        }
        if !monitors.contains(&monitor) {
            monitors.push(monitor);
        }
    }

    let primary = primary_monitor.as_ref().map(native_display_id);
    let mut ids = HashSet::with_capacity(monitors.len());
    let mut displays = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let id = native_display_id(&monitor);
        if !ids.insert(id) {
            tracing::warn!(?id, "ignoring a display identifier collision");
            continue;
        }
        let Some(display) = display_from_monitor(&monitor, id) else {
            ids.remove(&id);
            continue;
        };
        displays.push(display);
    }
    let primary = primary
        .filter(|primary| ids.contains(primary))
        .or_else(|| displays.first().map(Display::id));
    Displays::new(displays, primary).unwrap_or_default()
}

/// Resolve one short-lived native handle only when a new window needs monitor targeting.
pub(crate) fn native_monitor(event_loop: &ActiveEventLoop, id: DisplayId) -> Option<MonitorHandle> {
    if let Some(primary) = event_loop.primary_monitor()
        && native_display_id(&primary) == id
    {
        return Some(primary);
    }
    event_loop
        .available_monitors()
        .take(MAX_DISPLAYS)
        .find(|monitor| native_display_id(monitor) == id)
}

pub(crate) fn native_display_id(monitor: &MonitorHandle) -> DisplayId {
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::MonitorHandleExtMacOS;
        DisplayId::new(u64::from(monitor.native_id()))
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::MonitorHandleExtWindows;

        let mut hasher = DefaultHasher::new();
        monitor.native_id().hash(&mut hasher);
        DisplayId::new(hasher.finish().max(1))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let mut hasher = DefaultHasher::new();
        monitor.name().hash(&mut hasher);
        let position = monitor.position();
        position.x.hash(&mut hasher);
        position.y.hash(&mut hasher);
        let size = monitor.size();
        size.width.hash(&mut hasher);
        size.height.hash(&mut hasher);
        monitor.scale_factor().to_bits().hash(&mut hasher);
        monitor.refresh_rate_millihertz().hash(&mut hasher);
        DisplayId::new(hasher.finish().max(1))
    }
}

fn display_from_monitor(monitor: &MonitorHandle, id: DisplayId) -> Option<Display> {
    let scale_factor = monitor.scale_factor() as f32;
    if !scale_factor.is_finite()
        || !(MIN_DISPLAY_SCALE_FACTOR..=MAX_DISPLAY_SCALE_FACTOR).contains(&scale_factor)
    {
        return None;
    }
    let position = monitor.position();
    let size = monitor.size();
    let bounds = Rect::new(
        position.x as f32 / scale_factor,
        position.y as f32 / scale_factor,
        size.width as f32 / scale_factor,
        size.height as f32 / scale_factor,
    );
    if !valid_display_bounds(bounds) {
        return None;
    }

    #[cfg(target_os = "macos")]
    let (name, visible_bounds, uuid) = macos_display_metadata(monitor, id, bounds);
    #[cfg(not(target_os = "macos"))]
    let (name, visible_bounds, uuid) = (
        bounded_native_name(monitor.name(), id),
        bounds,
        None::<DisplayUuid>,
    );

    let mut display = Display::new(id, name, bounds, visible_bounds, scale_factor).ok()?;
    display.uuid = uuid;
    display.refresh_rate_millihertz = monitor
        .refresh_rate_millihertz()
        .filter(|refresh_rate| *refresh_rate > 0);
    Some(display)
}

fn bounded_native_name(name: Option<String>, id: DisplayId) -> Arc<str> {
    let Some(mut name) = name.filter(|name| !name.contains('\0')) else {
        return Arc::from(format!("Display {}", id.get()));
    };
    if name.len() > MAX_DISPLAY_NAME_BYTES {
        let mut end = MAX_DISPLAY_NAME_BYTES;
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        name.truncate(end);
    }
    Arc::from(name)
}

#[cfg(target_os = "macos")]
fn macos_display_metadata(
    monitor: &MonitorHandle,
    id: DisplayId,
    bounds: Rect,
) -> (Arc<str>, Rect, Option<DisplayUuid>) {
    use objc2_app_kit::NSScreen;
    use objc2_foundation::NSUTF8StringEncoding;
    use winit::platform::macos::MonitorHandleExtMacOS;

    let mut name = None;
    let mut visible_bounds = bounds;
    if let Some(screen) = monitor
        .ns_screen()
        .and_then(|screen| unsafe { (screen as *const NSScreen).as_ref() })
    {
        let native_name = unsafe { screen.localizedName() };
        if native_name.lengthOfBytesUsingEncoding(NSUTF8StringEncoding) <= MAX_DISPLAY_NAME_BYTES {
            name = Some(native_name.to_string());
        }
        let frame = screen.frame();
        let visible = screen.visibleFrame();
        let left_inset = (visible.origin.x - frame.origin.x) as f32;
        let top_inset =
            (frame.origin.y + frame.size.height - visible.origin.y - visible.size.height) as f32;
        let proposed = Rect::new(
            bounds.x + left_inset,
            bounds.y + top_inset,
            visible.size.width as f32,
            visible.size.height as f32,
        );
        if let Some(intersection) = bounds.intersection(proposed)
            && valid_display_bounds(intersection)
        {
            visible_bounds = intersection;
        }
    }
    (
        bounded_native_name(name.or_else(|| monitor.name()), id),
        visible_bounds,
        macos_display_uuid(id),
    )
}

#[cfg(target_os = "macos")]
fn macos_display_uuid(id: DisplayId) -> Option<DisplayUuid> {
    use core_foundation::{
        base::TCFType,
        uuid::{CFUUID, CFUUIDGetUUIDBytes, CFUUIDRef},
    };

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> CFUUIDRef;
    }

    let id = u32::try_from(id.get()).ok()?;
    let uuid = unsafe { CGDisplayCreateUUIDFromDisplayID(id) };
    if uuid.is_null() {
        return None;
    }
    let uuid = unsafe { CFUUID::wrap_under_create_rule(uuid) };
    let bytes = unsafe { CFUUIDGetUUIDBytes(uuid.as_concrete_TypeRef()) };
    Some(DisplayUuid::from_bytes([
        bytes.byte0,
        bytes.byte1,
        bytes.byte2,
        bytes.byte3,
        bytes.byte4,
        bytes.byte5,
        bytes.byte6,
        bytes.byte7,
        bytes.byte8,
        bytes.byte9,
        bytes.byte10,
        bytes.byte11,
        bytes.byte12,
        bytes.byte13,
        bytes.byte14,
        bytes.byte15,
    ]))
}

fn valid_display_bounds(bounds: Rect) -> bool {
    bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.x.abs() <= MAX_DISPLAY_LOGICAL_COORDINATE
        && bounds.y.abs() <= MAX_DISPLAY_LOGICAL_COORDINATE
        && bounds.width > 0.0
        && bounds.height > 0.0
        && bounds.width <= MAX_DISPLAY_LOGICAL_DIMENSION
        && bounds.height <= MAX_DISPLAY_LOGICAL_DIMENSION
}

fn sane_window_dimension(requested: f32, available: f32) -> f32 {
    if requested.is_finite() && requested > 0.0 {
        requested.min(available)
    } else {
        available
    }
}

pub(crate) fn display_for_rect(displays: &Displays, bounds: Rect) -> Option<DisplayId> {
    let center = Point::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    displays
        .all()
        .iter()
        .find(|display| display.bounds.contains(center))
        .or_else(|| {
            displays
                .all()
                .iter()
                .max_by(|left, right| {
                    intersection_area(left.bounds, bounds)
                        .total_cmp(&intersection_area(right.bounds, bounds))
                })
                .filter(|display| intersection_area(display.bounds, bounds) > 0.0)
        })
        .map(Display::id)
        .or_else(|| displays.primary_id())
}

fn intersection_area(left: Rect, right: Rect) -> f32 {
    left.intersection(right)
        .map_or(0.0, |bounds| bounds.width * bounds.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(id: u64, x: f32) -> Display {
        Display::new(
            DisplayId::new(id),
            format!("Display {id}"),
            Rect::new(x, 0.0, 1_000.0, 800.0),
            Rect::new(x, 24.0, 1_000.0, 776.0),
            2.0,
        )
        .unwrap()
    }

    #[test]
    fn snapshots_are_sorted_and_mark_exactly_one_primary() {
        let displays = Displays::new(
            vec![display(2, 1_000.0), display(1, 0.0)],
            Some(DisplayId::new(1)),
        )
        .unwrap();
        assert_eq!(displays.all()[0].id(), DisplayId::new(1));
        assert!(displays.all()[0].is_primary());
        assert!(!displays.all()[1].is_primary());
        assert_eq!(displays.primary().map(Display::id), Some(DisplayId::new(1)));
    }

    #[test]
    fn centered_and_constrained_bounds_stay_in_the_work_area() {
        let display = display(1, 0.0);
        assert_eq!(
            display.centered_bounds(Size::new(400.0, 300.0)),
            Rect::new(300.0, 262.0, 400.0, 300.0)
        );
        assert_eq!(
            display.constrain_bounds(Rect::new(-400.0, 900.0, 1_400.0, 900.0)),
            display.visible_bounds()
        );
    }

    #[test]
    fn display_selection_prefers_the_window_center_then_intersection() {
        let displays = Displays::new(
            vec![display(1, 0.0), display(2, 1_000.0)],
            Some(DisplayId::new(1)),
        )
        .unwrap();
        assert_eq!(
            display_for_rect(&displays, Rect::new(900.0, 100.0, 400.0, 400.0)),
            Some(DisplayId::new(2))
        );
        assert_eq!(
            display_for_rect(&displays, Rect::new(4_000.0, 100.0, 400.0, 400.0)),
            Some(DisplayId::new(1))
        );
    }

    #[test]
    fn invalid_or_unbounded_snapshots_are_rejected() {
        let duplicate = Displays::new(vec![display(1, 0.0), display(1, 1_000.0)], None);
        assert_eq!(duplicate, Err(DisplayError::DuplicateId));
        let mut too_many = Vec::new();
        for id in 0..=MAX_DISPLAYS {
            too_many.push(display(id as u64 + 1, id as f32 * 1_000.0));
        }
        assert_eq!(
            Displays::new(too_many, None),
            Err(DisplayError::TooManyDisplays)
        );
    }
}
