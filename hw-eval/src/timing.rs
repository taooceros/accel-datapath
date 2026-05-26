use std::mem;
use std::os::fd::RawFd;
use std::ptr;
use std::sync::atomic::{compiler_fence, Ordering};
use std::time::Instant;

use hw_eval::submit::{lfence, rdtscp};
use std::io;

const PERF_TYPE_HARDWARE: u32 = 0;
const PERF_COUNT_HW_CPU_CYCLES: u64 = 0;
const PERF_ATTR_DISABLED: u64 = 1 << 0;
const PERF_ATTR_EXCLUDE_KERNEL: u64 = 1 << 5;
const PERF_ATTR_EXCLUDE_HV: u64 = 1 << 6;
const PERF_CAP_USER_RDPMC: u64 = 1 << 2;

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
    rdpmc_cycles: Option<RdpmcCycleCounter>,
    rdpmc_error: Option<io::Error>,
    warned_pmu_unavailable: bool,
    warned_rdpmc_unavailable: bool,
}

impl MeasurementTimers {
    pub(crate) fn new() -> Self {
        Self {
            pmu_cycles: None,
            pmu_error: None,
            rdpmc_cycles: None,
            rdpmc_error: None,
            warned_pmu_unavailable: false,
            warned_rdpmc_unavailable: false,
        }
    }

    pub(crate) fn warn_if_pmu_unavailable(&mut self, json: bool) {
        self.ensure_pmu_cycles();
        if self.pmu_cycles.is_none() && !json && !self.warned_pmu_unavailable {
            if let Some(error) = &self.pmu_error {
                eprintln!(
                    "warning: PMU core-cycle counter unavailable ({error}); submit-only rows omit core_cycles"
                );
            }
            self.warned_pmu_unavailable = true;
        }
    }

    pub(crate) fn warn_if_rdpmc_unavailable(&mut self, json: bool) {
        self.ensure_rdpmc_cycles();
        if self.rdpmc_cycles.is_none() && !json && !self.warned_rdpmc_unavailable {
            if let Some(error) = &self.rdpmc_error {
                eprintln!(
                    "warning: RDPMC core-cycle counter unavailable ({error}); submit-only rows omit rdpmc_cycles"
                );
            }
            self.warned_rdpmc_unavailable = true;
        }
    }

    pub(crate) fn pmu_available(&self) -> bool {
        self.pmu_cycles.is_some()
    }

    pub(crate) fn rdpmc_available(&self) -> bool {
        self.rdpmc_cycles.is_some()
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
        self.ensure_pmu_cycles();
        let counter = self.pmu_cycles.as_ref()?;
        counter.reset_enable();
        f();
        counter.disable();
        Some(counter.read())
    }

    #[inline(always)]
    pub(crate) fn measure_rdpmc<F>(&mut self, f: F) -> Option<u64>
    where
        F: FnOnce(),
    {
        self.ensure_rdpmc_cycles();
        let counter = self.rdpmc_cycles.as_mut()?;
        counter.enable_once();
        let start = counter.read();
        f();
        let end = counter.read();
        Some(end.wrapping_sub(start))
    }

    fn ensure_pmu_cycles(&mut self) {
        if self.pmu_cycles.is_none() && self.pmu_error.is_none() {
            match PmuCycleCounter::open_thread() {
                Ok(counter) => self.pmu_cycles = Some(counter),
                Err(error) => self.pmu_error = Some(error),
            }
        }
    }

    fn ensure_rdpmc_cycles(&mut self) {
        if self.rdpmc_cycles.is_none() && self.rdpmc_error.is_none() {
            match RdpmcCycleCounter::open_thread() {
                Ok(counter) => self.rdpmc_cycles = Some(counter),
                Err(error) => self.rdpmc_error = Some(error),
            }
        }
    }
}

struct PmuCycleCounter {
    fd: RawFd,
}

impl PmuCycleCounter {
    fn open_thread_fd() -> std::io::Result<RawFd> {
        let attr = perf_cycles_attr();
        let fd = unsafe {
            // SAFETY: `attr` points to a valid perf_event_attr-compatible prefix
            // for the duration of the syscall. Other arguments request a
            // current-thread counter on any CPU with no group leader.
            perf_event_open(&attr, 0, -1, -1, PERF_FLAG_FD_CLOEXEC)
        };
        if fd < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    }

    fn open_thread() -> std::io::Result<Self> {
        Ok(Self {
            fd: Self::open_thread_fd()?,
        })
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

#[repr(C)]
struct PerfEventMmapPage {
    version: u32,
    compat_version: u32,
    lock: u32,
    index: u32,
    offset: i64,
    time_enabled: u64,
    time_running: u64,
    capabilities: u64,
    pmc_width: u16,
    time_shift: u16,
    time_mult: u32,
    time_offset: u64,
    time_zero: u64,
    size: u32,
    reserved_1: u32,
    time_cycles: u64,
    time_mask: u64,
}

struct RdpmcCycleCounter {
    fd: RawFd,
    page: *mut PerfEventMmapPage,
    page_len: usize,
    enabled: bool,
}

impl RdpmcCycleCounter {
    fn open_thread() -> io::Result<Self> {
        let fd = PmuCycleCounter::open_thread_fd()?;
        match Self::map_fd(fd) {
            Ok(counter) => Ok(counter),
            Err(error) => {
                unsafe {
                    // SAFETY: `fd` was returned by `perf_event_open` above and is
                    // not owned by any Rust object on this error path.
                    libc::close(fd);
                }
                Err(error)
            }
        }
    }

    fn map_fd(fd: RawFd) -> io::Result<Self> {
        let page_len = page_size()?;
        let mapped = unsafe {
            // SAFETY: Mapping one metadata page from a valid perf event fd.
            // The mapping is checked against MAP_FAILED before use and unmapped
            // exactly once in Drop.
            libc::mmap(
                ptr::null_mut(),
                page_len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if mapped == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        let page = mapped.cast::<PerfEventMmapPage>();
        let capabilities = unsafe {
            // SAFETY: `page` points to the mapped perf metadata page and is
            // valid for volatile field reads until Drop unmaps it.
            ptr::read_volatile(ptr::addr_of!((*page).capabilities))
        };
        if capabilities & PERF_CAP_USER_RDPMC == 0 {
            unsafe {
                // SAFETY: `mapped`/`page_len` came from the successful mmap
                // call above and have not yet been transferred to Self.
                libc::munmap(mapped, page_len);
            }
            return Err(io::Error::other("perf event does not allow user RDPMC"));
        }

        Ok(Self {
            fd,
            page,
            page_len,
            enabled: false,
        })
    }

    fn enable_once(&mut self) {
        if !self.enabled {
            unsafe {
                // SAFETY: `fd` is a live perf event owned by this counter. The
                // ioctls reset and enable only this event.
                libc::ioctl(self.fd, PERF_EVENT_IOC_RESET, 0);
                libc::ioctl(self.fd, PERF_EVENT_IOC_ENABLE, 0);
            }
            self.enabled = true;
        }
    }

    #[inline(always)]
    fn read(&self) -> u64 {
        loop {
            let seq = unsafe {
                // SAFETY: `page` is a live perf metadata mapping. Volatile
                // reads are required because the kernel updates this memory.
                ptr::read_volatile(ptr::addr_of!((*self.page).lock))
            };
            if seq & 1 != 0 {
                continue;
            }

            compiler_fence(Ordering::SeqCst);

            let index = unsafe { ptr::read_volatile(ptr::addr_of!((*self.page).index)) };
            let offset = unsafe { ptr::read_volatile(ptr::addr_of!((*self.page).offset)) };
            let width = unsafe { ptr::read_volatile(ptr::addr_of!((*self.page).pmc_width)) };

            let pmc = if index == 0 {
                0
            } else {
                read_rdpmc(index - 1, width)
            };

            compiler_fence(Ordering::SeqCst);

            let seq_after = unsafe { ptr::read_volatile(ptr::addr_of!((*self.page).lock)) };
            if seq == seq_after {
                return offset.wrapping_add(pmc) as u64;
            }
        }
    }
}

impl Drop for RdpmcCycleCounter {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: `page`/`page_len` represent the mmap owned by this object,
            // and `fd` is the perf event fd owned by this object.
            libc::munmap(self.page.cast::<libc::c_void>(), self.page_len);
            libc::close(self.fd);
        }
    }
}

fn perf_cycles_attr() -> PerfEventAttr {
    PerfEventAttr {
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
    }
}

fn page_size() -> io::Result<usize> {
    let value = unsafe {
        // SAFETY: `sysconf(_SC_PAGESIZE)` has no pointer arguments and does not
        // impose Rust-side aliasing or lifetime requirements.
        libc::sysconf(libc::_SC_PAGESIZE)
    };
    if value <= 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(value as usize)
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn read_rdpmc(counter: u32, width: u16) -> i64 {
    let low: u32;
    let high: u32;
    unsafe {
        // SAFETY: Caller only reaches this after perf_event_mmap_page reports
        // cap_user_rdpmc and provides a nonzero event index for this thread.
        core::arch::asm!(
            "rdpmc",
            in("ecx") counter,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    let value = (u64::from(high) << 32 | u64::from(low)) as i64;

    let shift = 64_u32.saturating_sub(u32::from(width));
    (value << shift) >> shift
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn read_rdpmc(_counter: u32, _width: u16) -> i64 {
    0
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
