use std::{fs, path::Path, time::Duration};

use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{OwnedFd, OwnedObjectPath},
};

use super::{
    BatteryState, BatteryStatus, PowerAssertionKind, PowerSource, PowerState, Result, SessionState,
    ThermalState,
};
use crate::platform;

const MAX_POWER_SUPPLIES: usize = 64;
const MAX_THERMAL_ZONES: usize = 128;
const MAX_THERMAL_TRIP_POINTS: usize = 32;
const MAX_CPU_POLICIES: usize = 256;
const MAX_SYSFS_VALUE_BYTES: usize = 4 * 1_024;

pub(super) struct PlatformPowerAssertion {
    inhibitor: Option<OwnedFd>,
}

impl PlatformPowerAssertion {
    pub(super) fn acquire(kind: PowerAssertionKind, reason: &str) -> Result<Self> {
        let connection = Connection::system().map_err(platform)?;
        let manager = Proxy::new(
            &connection,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )
        .map_err(platform)?;
        let what = match kind {
            PowerAssertionKind::PreventApplicationSuspension => "sleep",
            PowerAssertionKind::PreventDisplaySleep => "idle:sleep",
        };
        let inhibitor: OwnedFd = manager
            .call("Inhibit", &(what, "QuickGUI", reason, "block"))
            .map_err(platform)?;
        Ok(Self {
            inhibitor: Some(inhibitor),
        })
    }

    pub(super) fn release(mut self) -> Result<()> {
        self.inhibitor.take();
        Ok(())
    }
}

pub(super) fn snapshot() -> Result<PowerState> {
    let (source, battery) = battery_snapshot();
    Ok(PowerState::new(
        source,
        battery,
        thermal_state(),
        low_power_mode(),
        processor_speed_limit(),
    ))
}

pub(super) fn system_idle_time() -> Result<Duration> {
    let properties = session_properties()?;
    if !properties.idle {
        return Ok(Duration::ZERO);
    }
    let now = monotonic_microseconds()?;
    Ok(Duration::from_micros(
        now.saturating_sub(properties.idle_since_monotonic),
    ))
}

pub(super) fn session_state() -> Result<SessionState> {
    let properties = session_properties()?;
    Ok(if properties.locked {
        SessionState::Locked
    } else if properties.active {
        SessionState::Active
    } else {
        SessionState::Inactive
    })
}

struct SessionProperties {
    active: bool,
    locked: bool,
    idle: bool,
    idle_since_monotonic: u64,
}

fn session_properties() -> Result<SessionProperties> {
    let connection = Connection::system().map_err(platform)?;
    let manager = Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .map_err(platform)?;
    let path: OwnedObjectPath = manager
        .call("GetSessionByPID", &(std::process::id(),))
        .map_err(platform)?;
    let session = Proxy::new(
        &connection,
        "org.freedesktop.login1",
        path,
        "org.freedesktop.login1.Session",
    )
    .map_err(platform)?;
    Ok(SessionProperties {
        active: session.get_property("Active").map_err(platform)?,
        locked: session.get_property("LockedHint").unwrap_or(false),
        idle: session.get_property("IdleHint").map_err(platform)?,
        idle_since_monotonic: session
            .get_property("IdleSinceHintMonotonic")
            .map_err(platform)?,
    })
}

fn monotonic_microseconds() -> Result<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) } != 0 {
        return Err(platform(std::io::Error::last_os_error()));
    }
    let seconds = u64::try_from(value.tv_sec).unwrap_or(0);
    let nanoseconds = u64::try_from(value.tv_nsec).unwrap_or(0);
    Ok(seconds
        .saturating_mul(1_000_000)
        .saturating_add(nanoseconds / 1_000))
}

fn battery_snapshot() -> (PowerSource, Option<BatteryState>) {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return (PowerSource::Unknown, None);
    };
    let mut batteries = 0_u32;
    let mut charge_total = 0_u32;
    let mut charge_count = 0_u32;
    let mut charging = false;
    let mut discharging = false;
    let mut full = true;
    let mut not_charging = false;
    let mut external_supply_seen = false;
    let mut external_online = false;
    for entry in entries.take(MAX_POWER_SUPPLIES).flatten() {
        let path = entry.path();
        let Some(kind) = read_small(path.join("type")) else {
            continue;
        };
        if kind.eq_ignore_ascii_case("battery") {
            batteries = batteries.saturating_add(1);
            if let Some(capacity) = read_u64(path.join("capacity")) {
                charge_total = charge_total.saturating_add(capacity.min(100) as u32);
                charge_count = charge_count.saturating_add(1);
            }
            match read_small(path.join("status"))
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "charging" => {
                    charging = true;
                    full = false;
                }
                "discharging" => {
                    discharging = true;
                    full = false;
                }
                "full" => {}
                "not charging" => {
                    not_charging = true;
                    full = false;
                }
                _ => full = false,
            }
        } else if matches!(
            kind.to_ascii_lowercase().as_str(),
            "mains" | "usb" | "usb_c" | "usb_pd" | "wireless"
        ) {
            external_supply_seen = true;
            external_online |= read_u64(path.join("online")) == Some(1);
        }
    }
    let source = if external_online {
        PowerSource::Ac
    } else if batteries > 0 {
        PowerSource::Battery
    } else if external_supply_seen {
        PowerSource::Unknown
    } else {
        PowerSource::Ac
    };
    let battery = (batteries > 0).then(|| {
        let percent = (charge_count > 0).then_some((charge_total / charge_count).min(100) as u8);
        let status = if charging {
            BatteryStatus::Charging
        } else if full {
            BatteryStatus::Full
        } else if discharging {
            BatteryStatus::Discharging
        } else if not_charging {
            BatteryStatus::NotCharging
        } else {
            BatteryStatus::Unknown
        };
        BatteryState::new(percent, status)
    });
    (source, battery)
}

fn thermal_state() -> ThermalState {
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else {
        return ThermalState::Unknown;
    };
    let mut state = ThermalState::Unknown;
    for entry in entries.take(MAX_THERMAL_ZONES).flatten() {
        let path = entry.path();
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("thermal_zone")
        {
            continue;
        }
        let Some(temperature) = read_i64(path.join("temp")) else {
            continue;
        };
        let mut critical = None;
        let mut passive = None;
        for index in 0..MAX_THERMAL_TRIP_POINTS {
            let kind = read_small(path.join(format!("trip_point_{index}_type")));
            let threshold = read_i64(path.join(format!("trip_point_{index}_temp")));
            match (kind.as_deref(), threshold) {
                (Some("critical" | "hot"), Some(threshold)) => {
                    critical = Some(critical.map_or(threshold, |old: i64| old.min(threshold)));
                }
                (Some("passive" | "active"), Some(threshold)) => {
                    passive = Some(passive.map_or(threshold, |old: i64| old.min(threshold)));
                }
                _ => {}
            }
        }
        let candidate = match critical {
            Some(threshold) if temperature >= threshold => ThermalState::Critical,
            Some(threshold) if temperature.saturating_mul(100) >= threshold.saturating_mul(90) => {
                ThermalState::Serious
            }
            Some(threshold) if temperature.saturating_mul(100) >= threshold.saturating_mul(75) => {
                ThermalState::Fair
            }
            Some(_) => {
                if passive.is_some_and(|threshold| temperature >= threshold) {
                    ThermalState::Fair
                } else {
                    ThermalState::Nominal
                }
            }
            None => ThermalState::Unknown,
        };
        if thermal_rank(candidate) > thermal_rank(state) {
            state = candidate;
        }
    }
    state
}

fn thermal_rank(state: ThermalState) -> u8 {
    match state {
        ThermalState::Unknown => 0,
        ThermalState::Nominal => 1,
        ThermalState::Fair => 2,
        ThermalState::Serious => 3,
        ThermalState::Critical => 4,
    }
}

fn low_power_mode() -> Option<bool> {
    let profile = read_small("/sys/firmware/acpi/platform_profile")?;
    Some(matches!(profile.as_str(), "low-power" | "power-saver"))
}

fn processor_speed_limit() -> Option<u8> {
    let entries = fs::read_dir("/sys/devices/system/cpu/cpufreq").ok()?;
    entries
        .take(MAX_CPU_POLICIES)
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("policy"))
        .filter_map(|entry| {
            let path = entry.path();
            let limit = read_u64(path.join("scaling_max_freq"))?;
            let maximum = read_u64(path.join("cpuinfo_max_freq"))?;
            (maximum > 0).then_some(limit.saturating_mul(100) / maximum)
        })
        .map(|percent| percent.min(100) as u8)
        .min()
}

fn read_small(path: impl AsRef<Path>) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() > MAX_SYSFS_VALUE_BYTES {
        return None;
    }
    Some(String::from_utf8(bytes).ok()?.trim().to_owned())
}

fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_small(path)?.parse().ok()
}

fn read_i64(path: impl AsRef<Path>) -> Option<i64> {
    read_small(path)?.parse().ok()
}
