//! DSA interface for hw-eval benchmarks.
//!
//! Re-exports all hardware bindings from `dsa_bindings` and provides
//! benchmark-specific software baselines.

pub use dsa_bindings::*;

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
