use std::{fmt, sync::Arc, time::Duration};

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crate::SystemIntegrationError;
use crate::{Result, invalid};

#[cfg(target_os = "linux")]
#[path = "power/linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "power/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "power/windows.rs"]
mod platform;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;

    pub(super) struct PlatformPowerAssertion;

    impl PlatformPowerAssertion {
        pub(super) fn acquire(_kind: PowerAssertionKind, _reason: &str) -> Result<Self> {
            Err(SystemIntegrationError::Unsupported)
        }

        pub(super) fn release(self) -> Result<()> {
            Ok(())
        }
    }

    pub(super) fn snapshot() -> Result<PowerState> {
        Err(SystemIntegrationError::Unsupported)
    }

    pub(super) fn system_idle_time() -> Result<Duration> {
        Err(SystemIntegrationError::Unsupported)
    }

    pub(super) fn session_state() -> Result<SessionState> {
        Ok(SessionState::Unknown)
    }
}

/// Maximum UTF-8 bytes retained for a native power-assertion reason.
pub const MAX_POWER_ASSERTION_REASON_BYTES: usize = 1_024;
/// Largest threshold accepted by [`PowerMonitor::system_idle_state`].
pub const MAX_IDLE_THRESHOLD: Duration = Duration::from_secs(366 * 24 * 60 * 60);

/// The native sleep behavior prevented while a [`PowerAssertion`] is alive.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PowerAssertionKind {
    /// Keep the application and system awake while still allowing the display to sleep.
    PreventApplicationSuspension,
    /// Keep both the system and display awake.
    PreventDisplaySleep,
}

/// A native, reference-counted request that prevents one class of automatic sleep.
///
/// The assertion is released when this value is dropped or when [`Self::release`] is called.
/// Keep the value alive for exactly as long as the work requiring the assertion. Assertions are
/// process resources and do not schedule a timer or polling task.
pub struct PowerAssertion {
    kind: PowerAssertionKind,
    reason: Arc<str>,
    inner: Option<platform::PlatformPowerAssertion>,
}

impl PowerAssertion {
    /// Acquire a native assertion with a bounded, human-readable diagnostic reason.
    pub fn acquire(kind: PowerAssertionKind, reason: impl Into<Arc<str>>) -> Result<Self> {
        let reason = reason.into();
        validate_reason(&reason)?;
        let inner = platform::PlatformPowerAssertion::acquire(kind, &reason)?;
        Ok(Self {
            kind,
            reason,
            inner: Some(inner),
        })
    }

    pub const fn kind(&self) -> PowerAssertionKind {
        self.kind
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub const fn is_active(&self) -> bool {
        self.inner.is_some()
    }

    /// Release the assertion now. Returns `false` when it was already released.
    pub fn release(&mut self) -> Result<bool> {
        let Some(inner) = self.inner.take() else {
            return Ok(false);
        };
        inner.release()?;
        Ok(true)
    }
}

impl fmt::Debug for PowerAssertion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PowerAssertion")
            .field("kind", &self.kind)
            .field("reason", &self.reason)
            .field("active", &self.is_active())
            .finish()
    }
}

impl Drop for PowerAssertion {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            let _ = inner.release();
        }
    }
}

/// The source currently powering the system.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PowerSource {
    Ac,
    Battery,
    Unknown,
}

/// Current charging state reported by the operating system.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

/// A bounded aggregate battery snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BatteryState {
    charge_percent: Option<u8>,
    status: BatteryStatus,
}

impl BatteryState {
    pub(crate) const fn new(charge_percent: Option<u8>, status: BatteryStatus) -> Self {
        Self {
            charge_percent,
            status,
        }
    }

    pub const fn charge_percent(self) -> Option<u8> {
        self.charge_percent
    }

    pub const fn status(self) -> BatteryStatus {
        self.status
    }
}

/// Coarse system thermal pressure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ThermalState {
    Unknown,
    Nominal,
    Fair,
    Serious,
    Critical,
}

/// Current login-session activity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SessionState {
    Active,
    Inactive,
    Locked,
    Unknown,
}

/// System idleness classified against an application-supplied threshold.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IdleState {
    Active,
    Idle,
    Locked,
    Unknown,
}

/// One immutable power and thermal snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PowerState {
    source: PowerSource,
    battery: Option<BatteryState>,
    thermal_state: ThermalState,
    low_power_mode: Option<bool>,
    cpu_speed_limit_percent: Option<u8>,
}

impl PowerState {
    pub(crate) const fn new(
        source: PowerSource,
        battery: Option<BatteryState>,
        thermal_state: ThermalState,
        low_power_mode: Option<bool>,
        cpu_speed_limit_percent: Option<u8>,
    ) -> Self {
        Self {
            source,
            battery,
            thermal_state,
            low_power_mode,
            cpu_speed_limit_percent,
        }
    }

    pub const fn source(self) -> PowerSource {
        self.source
    }

    pub const fn battery(self) -> Option<BatteryState> {
        self.battery
    }

    pub const fn thermal_state(self) -> ThermalState {
        self.thermal_state
    }

    /// Whether the operating system reports an energy-saving mode, if supported.
    pub const fn low_power_mode(self) -> Option<bool> {
        self.low_power_mode
    }

    /// Current OS-advertised CPU speed limit, where `100` means no advertised throttling.
    pub const fn cpu_speed_limit_percent(self) -> Option<u8> {
        self.cpu_speed_limit_percent
    }
}

/// Stateless entry point for native power, idle, thermal, and session queries.
pub struct PowerMonitor;

impl PowerMonitor {
    pub fn snapshot() -> Result<PowerState> {
        platform::snapshot()
    }

    pub fn is_on_battery_power() -> Result<bool> {
        Ok(matches!(Self::snapshot()?.source(), PowerSource::Battery))
    }

    pub fn current_thermal_state() -> Result<ThermalState> {
        Ok(Self::snapshot()?.thermal_state())
    }

    pub fn system_idle_time() -> Result<Duration> {
        platform::system_idle_time()
    }

    pub fn session_state() -> Result<SessionState> {
        platform::session_state()
    }

    pub fn system_idle_state(threshold: Duration) -> Result<IdleState> {
        if threshold.is_zero() || threshold > MAX_IDLE_THRESHOLD {
            return Err(invalid(format!(
                "the idle threshold must be between one nanosecond and {} seconds",
                MAX_IDLE_THRESHOLD.as_secs()
            )));
        }
        let session = Self::session_state().unwrap_or(SessionState::Unknown);
        let idle = Self::system_idle_time()?;
        Ok(classify_idle(idle, session, threshold))
    }
}

fn validate_reason(reason: &str) -> Result<()> {
    if reason.is_empty()
        || reason.len() > MAX_POWER_ASSERTION_REASON_BYTES
        || reason.contains('\0')
        || reason.chars().any(|character| character.is_control())
    {
        return Err(invalid(format!(
            "a power assertion reason must be one non-empty line of at most {MAX_POWER_ASSERTION_REASON_BYTES} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn classify_idle(idle: Duration, session: SessionState, threshold: Duration) -> IdleState {
    if session == SessionState::Locked {
        IdleState::Locked
    } else if idle >= threshold {
        IdleState::Idle
    } else if session == SessionState::Unknown && idle.is_zero() {
        IdleState::Unknown
    } else {
        IdleState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assertion_reasons_are_bounded_and_single_line() {
        assert!(validate_reason("video playback").is_ok());
        assert!(validate_reason("").is_err());
        assert!(validate_reason("line one\nline two").is_err());
        assert!(validate_reason(&"x".repeat(MAX_POWER_ASSERTION_REASON_BYTES + 1)).is_err());
    }

    #[test]
    fn idle_classification_prioritizes_a_locked_session() {
        let threshold = Duration::from_secs(60);
        assert_eq!(
            classify_idle(Duration::ZERO, SessionState::Locked, threshold),
            IdleState::Locked
        );
        assert_eq!(
            classify_idle(Duration::from_secs(90), SessionState::Active, threshold),
            IdleState::Idle
        );
        assert_eq!(
            classify_idle(Duration::from_secs(5), SessionState::Active, threshold),
            IdleState::Active
        );
        assert_eq!(
            classify_idle(Duration::ZERO, SessionState::Unknown, threshold),
            IdleState::Unknown
        );
    }

    #[test]
    fn idle_threshold_rejects_zero_and_unbounded_values_before_native_queries() {
        assert!(PowerMonitor::system_idle_state(Duration::ZERO).is_err());
        assert!(
            PowerMonitor::system_idle_state(MAX_IDLE_THRESHOLD + Duration::from_nanos(1)).is_err()
        );
    }
}
