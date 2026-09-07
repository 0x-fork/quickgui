# Power, idle, thermal, and session state

[Documentation index](README.md)

QuickGUI exposes native power services from `quickgui-system`; they do not depend on a renderer or
Go frontend. A Rust `PowerAssertion` is an RAII process resource. Dropping or explicitly releasing it
removes the corresponding native assertion:

```rust
use quickgui::{PowerAssertion, PowerAssertionKind};

let mut playback = PowerAssertion::acquire(
    PowerAssertionKind::PreventDisplaySleep,
    "full-screen video playback",
)?;

// Keep `playback` alive while the video is active.
assert!(playback.is_active());
playback.release()?;
```

`PreventApplicationSuspension` keeps the system awake while allowing display sleep;
`PreventDisplaySleep` keeps both awake. Reasons are non-empty, single-line, NUL-free, and limited
to 1,024 UTF-8 bytes. macOS uses an `NSProcessInfo` activity, Windows uses a reason-bearing power
request handle, and Linux retains a logind inhibitor file descriptor. No implementation starts a
polling task.

## Point-in-time queries

`PowerMonitor::snapshot()` returns a bounded `PowerState` containing the current `PowerSource`, an
optional aggregate `BatteryState`, coarse `ThermalState`, and optional low-power-mode and
OS-advertised CPU speed-limit values. Unsupported individual measurements remain `None` or
`Unknown`; a platform API failure is returned as `SystemIntegrationError`.

```rust
use quickgui::{PowerMonitor, PowerSource};
use std::time::Duration;

let power = PowerMonitor::snapshot()?;
if power.source() == PowerSource::Battery {
    reduce_background_work();
}

let idle = PowerMonitor::system_idle_state(Duration::from_secs(5 * 60))?;
let session = PowerMonitor::session_state()?;
```

Idle thresholds must be nonzero and at most 366 days. macOS reads the combined CoreGraphics input
session, Windows reads the native last-input tick, and Linux reads the current logind session's
monotonic idle hint. A locked session takes precedence over the elapsed threshold. These are
explicit point-in-time calls; ordinary frames and the event-loop idle path never poll them.

The platform snapshot uses IOKit plus `NSProcessInfo` on macOS, native system power and processor
policy information on Windows, and bounded power-supply, thermal-zone, CPU-policy, and platform
profile sysfs reads on Linux. Windows currently reports `ThermalState::Unknown` but may expose an
advertised CPU speed limit; unavailable Linux thermal trip points likewise remain unknown.

## Runtime transitions

Register `Application::on_power_event` for event-driven transitions.
`PowerEvent` covers suspend/resume, lock/unlock, shutdown preparation, power-source changes,
thermal pressure, low-power mode, and CPU speed-limit changes. Each backend emits only transitions
its native notification source provides. `ShutdownRequested` is advisory and deliberately has no
fake cross-platform veto: return quickly, persist essential state, and request ordinary application
exit if appropriate.

Native monitors exist only while a power callback is registered. macOS uses workspace and process
notifications; Windows uses one invisible top-level window so it receives power and shutdown
broadcasts; Linux uses bounded logind signal threads.
Runtime teardown removes observers or closes their native channels before a scheduled relaunch is
spawned.
