//! Low-level Intel DSA hardware bindings — zero framework overhead.
//!
//! Directly maps the WQ portal, fills descriptors, submits via MOVDIR64B/ENQCMD,
//! and polls completion records. No allocators, no async, no abstractions.

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;

/// Bindgen-backed subset of the kernel `linux/idxd.h` UAPI used by IDXD
/// accelerator consumers, including DSA descriptor/completion ABI and IAX
/// definitions.
pub mod idxd_uapi {
    #![allow(
        non_camel_case_types,
        non_upper_case_globals,
        non_snake_case,
        dead_code
    )]
    include!(concat!(env!("OUT_DIR"), "/idxd_uapi_bindings.rs"));
}

/// Backward-compatible alias for existing callers that imported the generated
/// IDXD UAPI subset as `idxd_sys::idxd`.
pub use idxd_uapi as idxd;

// ============================================================================
// Bindgen-backed hardware descriptor and completion helpers
// ============================================================================

/// Bindgen-backed DSA hardware descriptor storage.
///
/// The ABI fields come from `linux/idxd.h` via bindgen (`idxd_uapi::dsa_hw_desc`).
/// This wrapper only restores the 64-byte alignment required by MOVDIR64B
/// descriptor submission; it does not define an independent descriptor layout.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct BindgenDsaHwDesc {
    raw: idxd_uapi::dsa_hw_desc,
}

/// Public descriptor helper type used by higher-level Rust bindings.
pub type DsaHwDesc = BindgenDsaHwDesc;

impl Default for BindgenDsaHwDesc {
    fn default() -> Self {
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

/// Bindgen-backed DSA completion record storage.
///
/// The ABI fields come from `linux/idxd.h` via bindgen
/// (`idxd_uapi::dsa_completion_record`). This wrapper only restores the
/// 32-byte alignment expected by the DSA completion record contract.
#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct BindgenDsaCompletionRecord {
    raw: idxd_uapi::dsa_completion_record,
}

/// Public completion helper type used by higher-level Rust bindings.
pub type DsaCompletionRecord = BindgenDsaCompletionRecord;

impl Default for BindgenDsaCompletionRecord {
    fn default() -> Self {
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

impl BindgenDsaCompletionRecord {
    #[inline(always)]
    fn as_raw_ptr(&self) -> *const idxd_uapi::dsa_completion_record {
        ptr::addr_of!(self.raw)
    }

    #[inline(always)]
    fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::dsa_completion_record {
        ptr::addr_of_mut!(self.raw)
    }

    /// Read the volatile hardware completion status byte.
    #[inline(always)]
    pub fn status(&self) -> u8 {
        unsafe { ptr::read_volatile(ptr::addr_of!((*self.as_raw_ptr()).status)) }
    }

    /// Read the result byte from the generated completion union.
    #[inline(always)]
    pub fn result(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).__bindgen_anon_1.result)) }
    }

    /// Read bytes completed from the generated completion record.
    #[inline(always)]
    pub fn bytes_completed(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).bytes_completed)) }
    }

    /// Read the faulting address from the generated completion record.
    #[inline(always)]
    pub fn fault_addr(&self) -> u64 {
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).fault_addr)) }
    }

    /// Read CRC value from completion record (for crc_gen / copy_crc ops).
    pub fn crc_value(&self) -> u64 {
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).__bindgen_anon_2.crc_val)) }
    }
}

// ============================================================================
// DSA opcodes and flags
// ============================================================================

pub const DSA_OPCODE_NOOP: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_NOOP as u8;
pub const DSA_OPCODE_BATCH: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_BATCH as u8;
pub const DSA_OPCODE_MEMMOVE: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMMOVE as u8;
pub const DSA_OPCODE_MEMFILL: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMFILL as u8;
pub const DSA_OPCODE_COMPARE: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_COMPARE as u8;
pub const DSA_OPCODE_COMPVAL: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_COMPVAL as u8;
pub const DSA_OPCODE_DUALCAST: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_DUALCAST as u8;
pub const DSA_OPCODE_CRCGEN: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_CRCGEN as u8;
pub const DSA_OPCODE_COPY_CRC: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_COPY_CRC as u8;
pub const DSA_OPCODE_CFLUSH: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_CFLUSH as u8;

pub const IDXD_OP_FLAG_CRAV: u32 = idxd_uapi::IDXD_OP_FLAG_CRAV;
pub const IDXD_OP_FLAG_RCR: u32 = idxd_uapi::IDXD_OP_FLAG_RCR;
pub const IDXD_OP_FLAG_CC: u32 = idxd_uapi::IDXD_OP_FLAG_CC;

pub const DSA_COMP_NONE: u8 = idxd_uapi::dsa_completion_status::DSA_COMP_NONE as u8;
pub const DSA_COMP_SUCCESS: u8 = idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS as u8;
pub const DSA_COMP_PAGE_FAULT_NOBOF: u8 =
    idxd_uapi::dsa_completion_status::DSA_COMP_PAGE_FAULT_NOBOF as u8;
pub const DSA_COMP_STATUS_MASK: u8 = idxd_uapi::DSA_COMP_STATUS_MASK as u8;

// ============================================================================
// Descriptor builders
// ============================================================================

impl BindgenDsaHwDesc {
    #[inline(always)]
    fn as_raw_ptr(&self) -> *const idxd_uapi::dsa_hw_desc {
        ptr::addr_of!(self.raw)
    }

    #[inline(always)]
    fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::dsa_hw_desc {
        ptr::addr_of_mut!(self.raw)
    }

    /// Set opcode and standard flags (RCR + CRAV).
    fn set_opcode_flags(&mut self, opcode: u8, extra_flags: u32) {
        let flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | extra_flags;
        self.raw.set_flags(flags & 0x00FF_FFFF);
        self.raw.set_opcode(opcode as u32);
    }

    /// Set completion record address.
    pub fn set_completion(&mut self, comp: &mut DsaCompletionRecord) {
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).completion_addr),
                comp.as_raw_mut_ptr() as u64,
            );
        }
    }

    /// Fill for memmove (data_move) operation.
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMMOVE, IDXD_OP_FLAG_CC);
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
        }
    }

    /// Fill for CRC generation operation.
    pub fn fill_crc_gen(&mut self, src: *const u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_CRCGEN, 0);
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
            ptr::copy_nonoverlapping(
                seed.to_le_bytes().as_ptr(),
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_4.op_specific)
                    .cast::<u8>(),
                std::mem::size_of::<u32>(),
            );
        }
    }

    /// Fill for copy + CRC operation (fused copy and CRC-32C).
    pub fn fill_copy_crc(&mut self, src: *const u8, dst: *mut u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COPY_CRC, 0);
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
            ptr::copy_nonoverlapping(
                seed.to_le_bytes().as_ptr(),
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_4.op_specific)
                    .cast::<u8>(),
                std::mem::size_of::<u32>(),
            );
        }
    }

    /// Fill for memory fill operation.
    pub fn fill_memfill(&mut self, dst: *mut u8, size: u32, pattern: u64) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMFILL, IDXD_OP_FLAG_CC);
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.pattern),
                pattern,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
        }
    }

    /// Fill for compare operation.
    pub fn fill_compare(&mut self, src1: *const u8, src2: *const u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COMPARE, 0);
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src1 as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.src2_addr),
                src2 as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
        }
    }

    /// Fill for batch operation.
    pub fn fill_batch(&mut self, desc_list: *const DsaHwDesc, count: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_BATCH, 0);
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.desc_list_addr),
                desc_list as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.desc_count),
                count,
            );
        }
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
            src = in(reg) desc.as_raw_ptr(),
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
            src = in(reg) desc.as_raw_ptr(),
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
        let status = comp.status();
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
        let status = comp.status();
        if status == DSA_COMP_NONE {
            poll_completion(comp);
        }
    }
}

/// Touch the faulted page reported in a completion record (write touch for
/// destination faults, ensuring the PTE is mapped writable for DMA).
pub fn touch_fault_page(comp: &DsaCompletionRecord) {
    let addr = comp.fault_addr();
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
