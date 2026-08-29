use std::{ffi::c_void, ptr, sync::Arc, time::Duration};

use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef},
    base::{CFEqual, CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef},
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use objc2::rc::Retained;
use objc2_foundation::{NSActivityOptions, NSObject, NSProcessInfo, NSString};

use super::{
    BatteryState, BatteryStatus, PowerAssertionKind, PowerSource, PowerState, Result, SessionState,
    ThermalState,
};
use crate::SystemIntegrationError;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
    fn IOPSGetProvidingPowerSourceType(blob: CFTypeRef) -> CFStringRef;
}

const COMBINED_SESSION_STATE: i32 = 0;
const ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;

pub(super) struct PlatformPowerAssertion {
    activity: Option<Retained<NSObject>>,
}

impl PlatformPowerAssertion {
    pub(super) fn acquire(kind: PowerAssertionKind, reason: &str) -> Result<Self> {
        let options = match kind {
            PowerAssertionKind::PreventApplicationSuspension => {
                NSActivityOptions::NSActivityIdleSystemSleepDisabled
            }
            PowerAssertionKind::PreventDisplaySleep => {
                NSActivityOptions::NSActivityIdleSystemSleepDisabled
                    | NSActivityOptions::NSActivityIdleDisplaySleepDisabled
            }
        };
        let process = NSProcessInfo::processInfo();
        let reason = NSString::from_str(reason);
        let activity = unsafe { process.beginActivityWithOptions_reason(options, &reason) };
        Ok(Self {
            activity: Some(activity),
        })
    }

    pub(super) fn release(mut self) -> Result<()> {
        if let Some(activity) = self.activity.take() {
            unsafe {
                NSProcessInfo::processInfo().endActivity(&activity);
            }
        }
        Ok(())
    }
}

pub(super) fn snapshot() -> Result<PowerState> {
    let (source, battery) = battery_snapshot();
    let process = NSProcessInfo::processInfo();
    let thermal_state = match unsafe { process.thermalState() } {
        objc2_foundation::NSProcessInfoThermalState::Nominal => ThermalState::Nominal,
        objc2_foundation::NSProcessInfoThermalState::Fair => ThermalState::Fair,
        objc2_foundation::NSProcessInfoThermalState::Serious => ThermalState::Serious,
        objc2_foundation::NSProcessInfoThermalState::Critical => ThermalState::Critical,
        _ => ThermalState::Unknown,
    };
    let low_power_mode = Some(unsafe { process.isLowPowerModeEnabled() });
    Ok(PowerState::new(
        source,
        battery,
        thermal_state,
        low_power_mode,
        None,
    ))
}

pub(super) fn system_idle_time() -> Result<Duration> {
    let seconds = unsafe {
        CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT_EVENT_TYPE)
    };
    Duration::try_from_secs_f64(seconds).map_err(|_| {
        SystemIntegrationError::Platform(Arc::from(
            "CoreGraphics returned an invalid system idle duration",
        ))
    })
}

pub(super) fn session_state() -> Result<SessionState> {
    let dictionary = unsafe { CGSessionCopyCurrentDictionary() };
    if dictionary.is_null() {
        return Ok(SessionState::Unknown);
    }
    let _dictionary_owner = unsafe { CFType::wrap_under_create_rule(dictionary.cast()) };
    let locked_key = CFString::new("CGSSessionScreenIsLocked");
    let locked = unsafe { dictionary_boolean(dictionary, locked_key.as_concrete_TypeRef()) };
    Ok(if locked == Some(true) {
        SessionState::Locked
    } else {
        SessionState::Active
    })
}

fn battery_snapshot() -> (PowerSource, Option<BatteryState>) {
    let ac_power_value = CFString::new("AC Power");
    let battery_power_value = CFString::new("Battery Power");
    let current_capacity_key = CFString::new("Current Capacity");
    let charging_key = CFString::new("Is Charging");
    let maximum_capacity_key = CFString::new("Max Capacity");
    let power_source_state_key = CFString::new("Power Source State");
    let blob = unsafe { IOPSCopyPowerSourcesInfo() };
    if blob.is_null() {
        return (PowerSource::Unknown, None);
    }
    let _blob_owner = unsafe { CFType::wrap_under_create_rule(blob) };
    let source = unsafe {
        let source = IOPSGetProvidingPowerSourceType(blob);
        if source.is_null() {
            PowerSource::Unknown
        } else if CFEqual(source.cast(), battery_power_value.as_CFTypeRef()) != 0 {
            PowerSource::Battery
        } else if CFEqual(source.cast(), ac_power_value.as_CFTypeRef()) != 0 {
            PowerSource::Ac
        } else {
            PowerSource::Unknown
        }
    };
    let list = unsafe { IOPSCopyPowerSourcesList(blob) };
    if list.is_null() {
        return (source, None);
    }
    let _list_owner = unsafe { CFType::wrap_under_create_rule(list.cast()) };
    let mut current = 0_i64;
    let mut maximum = 0_i64;
    let mut found = false;
    let mut charging = false;
    let mut discharging = false;
    let mut full = true;
    let count = unsafe { CFArrayGetCount(list) }.clamp(0, 64);
    for index in 0..count {
        let power_source = unsafe { CFArrayGetValueAtIndex(list, index) };
        if power_source.is_null() {
            continue;
        }
        let description = unsafe { IOPSGetPowerSourceDescription(blob, power_source.cast()) };
        if description.is_null() {
            continue;
        }
        let item_current =
            unsafe { dictionary_number(description, current_capacity_key.as_concrete_TypeRef()) };
        let item_maximum =
            unsafe { dictionary_number(description, maximum_capacity_key.as_concrete_TypeRef()) };
        if let (Some(item_current), Some(item_maximum)) = (item_current, item_maximum)
            && item_current >= 0
            && item_maximum > 0
        {
            found = true;
            current = current.saturating_add(item_current.min(item_maximum));
            maximum = maximum.saturating_add(item_maximum);
            full &= item_current >= item_maximum;
        }
        charging |= unsafe {
            dictionary_boolean(description, charging_key.as_concrete_TypeRef()).unwrap_or(false)
        };
        let item_source =
            unsafe { dictionary_value(description, power_source_state_key.as_concrete_TypeRef()) };
        discharging |= item_source.is_some_and(|value| unsafe {
            CFEqual(value.as_CFTypeRef(), battery_power_value.as_CFTypeRef()) != 0
        });
    }
    if !found || maximum <= 0 {
        return (source, None);
    }
    let charge_percent = ((current.saturating_mul(100) + maximum / 2) / maximum).clamp(0, 100);
    let status = if charging {
        BatteryStatus::Charging
    } else if full {
        BatteryStatus::Full
    } else if discharging || source == PowerSource::Battery {
        BatteryStatus::Discharging
    } else {
        BatteryStatus::NotCharging
    };
    (
        source,
        Some(BatteryState::new(Some(charge_percent as u8), status)),
    )
}

unsafe fn dictionary_value(dictionary: CFDictionaryRef, key: CFStringRef) -> Option<CFType> {
    let mut value: *const c_void = ptr::null();
    if unsafe { CFDictionaryGetValueIfPresent(dictionary, key.cast(), &mut value) } == 0
        || value.is_null()
    {
        None
    } else {
        Some(unsafe { CFType::wrap_under_get_rule(value.cast()) })
    }
}

unsafe fn dictionary_number(dictionary: CFDictionaryRef, key: CFStringRef) -> Option<i64> {
    unsafe { dictionary_value(dictionary, key) }
        .and_then(|value| value.downcast::<CFNumber>())
        .and_then(|number| number.to_i64())
}

unsafe fn dictionary_boolean(dictionary: CFDictionaryRef, key: CFStringRef) -> Option<bool> {
    unsafe { dictionary_value(dictionary, key) }
        .and_then(|value| value.downcast::<CFBoolean>())
        .map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_snapshot_and_idle_queries_are_bounded() {
        let snapshot = snapshot().unwrap();
        assert!(
            snapshot
                .battery()
                .and_then(BatteryState::charge_percent)
                .is_none_or(|percent| percent <= 100)
        );
        assert!(snapshot.low_power_mode().is_some());
        system_idle_time().unwrap();
        assert!(matches!(
            session_state().unwrap(),
            SessionState::Active | SessionState::Locked | SessionState::Unknown
        ));
    }

    #[test]
    fn native_assertion_has_an_exact_release_lifecycle() {
        let mut assertion = super::super::PowerAssertion::acquire(
            PowerAssertionKind::PreventApplicationSuspension,
            "QuickGUI power assertion test",
        )
        .unwrap();
        assert!(assertion.is_active());
        assert!(assertion.release().unwrap());
        assert!(!assertion.is_active());
        assert!(!assertion.release().unwrap());
    }
}
