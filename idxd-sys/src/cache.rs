/// Flush all cache lines covering [ptr, ptr+len).
pub fn flush_range(ptr: *const u8, len: usize) {
    let mut offset = 0;
    while offset < len {
        // SAFETY: The caller supplies the address range to flush. CLFLUSH is a
        // cache-control primitive; it does not create Rust references to the
        // pointed-to bytes, and each instruction targets one cache-line address.
        unsafe {
            core::arch::asm!(
                "clflush [{}]",
                in(reg) ptr.add(offset),
                options(nostack, preserves_flags),
            );
        }
        offset += 64;
    }
    // SAFETY: MFENCE orders the preceding cache flushes before later memory
    // operations and does not access Rust memory directly.
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }
}
