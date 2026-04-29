/// Read TSC with serialization. Returns (cycles, processor_id).
#[inline(always)]
pub fn rdtscp() -> (u64, u32) {
    let lo: u32;
    let hi: u32;
    let aux: u32;
    // SAFETY: RDTSCP is a CPU instruction used here only to read timing and
    // processor-id registers; it does not dereference memory or modify Rust
    // data. Callers choose whether this low-level timing primitive is available
    // on their host.
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
    // SAFETY: LFENCE is used as a serialization barrier around timing regions;
    // it does not access Rust memory.
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
