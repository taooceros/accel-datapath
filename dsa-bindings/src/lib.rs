//! Low-level Intel DSA hardware bindings — zero framework overhead.
//!
//! Directly maps the WQ portal, fills descriptors, submits via MOVDIR64B/ENQCMD,
//! and polls completion records. No allocators, no async, no abstractions.

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;

/// Bindgen-backed subset of the kernel `linux/idxd.h` UAPI used by IAX
/// consumers.
pub mod idxd {
    #![allow(
        non_camel_case_types,
        non_upper_case_globals,
        non_snake_case,
        dead_code
    )]
    include!(concat!(env!("OUT_DIR"), "/idxd_iax_bindings.rs"));
}

// ============================================================================
// Hardware descriptor and completion record (mirrors linux/idxd.h)
// ============================================================================

/// DSA hardware descriptor — 64 bytes, must be 64-byte aligned.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct DsaHwDesc {
    pub pasid_priv: u32,   // pasid:20, rsvd:11, priv:1
    pub flags_opcode: u32, // flags:24, opcode:8
    pub completion_addr: u64,
    pub src_addr: u64,
    pub dst_addr: u64,
    pub xfer_size: u32,
    pub int_handle: u16,
    pub rsvd1: u16,
    // op_specific fields (24 bytes)
    pub op_specific: [u8; 24],
}

impl Default for DsaHwDesc {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// DSA completion record — 32 bytes, must be 32-byte aligned.
#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct DsaCompletionRecord {
    pub status: u8, // volatile — hardware writes this
    pub result: u8,
    pub rsvd: u16,
    pub bytes_completed: u32,
    pub fault_addr: u64,
    pub op_specific: [u8; 16],
}

impl Default for DsaCompletionRecord {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl DsaCompletionRecord {
    /// Read CRC value from completion record (for crc_gen / copy_crc ops).
    pub fn crc_value(&self) -> u64 {
        u64::from_le_bytes(self.op_specific[..8].try_into().unwrap())
    }
}

// ============================================================================
// DSA opcodes and flags
// ============================================================================

pub const DSA_OPCODE_NOOP: u8 = 0x00;
pub const DSA_OPCODE_BATCH: u8 = 0x01;
pub const DSA_OPCODE_MEMMOVE: u8 = 0x03;
pub const DSA_OPCODE_MEMFILL: u8 = 0x04;
pub const DSA_OPCODE_COMPARE: u8 = 0x05;
pub const DSA_OPCODE_COMPVAL: u8 = 0x06;
pub const DSA_OPCODE_DUALCAST: u8 = 0x09;
pub const DSA_OPCODE_CRCGEN: u8 = 0x10;
pub const DSA_OPCODE_COPY_CRC: u8 = 0x11;
pub const DSA_OPCODE_CFLUSH: u8 = 0x20;

pub const IDXD_OP_FLAG_CRAV: u32 = 0x0004;
pub const IDXD_OP_FLAG_RCR: u32 = 0x0008;
pub const IDXD_OP_FLAG_CC: u32 = 0x0100;

pub const DSA_COMP_NONE: u8 = 0;
pub const DSA_COMP_SUCCESS: u8 = 1;
pub const DSA_COMP_PAGE_FAULT_NOBOF: u8 = 3;
pub const DSA_COMP_STATUS_MASK: u8 = 0x7f;

// ============================================================================
// Descriptor builders
// ============================================================================

impl DsaHwDesc {
    /// Set opcode and standard flags (RCR + CRAV).
    fn set_opcode_flags(&mut self, opcode: u8, extra_flags: u32) {
        let flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | extra_flags;
        self.flags_opcode = (flags & 0x00FF_FFFF) | ((opcode as u32) << 24);
    }

    /// Set completion record address.
    pub fn set_completion(&mut self, comp: &mut DsaCompletionRecord) {
        self.completion_addr = comp as *mut DsaCompletionRecord as u64;
    }

    /// Fill for memmove (data_move) operation.
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMMOVE, IDXD_OP_FLAG_CC);
        self.src_addr = src as u64;
        self.dst_addr = dst as u64;
        self.xfer_size = size;
    }

    /// Fill for CRC generation operation.
    pub fn fill_crc_gen(&mut self, src: *const u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_CRCGEN, 0);
        self.src_addr = src as u64;
        self.xfer_size = size;
        // crc_seed is at op_specific[0..4]
        self.op_specific[0..4].copy_from_slice(&seed.to_le_bytes());
    }

    /// Fill for copy + CRC operation (fused copy and CRC-32C).
    pub fn fill_copy_crc(&mut self, src: *const u8, dst: *mut u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COPY_CRC, 0);
        self.src_addr = src as u64;
        self.dst_addr = dst as u64;
        self.xfer_size = size;
        // crc_seed is at op_specific[0..4]
        self.op_specific[0..4].copy_from_slice(&seed.to_le_bytes());
    }

    /// Fill for memory fill operation.
    pub fn fill_memfill(&mut self, dst: *mut u8, size: u32, pattern: u64) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMFILL, IDXD_OP_FLAG_CC);
        self.src_addr = pattern; // pattern goes in src_addr union
        self.dst_addr = dst as u64;
        self.xfer_size = size;
    }

    /// Fill for compare operation.
    pub fn fill_compare(&mut self, src1: *const u8, src2: *const u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COMPARE, 0);
        self.src_addr = src1 as u64;
        self.dst_addr = src2 as u64; // src2_addr is in dst_addr union
        self.xfer_size = size;
    }

    /// Fill for batch operation.
    pub fn fill_batch(&mut self, desc_list: *const DsaHwDesc, count: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_BATCH, 0);
        self.src_addr = desc_list as u64; // desc_list_addr union
        self.xfer_size = count; // desc_count union
    }

    /// Fill for noop operation (useful for measuring submission overhead).
    pub fn fill_noop(&mut self) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_NOOP, 0);
    }
}

// ============================================================================
// WQ portal — mmap the device file for MMIO submission
// ============================================================================

pub struct WqPortal {
    portal: *mut u8,
    dedicated: bool,
}

// Safety: WqPortal is used from a single thread in benchmarks.
unsafe impl Send for WqPortal {}
unsafe impl Sync for WqPortal {}

impl WqPortal {
    /// Open a DSA work queue device (e.g., "/dev/dsa/wq0.0").
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let fd = file.as_raw_fd();

        // mmap the WQ portal — one page (4096 bytes)
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

        let dedicated = detect_wq_mode(path);

        Ok(Self {
            portal: portal as *mut u8,
            dedicated,
        })
    }

    /// Returns true if this is a dedicated WQ (MOVDIR64B), false for shared (ENQCMD).
    pub fn is_dedicated(&self) -> bool {
        self.dedicated
    }

    /// Submit a descriptor to the work queue via MOVDIR64B (dedicated WQ).
    ///
    /// # Safety
    /// Descriptor must be valid and 64-byte aligned. Completion record must
    /// remain valid until the operation completes.
    #[inline(always)]
    pub unsafe fn submit_movdir64b(&self, desc: &DsaHwDesc) {
        core::arch::asm!(
            "movdir64b ({src}), {dst}",
            dst = in(reg) self.portal,
            src = in(reg) desc as *const DsaHwDesc,
            options(nostack, preserves_flags, att_syntax),
        );
    }

    /// Submit a descriptor via ENQCMD (shared WQ). Returns true if accepted.
    ///
    /// # Safety
    /// Same requirements as submit_movdir64b.
    #[inline(always)]
    pub unsafe fn submit_enqcmd(&self, desc: &DsaHwDesc) -> bool {
        let mut retry: u8;
        core::arch::asm!(
            "enqcmd {dst}, [{src}]", // Intel syntax: dst, [src]
            "setnz {result}",        // ZF=0 (success) -> result=1
            dst = in(reg) self.portal,
            src = in(reg) desc,
            result = out(reg_byte) retry,
            // Removed preserves_flags because we modify ZF
            options(nostack),
        );
        retry != 0
    }

    /// Submit a descriptor using the appropriate method for this WQ type.
    ///
    /// # Safety
    /// Descriptor and completion record must be valid.
    #[inline(always)]
    pub unsafe fn submit(&self, desc: &DsaHwDesc) {
        if self.dedicated {
            self.submit_movdir64b(desc);
        } else {
            // Retry until accepted (shared WQ may reject under contention)
            while !self.submit_enqcmd(desc) {
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

// ============================================================================
// Polling — busy-wait on completion record
// ============================================================================

/// Poll a completion record until status is non-zero.
/// Returns the status byte.
#[inline(always)]
pub fn poll_completion(comp: &DsaCompletionRecord) -> u8 {
    const MAX_SPINS: u64 = 2_000_000_000; // ~1s at ~2GHz
    let mut spins: u64 = 0;
    loop {
        // Volatile read — hardware writes this field
        let status = unsafe { ptr::read_volatile(&comp.status) };
        if status != DSA_COMP_NONE {
            return status & DSA_COMP_STATUS_MASK;
        }
        spins += 1;
        if spins >= MAX_SPINS {
            eprintln!("poll_completion: timeout after {} spins", spins);
            return 0xFF;
        }
        core::hint::spin_loop();
    }
}

/// Reset a completion record for reuse.
#[inline(always)]
pub fn reset_completion(comp: &mut DsaCompletionRecord) {
    unsafe {
        ptr::write_bytes(comp as *mut DsaCompletionRecord, 0, 1);
    }
}

/// Drain all in-flight descriptors by polling every completion record to non-zero.
/// Must be called before dropping buffers when descriptors may still be in-flight,
/// otherwise closing the WQ fd with in-flight DMA causes kernel D-state hang.
pub fn drain_completions(comps: &[DsaCompletionRecord]) {
    for comp in comps {
        let status = unsafe { ptr::read_volatile(&comp.status) };
        if status == DSA_COMP_NONE {
            poll_completion(comp);
        }
    }
}

/// Touch the faulted page reported in a completion record (write touch for
/// destination faults, ensuring the PTE is mapped writable for DMA).
pub fn touch_fault_page(comp: &DsaCompletionRecord) {
    let addr = unsafe { ptr::read_volatile(&comp.fault_addr) };
    if addr != 0 {
        unsafe {
            let p = addr as *mut u8;
            // Write touch to ensure writable PTE
            ptr::write_volatile(p, ptr::read_volatile(p));
        }
    }
}

// ============================================================================
// Cycle-accurate timing via RDTSCP
// ============================================================================

/// Read TSC with serialization. Returns (cycles, processor_id).
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

/// Serializing fence before timing region.
#[inline(always)]
pub fn lfence() {
    unsafe {
        core::arch::asm!("lfence", options(nostack, nomem, preserves_flags));
    }
}

/// Detect TSC frequency in Hz. Parses /proc/cpuinfo for base frequency,
/// falls back to calibration against Instant.
pub fn tsc_frequency_hz() -> u64 {
    // Strategy 1: parse "model name" line for "@ X.XXGHz"
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
    // Strategy 2: calibrate against Instant over 10ms
    let start_tsc = rdtscp().0;
    let start_wall = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let end_tsc = rdtscp().0;
    let elapsed_ns = start_wall.elapsed().as_nanos() as u64;
    (end_tsc - start_tsc) * 1_000_000_000 / elapsed_ns
}

/// Convert cycles to nanoseconds given a known TSC frequency.
#[inline(always)]
pub fn cycles_to_ns(cycles: u64, tsc_freq_hz: u64) -> u64 {
    ((cycles as u128 * 1_000_000_000) / tsc_freq_hz as u128) as u64
}

// ============================================================================
// WQ mode detection
// ============================================================================

/// Detect WQ mode from sysfs. Returns true for dedicated, false for shared.
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
        Ok(mode) => {
            let mode = mode.trim();
            if mode == "dedicated" {
                true
            } else if mode == "shared" {
                false
            } else {
                eprintln!("WARNING: unknown WQ mode '{}', assuming dedicated", mode);
                true
            }
        }
        Err(_) => {
            eprintln!("WARNING: cannot read {}, assuming dedicated WQ", sysfs);
            true
        }
    }
}

// ============================================================================
// Thread pinning and NUMA topology
// ============================================================================

/// Pin the calling thread to the specified CPU core.
pub fn pin_to_core(core: usize) -> std::io::Result<usize> {
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

/// Get the current CPU core.
pub fn current_core() -> usize {
    unsafe { libc::sched_getcpu() as usize }
}

/// Get the NUMA node for a CPU core.
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

/// Get the NUMA node for a DSA device (e.g., /dev/dsa/wq0.0 -> dsa0).
pub fn device_numa_node(dev_path: &Path) -> Option<i32> {
    let filename = dev_path.file_name()?.to_str()?;
    let dsa_id = filename.strip_prefix("wq")?;
    let dot = dsa_id.find('.')?;
    let dsa_device = format!("dsa{}", &dsa_id[..dot]);
    let sysfs = format!("/sys/bus/dsa/devices/{}/numa_node", dsa_device);
    std::fs::read_to_string(&sysfs).ok()?.trim().parse().ok()
}

// ============================================================================
// Cache control
// ============================================================================

/// Flush all cache lines covering [ptr, ptr+len).
pub fn flush_range(ptr: *const u8, len: usize) {
    let mut offset = 0;
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
