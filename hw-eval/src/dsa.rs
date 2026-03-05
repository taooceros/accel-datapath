//! Raw DSA hardware interface — zero framework overhead.
//!
//! Directly maps the WQ portal, fills descriptors, submits via MOVDIR64B/ENQCMD,
//! and polls completion records. No allocators, no async, no abstractions.

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;

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
    _fd: std::fs::File,
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

        // Detect if dedicated WQ by trying to check the device name
        // Dedicated WQs use movdir64b, shared WQs use enqcmd
        // Default to dedicated WQ (movdir64b). Set dedicated=false for shared WQs.
        let dedicated = true;

        Ok(Self {
            portal: portal as *mut u8,
            _fd: file,
            dedicated,
        })
    }

    /// Submit a descriptor to the work queue via MOVDIR64B (dedicated WQ).
    ///
    /// # Safety
    /// Descriptor must be valid and 64-byte aligned. Completion record must
    /// remain valid until the operation completes.
    #[inline(always)]
    pub unsafe fn submit_movdir64b(&self, desc: &DsaHwDesc) {
        core::arch::asm!(
            ".byte 0x66, 0x0f, 0x38, 0xf8, 0x02", // movdir64b (%rdx), %rax
            in("rax") self.portal,
            in("rdx") desc as *const DsaHwDesc,
            options(nostack, preserves_flags),
        );
    }

    /// Submit a descriptor via ENQCMD (shared WQ). Returns true if accepted.
    ///
    /// # Safety
    /// Same requirements as submit_movdir64b.
    #[inline(always)]
    pub unsafe fn submit_enqcmd(&self, desc: &DsaHwDesc) -> bool {
        let result: u8;
        core::arch::asm!(
            ".byte 0xf2, 0x0f, 0x38, 0xf8, 0x02", // enqcmd (%rdx), %rax
            "setz {result}",
            in("rax") self.portal,
            in("rdx") desc as *const DsaHwDesc,
            result = out(reg_byte) result,
            options(nostack, preserves_flags),
        );
        result != 0
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
            // Retry until accepted
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
    loop {
        // Volatile read — hardware writes this field
        let status = unsafe { ptr::read_volatile(&comp.status) };
        if status != DSA_COMP_NONE {
            return status & DSA_COMP_STATUS_MASK;
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

// ============================================================================
// Software baselines
// ============================================================================

/// Software memcpy baseline.
#[inline(never)]
pub fn sw_memcpy(dst: &mut [u8], src: &[u8]) {
    dst[..src.len()].copy_from_slice(src);
}

/// Software CRC-32C using SSE4.2 intrinsic.
#[inline(never)]
pub fn sw_crc32c(data: &[u8], seed: u32) -> u32 {
    let mut crc = seed;
    // Process 8 bytes at a time using CRC32Q
    let chunks = data.len() / 8;

    unsafe {
        let ptr = data.as_ptr() as *const u64;
        for i in 0..chunks {
            crc = core::arch::x86_64::_mm_crc32_u64(crc as u64, *ptr.add(i)) as u32;
        }
        let tail = &data[chunks * 8..];
        for &byte in tail {
            crc = core::arch::x86_64::_mm_crc32_u8(crc, byte);
        }
    }
    crc
}
