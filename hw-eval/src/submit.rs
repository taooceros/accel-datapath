//! Shared submission, polling primitive, timing, and topology helpers.
//!
//! Accelerator-specific descriptor/completion formats live in `dsa.rs` and `iax.rs`.

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;

pub const IDXD_OP_FLAG_CRAV: u32 = 0x0004;
pub const IDXD_OP_FLAG_RCR: u32 = 0x0008;
pub const IDXD_OP_FLAG_CC: u32 = 0x0100;
pub const IDXD_OP_FLAG_RD_SRC2_AECS: u32 = 0x010000;

pub const STATUS_MASK: u8 = 0x7f;

pub struct WqPortal {
    portal: *mut u8,
    dedicated: bool,
}

unsafe impl Send for WqPortal {}
unsafe impl Sync for WqPortal {}

impl WqPortal {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let fd = file.as_raw_fd();

        let portal = unsafe {
            libc::mmap(
                ptr::null_mut(),
                4096,
                libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                0,
            )
        };

        if portal == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            portal: portal as *mut u8,
            dedicated: detect_wq_mode(path),
        })
    }

    pub fn is_dedicated(&self) -> bool {
        self.dedicated
    }

    #[inline(always)]
    unsafe fn submit_movdir64b_raw(&self, desc: *const u8) {
        core::arch::asm!(
            "movdir64b ({src}), {dst}",
            dst = in(reg) self.portal,
            src = in(reg) desc,
            options(nostack, preserves_flags, att_syntax),
        );
    }

    #[inline(always)]
    unsafe fn submit_enqcmd_raw(&self, desc: *const u8) -> bool {
        let mut accepted: u8;
        core::arch::asm!(
            "enqcmd {dst}, [{src}]",
            "setnz {accepted}",
            dst = in(reg) self.portal,
            src = in(reg) desc,
            accepted = out(reg_byte) accepted,
            options(nostack),
        );
        accepted != 0
    }

    #[inline(always)]
    pub unsafe fn submit_desc64(&self, desc: *const u8) {
        if self.dedicated {
            self.submit_movdir64b_raw(desc);
        } else {
            while !self.submit_enqcmd_raw(desc) {
                core::hint::spin_loop();
            }
        }
    }
}

impl Drop for WqPortal {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.portal as *mut libc::c_void, 4096);
        }
    }
}

#[inline(always)]
pub(crate) fn poll_status(status_ptr: *const u8, none_value: u8) -> u8 {
    const MAX_SPINS: u64 = 2_000_000_000;
    let mut spins: u64 = 0;

    loop {
        let status = unsafe { ptr::read_volatile(status_ptr) };
        if status != none_value {
            return status & STATUS_MASK;
        }

        spins += 1;
        if spins >= MAX_SPINS {
            eprintln!("poll completion: timeout after {} spins", spins);
            return 0xFF;
        }
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub(crate) fn zero_record<T>(record: &mut T) {
    unsafe {
        ptr::write_bytes(record as *mut T, 0, 1);
    }
}

#[inline(always)]
pub fn rdtscp() -> (u64, u32) {
    let lo: u32;
    let hi: u32;
    let aux: u32;
    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") lo,
            out("edx") hi,
            out("ecx") aux,
            options(nostack, nomem, preserves_flags),
        );
    }
    (((hi as u64) << 32) | lo as u64, aux)
}

#[inline(always)]
pub fn lfence() {
    unsafe {
        core::arch::asm!("lfence", options(nostack, nomem, preserves_flags));
    }
}

pub fn tsc_frequency_hz() -> u64 {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if line.starts_with("model name") {
                if let Some(at_pos) = line.find("@ ") {
                    let ghz_str = &line[at_pos + 2..];
                    if let Some(ghz_end) = ghz_str.find("GHz") {
                        if let Ok(ghz) = ghz_str[..ghz_end].trim().parse::<f64>() {
                            return (ghz * 1e9) as u64;
                        }
                    }
                }
            }
        }
    }

    let start_tsc = rdtscp().0;
    let start_wall = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let end_tsc = rdtscp().0;
    let elapsed_ns = start_wall.elapsed().as_nanos() as u64;
    (end_tsc - start_tsc) * 1_000_000_000 / elapsed_ns
}

#[inline(always)]
pub fn cycles_to_ns(cycles: u64, tsc_freq_hz: u64) -> u64 {
    ((cycles as u128 * 1_000_000_000) / tsc_freq_hz as u128) as u64
}

fn detect_wq_mode(dev_path: &Path) -> bool {
    let filename = match dev_path.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => {
            eprintln!(
                "WARNING: cannot parse device name from {:?}, assuming dedicated WQ",
                dev_path
            );
            return true;
        }
    };

    let sysfs = format!("/sys/bus/dsa/devices/{}/mode", filename);
    match std::fs::read_to_string(&sysfs) {
        Ok(mode) => match mode.trim() {
            "dedicated" => true,
            "shared" => false,
            m => {
                eprintln!("WARNING: unknown WQ mode '{}', assuming dedicated", m);
                true
            }
        },
        Err(_) => {
            eprintln!("WARNING: cannot read {}, assuming dedicated WQ", sysfs);
            true
        }
    }
}

pub fn pin_to_core(core: usize) -> std::io::Result<usize> {
    let max_core = std::mem::size_of::<libc::cpu_set_t>() * 8;
    if core >= max_core {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("requested CPU core {core} exceeds affinity set capacity {max_core}"),
        ));
    }

    unsafe {
        let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut cpuset);
        libc::CPU_SET(core, &mut cpuset);
        let ret = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpuset);
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(core)
}

pub fn current_core() -> usize {
    unsafe { libc::sched_getcpu() as usize }
}

pub fn cpu_numa_node(core: usize) -> Option<usize> {
    let cpu_dir = format!("/sys/devices/system/cpu/cpu{}", core);
    for entry in std::fs::read_dir(&cpu_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_str()?;
        if name.starts_with("node") {
            return name[4..].parse().ok();
        }
    }
    None
}

pub fn device_numa_node(dev_path: &Path) -> Option<i32> {
    let wq = dev_path.file_name()?.to_str()?;
    let sysfs = format!("/sys/bus/dsa/devices/{}/device/numa_node", wq);
    std::fs::read_to_string(&sysfs).ok()?.trim().parse().ok()
}

pub fn flush_range(ptr: *const u8, len: usize) {
    let mut offset = 0usize;
    while offset < len {
        unsafe {
            core::arch::asm!(
                "clflush [{}]",
                in(reg) ptr.add(offset),
                options(nostack, preserves_flags),
            );
        }
        offset += 64;
    }
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }
}
