use std::{mem, ptr, time::Duration};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        Power::{
            CallNtPowerInformation, GetSystemPowerStatus, PROCESSOR_POWER_INFORMATION,
            PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired,
            PowerRequestSystemRequired, PowerSetRequest, ProcessorInformation, SYSTEM_POWER_STATUS,
        },
        RemoteDesktop::{
            WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK,
            WTS_SESSIONSTATE_UNLOCK, WTSFreeMemory, WTSINFOEXW, WTSQuerySessionInformationW,
            WTSSessionInfoEx,
        },
        SystemInformation::GetTickCount64,
        Threading::{POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0},
    },
    UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
};

use super::{
    BatteryState, BatteryStatus, PowerAssertionKind, PowerSource, PowerState, Result, SessionState,
    ThermalState,
};
use crate::platform;

const BATTERY_FLAG_CHARGING: u8 = 8;
const BATTERY_FLAG_NO_BATTERY: u8 = 128;
const UNKNOWN_BYTE: u8 = u8::MAX;

pub(super) struct PlatformPowerAssertion {
    handle: HANDLE,
    requests: [i32; 2],
    request_count: usize,
}

impl PlatformPowerAssertion {
    pub(super) fn acquire(kind: PowerAssertionKind, reason: &str) -> Result<Self> {
        let mut reason: Vec<u16> = reason.encode_utf16().chain([0]).collect();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: reason.as_mut_ptr(),
            },
        };
        let handle = unsafe { PowerCreateRequest(&context) };
        if handle.is_null() || handle == -1_isize as HANDLE {
            return Err(platform(std::io::Error::last_os_error()));
        }
        let (requests, request_count) = match kind {
            PowerAssertionKind::PreventApplicationSuspension => {
                ([PowerRequestSystemRequired, 0], 1)
            }
            PowerAssertionKind::PreventDisplaySleep => {
                ([PowerRequestSystemRequired, PowerRequestDisplayRequired], 2)
            }
        };
        let mut acquired = 0_usize;
        while acquired < request_count {
            if unsafe { PowerSetRequest(handle, requests[acquired]) } == 0 {
                let error = std::io::Error::last_os_error();
                while acquired > 0 {
                    acquired -= 1;
                    unsafe {
                        PowerClearRequest(handle, requests[acquired]);
                    }
                }
                unsafe {
                    CloseHandle(handle);
                }
                return Err(platform(error));
            }
            acquired += 1;
        }
        Ok(Self {
            handle,
            requests,
            request_count,
        })
    }

    pub(super) fn release(mut self) -> Result<()> {
        let mut error = None;
        while self.request_count > 0 {
            self.request_count -= 1;
            if unsafe { PowerClearRequest(self.handle, self.requests[self.request_count]) } == 0 {
                error.get_or_insert_with(std::io::Error::last_os_error);
            }
        }
        unsafe {
            CloseHandle(self.handle);
        }
        self.handle = ptr::null_mut();
        error.map_or(Ok(()), |error| Err(platform(error)))
    }
}

pub(super) fn snapshot() -> Result<PowerState> {
    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return Err(platform(std::io::Error::last_os_error()));
    }
    let source = match status.ACLineStatus {
        0 => PowerSource::Battery,
        1 => PowerSource::Ac,
        _ => PowerSource::Unknown,
    };
    let battery = if status.BatteryFlag == UNKNOWN_BYTE
        || status.BatteryFlag & BATTERY_FLAG_NO_BATTERY != 0
    {
        None
    } else {
        let percent = (status.BatteryLifePercent != UNKNOWN_BYTE)
            .then_some(status.BatteryLifePercent.min(100));
        let battery_status = if status.BatteryFlag & BATTERY_FLAG_CHARGING != 0 {
            BatteryStatus::Charging
        } else if percent == Some(100) && source == PowerSource::Ac {
            BatteryStatus::Full
        } else if source == PowerSource::Battery {
            BatteryStatus::Discharging
        } else if source == PowerSource::Ac {
            BatteryStatus::NotCharging
        } else {
            BatteryStatus::Unknown
        };
        Some(BatteryState::new(percent, battery_status))
    };
    Ok(PowerState::new(
        source,
        battery,
        ThermalState::Unknown,
        Some(status.SystemStatusFlag != 0),
        processor_speed_limit(),
    ))
}

pub(super) fn system_idle_time() -> Result<Duration> {
    let mut input = LASTINPUTINFO {
        cbSize: mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut input) } == 0 {
        return Err(platform(std::io::Error::last_os_error()));
    }
    let now = unsafe { GetTickCount64() } as u32;
    Ok(Duration::from_millis(now.wrapping_sub(input.dwTime).into()))
}

pub(super) fn session_state() -> Result<SessionState> {
    let mut buffer = ptr::null_mut();
    let mut bytes = 0_u32;
    if unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buffer,
            &mut bytes,
        )
    } == 0
    {
        return Ok(SessionState::Unknown);
    }
    let state = if bytes as usize >= mem::size_of::<WTSINFOEXW>() {
        let info = unsafe { &*buffer.cast::<WTSINFOEXW>() };
        if info.Level == 1 {
            let info = unsafe { info.Data.WTSInfoExLevel1 };
            match info.SessionFlags as u32 {
                WTS_SESSIONSTATE_LOCK => SessionState::Locked,
                WTS_SESSIONSTATE_UNLOCK => SessionState::Active,
                _ => SessionState::Unknown,
            }
        } else {
            SessionState::Unknown
        }
    } else {
        SessionState::Unknown
    };
    unsafe {
        WTSFreeMemory(buffer.cast());
    }
    Ok(state)
}

fn processor_speed_limit() -> Option<u8> {
    let count = std::thread::available_parallelism().ok()?.get().min(256);
    let mut processors = vec![PROCESSOR_POWER_INFORMATION::default(); count];
    let bytes = processors
        .len()
        .checked_mul(mem::size_of::<PROCESSOR_POWER_INFORMATION>())?;
    let bytes = u32::try_from(bytes).ok()?;
    let status = unsafe {
        CallNtPowerInformation(
            ProcessorInformation,
            ptr::null(),
            0,
            processors.as_mut_ptr().cast(),
            bytes,
        )
    };
    if status != 0 {
        return None;
    }
    processors
        .iter()
        .filter(|processor| processor.MaxMhz > 0 && processor.MhzLimit > 0)
        .map(|processor| {
            processor
                .MhzLimit
                .saturating_mul(100)
                .checked_div(processor.MaxMhz)
                .unwrap_or(100)
                .min(100) as u8
        })
        .min()
}
