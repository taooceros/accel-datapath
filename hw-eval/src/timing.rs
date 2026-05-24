use std::mem;
use std::os::fd::RawFd;
use std::time::Instant;

use hw_eval::submit::{lfence, rdtscp};
use std::io;

const PERF_TYPE_HARDWARE: u32 = 0;
const PERF_COUNT_HW_CPU_CYCLES: u64 = 0;
const PERF_ATTR_DISABLED: u64 = 1 << 0;
const PERF_ATTR_EXCLUDE_KERNEL: u64 = 1 << 5;
const PERF_ATTR_EXCLUDE_HV: u64 = 1 << 6;

#[repr(C)]
#[derive(Clone, Copy)]
struct PerfEventAttr {
    type_: u32,
    size: u32,
    config: u64,
    sample_period_or_freq: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    wakeup_events: u32,
    bp_type: u32,
    config1: u64,
}

const PERF_EVENT_IOC_ENABLE: libc::c_ulong = 0x2400;
const PERF_EVENT_IOC_DISABLE: libc::c_ulong = 0x2401;
const PERF_EVENT_IOC_RESET: libc::c_ulong = 0x2403;
const PERF_FLAG_FD_CLOEXEC: libc::c_ulong = 1 << 3;

pub(crate) struct MeasurementTimers {
    pmu_cycles: Option<PmuCycleCounter>,
    pmu_error: Option<io::Error>,
    warned_pmu_unavailable: bool,
}

impl MeasurementTimers {
    pub(crate) fn new() -> Self {
        let (pmu_cycles, pmu_error) = match PmuCycleCounter::open_thread() {
            Ok(counter) => (Some(counter), None),
            Err(error) => (None, Some(error)),
        };

        Self {
            pmu_cycles,
            pmu_error,
            warned_pmu_unavailable: false,
        }
    }

    pub(crate) fn warn_if_pmu_unavailable(&mut self, json: bool) {
        if self.pmu_cycles.is_none() && !json && !self.warned_pmu_unavailable {
            if let Some(error) = &self.pmu_error {
                eprintln!(
                    "warning: PMU core-cycle counter unavailable ({error}); submit-only rows omit core_cycles"
                );
            }
            self.warned_pmu_unavailable = true;
        }
    }

    #[inline(always)]
    pub(crate) fn measure_tsc<F>(&mut self, f: F) -> u64
    where
        F: FnOnce(),
    {
        lfence();
        let start = rdtscp().0;
        f();
        let end = rdtscp().0;
        end - start
    }

    #[inline(always)]
    pub(crate) fn measure_wall<F>(&mut self, f: F) -> u64
    where
        F: FnOnce(),
    {
        let start = Instant::now();
        f();
        start.elapsed().as_nanos() as u64
    }

    #[inline(always)]
    pub(crate) fn measure_pmu<F>(&mut self, f: F) -> Option<u64>
    where
        F: FnOnce(),
    {
        let counter = self.pmu_cycles.as_ref()?;
        counter.reset_enable();
        f();
        counter.disable();
        Some(counter.read())
    }
}

struct PmuCycleCounter {
    fd: RawFd,
}

impl PmuCycleCounter {
    fn open_thread() -> std::io::Result<Self> {
        let attr = PerfEventAttr {
            type_: PERF_TYPE_HARDWARE,
            size: mem::size_of::<PerfEventAttr>() as u32,
            config: PERF_COUNT_HW_CPU_CYCLES,
            sample_period_or_freq: 0,
            sample_type: 0,
            read_format: 0,
            flags: PERF_ATTR_DISABLED | PERF_ATTR_EXCLUDE_KERNEL | PERF_ATTR_EXCLUDE_HV,
            wakeup_events: 0,
            bp_type: 0,
            config1: 0,
        };

        let fd = unsafe { perf_event_open(&attr, 0, -1, -1, PERF_FLAG_FD_CLOEXEC) };
        if fd < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self { fd })
        }
    }

    #[inline(always)]
    fn reset_enable(&self) {
        unsafe {
            libc::ioctl(self.fd, PERF_EVENT_IOC_RESET, 0);
            libc::ioctl(self.fd, PERF_EVENT_IOC_ENABLE, 0);
        }
    }

    #[inline(always)]
    fn disable(&self) {
        unsafe {
            libc::ioctl(self.fd, PERF_EVENT_IOC_DISABLE, 0);
        }
    }

    #[inline(always)]
    fn read(&self) -> u64 {
        let mut value = 0u64;
        let ret = unsafe {
            libc::read(
                self.fd,
                (&mut value as *mut u64).cast::<libc::c_void>(),
                mem::size_of::<u64>(),
            )
        };
        if ret == mem::size_of::<u64>() as isize {
            value
        } else {
            0
        }
    }
}

impl Drop for PmuCycleCounter {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

unsafe fn perf_event_open(
    attr: &PerfEventAttr,
    pid: libc::pid_t,
    cpu: libc::c_int,
    group_fd: libc::c_int,
    flags: libc::c_ulong,
) -> libc::c_int {
    libc::syscall(
        libc::SYS_perf_event_open,
        attr as *const PerfEventAttr,
        pid,
        cpu,
        group_fd,
        flags,
    ) as libc::c_int
}
