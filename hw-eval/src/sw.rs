//! Software baselines used by hw-eval.

#[inline(never)]
pub fn sw_memcpy(dst: &mut [u8], src: &[u8]) {
    dst[..src.len()].copy_from_slice(src);
}

#[inline(never)]
pub fn sw_crc32c(data: &[u8], seed: u32) -> u32 {
    let mut crc = seed;
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
