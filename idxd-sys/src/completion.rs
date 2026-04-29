use crate::descriptor::{DsaCompletionRecord, DSA_COMP_NONE, DSA_COMP_STATUS_MASK};
use std::ptr;

/// Poll a completion record until status is non-zero.
/// Returns the status byte.
#[inline(always)]
pub fn poll_completion(comp: &DsaCompletionRecord) -> u8 {
    const MAX_SPINS: u64 = 2_000_000_000; // ~1s at ~2GHz
    let mut spins: u64 = 0;
    loop {
        // Volatile read — hardware writes this field, and `status()` keeps that
        // volatile load adjacent to the generated completion-record accessor.
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
    // SAFETY: `comp` is an initialized completion-record wrapper. Zeroing the
    // entire record restores the hardware contract's `DSA_COMP_NONE` state
    // before the caller reuses it for a new descriptor.
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
        // SAFETY: The completion record reports the faulting virtual address.
        // The raw recovery contract is a volatile read+write touch of that page
        // so the OS installs a writable PTE before a caller retries DMA.
        unsafe {
            let p = addr as *mut u8;
            // Write touch to ensure writable PTE.
            ptr::write_volatile(p, ptr::read_volatile(p));
        }
    }
}
