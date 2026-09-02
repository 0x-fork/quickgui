//! Bounded process and system metrics: CPU time, memory footprint, thread count, and uptime.
//!
//! Every reading is a single explicit call; nothing is sampled in the background and no thread is
//! started. Fields that an operating system does not expose are `None` rather than guessed, and
//! platforms without an implementation return [`SystemIntegrationError::Unsupported`].

use std::time::Duration;

use crate::Result;

/// Maximum accepted CPU-sampler interval; longer gaps are reported without a percentage.
pub const MAX_CPU_SAMPLE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// One immutable snapshot of the current process.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProcessMetrics {
    /// Accumulated user-mode CPU time.
    pub cpu_user: Duration,
    /// Accumulated kernel-mode CPU time.
    pub cpu_system: Duration,
    /// Resident set size in bytes.
    pub resident_bytes: u64,
    /// macOS `phys_footprint`: the memory the system charges to this process. `None` elsewhere.
    pub footprint_bytes: Option<u64>,
    /// Reserved virtual address space in bytes.
    pub virtual_bytes: u64,
    /// Live threads, when the platform exposes an inexpensive count.
    pub thread_count: Option<u32>,
    /// Time since the process started.
    pub uptime: Duration,
}

impl ProcessMetrics {
    /// Read the current process's metrics.
    pub fn current() -> Result<Self> {
        platform::process_metrics()
    }

    /// Total CPU time charged to the process.
    pub fn cpu_total(&self) -> Duration {
        self.cpu_user.saturating_add(self.cpu_system)
    }
}

/// Whole-system physical memory, in bytes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SystemMemory {
    pub total_bytes: u64,
    /// Memory available to new allocations without swapping.
    pub available_bytes: u64,
    /// Completely unused memory.
    pub free_bytes: u64,
    /// `total_bytes - available_bytes`.
    pub used_bytes: u64,
}

impl SystemMemory {
    /// Read whole-system physical memory.
    pub fn current() -> Result<Self> {
        platform::system_memory()
    }
}

/// One CPU-usage reading produced by [`CpuUsageSampler::sample`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CpuUsage {
    /// Percent of one core consumed since the previous sample. `None` for the first sample.
    pub percent: Option<f64>,
    /// Wall-clock time covered by this sample.
    pub interval: Duration,
    /// CPU time consumed during `interval`.
    pub cpu_time: Duration,
    /// Total CPU time charged to the process so far.
    pub total_cpu_time: Duration,
}

/// Stateful sampler that converts cumulative CPU time into a per-interval percentage.
///
/// The sampler stores exactly one previous reading; it never allocates and never polls in the
/// background. The caller decides how often to sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct CpuUsageSampler {
    previous: Option<(std::time::Instant, Duration)>,
}

impl CpuUsageSampler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read process CPU time and report usage since the previous call.
    pub fn sample(&mut self) -> Result<CpuUsage> {
        let metrics = ProcessMetrics::current()?;
        let now = std::time::Instant::now();
        let total = metrics.cpu_total();
        let usage = match self.previous {
            Some((instant, previous_total)) => {
                let interval = now.saturating_duration_since(instant);
                let cpu_time = total.saturating_sub(previous_total);
                CpuUsage {
                    percent: cpu_usage_percent(cpu_time, interval),
                    interval,
                    cpu_time,
                    total_cpu_time: total,
                }
            }
            None => CpuUsage {
                percent: None,
                interval: Duration::ZERO,
                cpu_time: Duration::ZERO,
                total_cpu_time: total,
            },
        };
        self.previous = Some((now, total));
        Ok(usage)
    }
}

/// Percent of one core, or `None` when the interval is empty or implausibly long.
fn cpu_usage_percent(cpu_time: Duration, interval: Duration) -> Option<f64> {
    if interval.is_zero() || interval > MAX_CPU_SAMPLE_INTERVAL {
        return None;
    }
    Some((cpu_time.as_secs_f64() / interval.as_secs_f64()) * 100.0)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{ProcessMetrics, SystemMemory};
    use crate::{Result, SystemIntegrationError};
    use std::{
        sync::Arc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    pub(super) fn process_metrics() -> Result<ProcessMetrics> {
        // SAFETY: `proc_pidinfo` writes at most `size` bytes into the zeroed structure and
        // reports how many it wrote.
        let (info, written) = unsafe {
            let mut info: libc::proc_taskallinfo = std::mem::zeroed();
            let size = size_of::<libc::proc_taskallinfo>() as libc::c_int;
            let written = libc::proc_pidinfo(
                std::process::id() as libc::c_int,
                libc::PROC_PIDTASKALLINFO,
                0,
                std::ptr::from_mut(&mut info).cast(),
                size,
            );
            (info, written)
        };
        if written < size_of::<libc::proc_taskallinfo>() as libc::c_int {
            return Err(platform_error("proc_pidinfo(PROC_PIDTASKALLINFO) failed"));
        }
        let started = Duration::from_secs(info.pbsd.pbi_start_tvsec)
            + Duration::from_micros(info.pbsd.pbi_start_tvusec);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Ok(ProcessMetrics {
            cpu_user: Duration::from_nanos(info.ptinfo.pti_total_user),
            cpu_system: Duration::from_nanos(info.ptinfo.pti_total_system),
            resident_bytes: info.ptinfo.pti_resident_size,
            footprint_bytes: physical_footprint(),
            virtual_bytes: info.ptinfo.pti_virtual_size,
            thread_count: u32::try_from(info.ptinfo.pti_threadnum).ok(),
            uptime: now.saturating_sub(started),
        })
    }

    fn physical_footprint() -> Option<u64> {
        // SAFETY: `proc_pid_rusage` fills the zeroed flavor-2 structure it is handed.
        unsafe {
            let mut usage: libc::rusage_info_v2 = std::mem::zeroed();
            let status = libc::proc_pid_rusage(
                std::process::id() as libc::c_int,
                libc::RUSAGE_INFO_V2,
                std::ptr::from_mut(&mut usage).cast(),
            );
            (status == 0).then_some(usage.ri_phys_footprint)
        }
    }

    // `libc` deprecates its Mach re-exports in favour of the `mach2` crate. QuickGUI does not add
    // that dependency for one host statistics call, so the deprecated path is used deliberately.
    #[allow(deprecated)]
    pub(super) fn system_memory() -> Result<SystemMemory> {
        let total = physical_memory()?;
        // SAFETY: the call fills `statistics` with exactly `count` 32-bit words.
        let (statistics, status) = unsafe {
            let mut statistics: libc::vm_statistics64 = std::mem::zeroed();
            let mut count = libc::HOST_VM_INFO64_COUNT;
            let status = libc::host_statistics64(
                libc::mach_host_self(),
                libc::HOST_VM_INFO64,
                std::ptr::from_mut(&mut statistics).cast(),
                &raw mut count,
            );
            (statistics, status)
        };
        if status != 0 {
            return Err(platform_error("host_statistics64(HOST_VM_INFO64) failed"));
        }
        // SAFETY: `sysconf` reads a static system value.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as u64;
        let free = u64::from(statistics.free_count) * page;
        let inactive = u64::from(statistics.inactive_count) * page;
        let purgeable = u64::from(statistics.purgeable_count) * page;
        let speculative = u64::from(statistics.speculative_count) * page;
        let available = free
            .saturating_add(inactive)
            .saturating_add(purgeable)
            .saturating_sub(speculative)
            .min(total);
        Ok(SystemMemory {
            total_bytes: total,
            available_bytes: available,
            free_bytes: free.min(total),
            used_bytes: total.saturating_sub(available),
        })
    }

    fn physical_memory() -> Result<u64> {
        let mut value = 0_u64;
        let mut length = size_of::<u64>();
        // SAFETY: the name is a NUL-terminated literal and the output buffer matches `length`.
        let status = unsafe {
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                std::ptr::from_mut(&mut value).cast(),
                &raw mut length,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 || value == 0 {
            return Err(platform_error("sysctlbyname(hw.memsize) failed"));
        }
        Ok(value)
    }

    fn platform_error(message: &str) -> SystemIntegrationError {
        SystemIntegrationError::Platform(Arc::from(message))
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{ProcessMetrics, SystemMemory, parse_proc_meminfo, parse_proc_self_stat};
    use crate::{Result, SystemIntegrationError};
    use std::{sync::Arc, time::Duration};

    pub(super) fn process_metrics() -> Result<ProcessMetrics> {
        let stat = std::fs::read_to_string("/proc/self/stat").map_err(io_error)?;
        let uptime = std::fs::read_to_string("/proc/uptime").map_err(io_error)?;
        // SAFETY: `sysconf` reads a static system value.
        let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
        // SAFETY: `sysconf` reads a static system value.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as u64;
        let parsed = parse_proc_self_stat(&stat, ticks, page)
            .ok_or_else(|| platform_error("could not parse /proc/self/stat"))?;
        let system_uptime = uptime
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default();
        Ok(ProcessMetrics {
            uptime: Duration::from_secs_f64(
                (system_uptime - parsed.started_seconds).clamp(0.0, f64::from(u32::MAX)),
            ),
            ..parsed.metrics
        })
    }

    pub(super) fn system_memory() -> Result<SystemMemory> {
        let text = std::fs::read_to_string("/proc/meminfo").map_err(io_error)?;
        parse_proc_meminfo(&text).ok_or_else(|| platform_error("could not parse /proc/meminfo"))
    }

    fn io_error(error: std::io::Error) -> SystemIntegrationError {
        SystemIntegrationError::Platform(Arc::from(error.to_string()))
    }

    fn platform_error(message: &str) -> SystemIntegrationError {
        SystemIntegrationError::Platform(Arc::from(message))
    }
}

#[cfg(windows)]
mod platform {
    use super::{ProcessMetrics, SystemMemory};
    use crate::{Result, SystemIntegrationError};
    use std::{sync::Arc, time::Duration};
    use windows_sys::Win32::{
        Foundation::FILETIME,
        System::{
            ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
            Threading::{GetCurrentProcess, GetProcessTimes, GetSystemTimeAsFileTime},
        },
    };

    fn duration_from_filetime(time: FILETIME) -> Duration {
        let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        Duration::from_nanos(ticks.saturating_mul(100))
    }

    pub(super) fn process_metrics() -> Result<ProcessMetrics> {
        // SAFETY: every output structure is zeroed before the call and sized exactly.
        unsafe {
            let process = GetCurrentProcess();
            let mut creation: FILETIME = std::mem::zeroed();
            let mut exit: FILETIME = std::mem::zeroed();
            let mut kernel: FILETIME = std::mem::zeroed();
            let mut user: FILETIME = std::mem::zeroed();
            if GetProcessTimes(
                process,
                &raw mut creation,
                &raw mut exit,
                &raw mut kernel,
                &raw mut user,
            ) == 0
            {
                return Err(platform_error("GetProcessTimes failed"));
            }
            let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            if K32GetProcessMemoryInfo(process, &raw mut counters, counters.cb) == 0 {
                return Err(platform_error("GetProcessMemoryInfo failed"));
            }
            let mut now: FILETIME = std::mem::zeroed();
            GetSystemTimeAsFileTime(&raw mut now);
            Ok(ProcessMetrics {
                cpu_user: duration_from_filetime(user),
                cpu_system: duration_from_filetime(kernel),
                resident_bytes: counters.WorkingSetSize as u64,
                footprint_bytes: None,
                virtual_bytes: counters.PagefileUsage as u64,
                thread_count: None,
                uptime: duration_from_filetime(now)
                    .saturating_sub(duration_from_filetime(creation)),
            })
        }
    }

    pub(super) fn system_memory() -> Result<SystemMemory> {
        // SAFETY: the structure is zeroed and its `dwLength` set before the call.
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&raw mut status) == 0 {
                return Err(platform_error("GlobalMemoryStatusEx failed"));
            }
            Ok(SystemMemory {
                total_bytes: status.ullTotalPhys,
                available_bytes: status.ullAvailPhys,
                free_bytes: status.ullAvailPhys,
                used_bytes: status.ullTotalPhys.saturating_sub(status.ullAvailPhys),
            })
        }
    }

    fn platform_error(message: &str) -> SystemIntegrationError {
        SystemIntegrationError::Platform(Arc::from(message))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
mod platform {
    use super::{ProcessMetrics, SystemMemory};
    use crate::{Result, SystemIntegrationError};

    pub(super) fn process_metrics() -> Result<ProcessMetrics> {
        Err(SystemIntegrationError::Unsupported)
    }

    pub(super) fn system_memory() -> Result<SystemMemory> {
        Err(SystemIntegrationError::Unsupported)
    }
}

/// Parsed `/proc/self/stat` fields, kept separate so tests can feed fixture text on any host.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct ProcSelfStat {
    metrics: ProcessMetrics,
    started_seconds: f64,
}

/// Parse the fields QuickGUI needs from `/proc/self/stat`.
///
/// The executable name in field 2 may contain spaces and parentheses, so parsing resumes after
/// the final `)`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_proc_self_stat(
    text: &str,
    ticks_per_second: u64,
    page_bytes: u64,
) -> Option<ProcSelfStat> {
    let tail = text.rsplit_once(')')?.1;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    // `fields[0]` is field 3 (state); field N of the manual page is `fields[N - 3]`.
    let field = |index: usize| -> Option<u64> { fields.get(index - 3)?.parse::<u64>().ok() };
    let user = field(14)?;
    let system = field(15)?;
    let threads = field(20)?;
    let started = field(22)?;
    let virtual_bytes = field(23)?;
    let resident_pages = field(24)?;
    let ticks = ticks_per_second.max(1);
    Some(ProcSelfStat {
        metrics: ProcessMetrics {
            cpu_user: Duration::from_secs_f64(user as f64 / ticks as f64),
            cpu_system: Duration::from_secs_f64(system as f64 / ticks as f64),
            resident_bytes: resident_pages.saturating_mul(page_bytes),
            footprint_bytes: None,
            virtual_bytes,
            thread_count: u32::try_from(threads).ok(),
            uptime: Duration::ZERO,
        },
        started_seconds: started as f64 / ticks as f64,
    })
}

/// Parse `MemTotal`, `MemAvailable`, and `MemFree` from `/proc/meminfo`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_proc_meminfo(text: &str) -> Option<SystemMemory> {
    let mut total = None;
    let mut available = None;
    let mut free = None;
    for line in text.lines().take(256) {
        let (key, rest) = line.split_once(':')?;
        let value = rest
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok());
        match key {
            "MemTotal" => total = value,
            "MemAvailable" => available = value,
            "MemFree" => free = value,
            _ => {}
        }
        if total.is_some() && available.is_some() && free.is_some() {
            break;
        }
    }
    let total = total?.saturating_mul(1024);
    let free = free.unwrap_or_default().saturating_mul(1024);
    let available = available
        .map_or(free, |value| value.saturating_mul(1024))
        .min(total);
    Some(SystemMemory {
        total_bytes: total,
        available_bytes: available,
        free_bytes: free.min(total),
        used_bytes: total.saturating_sub(available),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real `/proc/self/stat` line, including a command name with a space and parentheses.
    const PROC_SELF_STAT: &str = "42 (my app (1)) R 1 42 42 0 -1 4194560 1234 0 0 0 250 125 0 0 20 0 7 0 900 104857600 512 18446744073709551615 1 1 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0\n";

    const PROC_MEMINFO: &str = "MemTotal:       16316360 kB\nMemFree:          210268 kB\nMemAvailable:    8127384 kB\nBuffers:          123456 kB\n";

    #[test]
    fn parses_proc_self_stat_with_a_spaced_command_name() {
        let parsed = parse_proc_self_stat(PROC_SELF_STAT, 100, 4096).unwrap();
        assert_eq!(parsed.metrics.cpu_user, Duration::from_millis(2_500));
        assert_eq!(parsed.metrics.cpu_system, Duration::from_millis(1_250));
        assert_eq!(parsed.metrics.thread_count, Some(7));
        assert_eq!(parsed.metrics.virtual_bytes, 104_857_600);
        assert_eq!(parsed.metrics.resident_bytes, 512 * 4096);
        assert_eq!(parsed.started_seconds, 9.0);
    }

    #[test]
    fn rejects_truncated_proc_self_stat() {
        assert!(parse_proc_self_stat("42 (app) R 1 2 3", 100, 4096).is_none());
        assert!(parse_proc_self_stat("no-parenthesis", 100, 4096).is_none());
    }

    #[test]
    fn parses_proc_meminfo_kilobyte_values() {
        let memory = parse_proc_meminfo(PROC_MEMINFO).unwrap();
        assert_eq!(memory.total_bytes, 16_316_360 * 1024);
        assert_eq!(memory.available_bytes, 8_127_384 * 1024);
        assert_eq!(memory.free_bytes, 210_268 * 1024);
        assert_eq!(
            memory.used_bytes,
            memory.total_bytes - memory.available_bytes
        );
    }

    #[test]
    fn falls_back_to_free_memory_without_a_mem_available_line() {
        let memory = parse_proc_meminfo("MemTotal: 1024 kB\nMemFree: 256 kB\n").unwrap();
        assert_eq!(memory.available_bytes, 256 * 1024);
        assert_eq!(memory.used_bytes, 768 * 1024);
    }

    #[test]
    fn converts_cpu_time_into_a_percentage_of_one_core() {
        assert_eq!(
            cpu_usage_percent(Duration::from_millis(500), Duration::from_secs(1)),
            Some(50.0)
        );
        assert_eq!(
            cpu_usage_percent(Duration::from_millis(1), Duration::ZERO),
            None
        );
        assert_eq!(
            cpu_usage_percent(
                Duration::from_secs(1),
                MAX_CPU_SAMPLE_INTERVAL + Duration::from_secs(1)
            ),
            None
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    #[test]
    fn reads_monotonically_sane_host_metrics() {
        let first = ProcessMetrics::current().unwrap();
        assert!(first.resident_bytes > 0);
        assert!(first.virtual_bytes >= first.resident_bytes);
        let mut sampler = CpuUsageSampler::new();
        let initial = sampler.sample().unwrap();
        assert!(initial.percent.is_none());
        let mut sink = 0_u64;
        for value in 0..2_000_000_u64 {
            sink = sink.wrapping_add(value);
        }
        assert!(sink > 0);
        let second = sampler.sample().unwrap();
        assert!(second.percent.is_some_and(|percent| percent >= 0.0));
        assert!(second.total_cpu_time >= initial.total_cpu_time);

        let later = ProcessMetrics::current().unwrap();
        assert!(later.cpu_total() >= first.cpu_total());
        assert!(later.uptime >= first.uptime);
    }

    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    #[test]
    fn reads_plausible_system_memory() {
        let memory = SystemMemory::current().unwrap();
        assert!(memory.total_bytes > 64 * 1024 * 1024);
        assert!(memory.available_bytes <= memory.total_bytes);
        assert_eq!(
            memory.used_bytes,
            memory.total_bytes - memory.available_bytes
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reports_a_macos_physical_footprint() {
        let metrics = ProcessMetrics::current().unwrap();
        assert!(metrics.footprint_bytes.is_some_and(|bytes| bytes > 0));
        assert!(metrics.thread_count.is_some_and(|count| count >= 1));
    }
}
