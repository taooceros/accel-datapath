use std::path::Path;

/// Pin the calling thread to the specified CPU core.
pub fn pin_to_core(core: usize) -> std::io::Result<usize> {
    // SAFETY: The cpuset is initialized before use and passed to libc with its
    // exact size. OS affinity failures are returned as raw `last_os_error`.
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
    // SAFETY: `sched_getcpu` has no Rust-side memory preconditions and returns
    // the OS-reported current CPU id.
    unsafe { libc::sched_getcpu() as usize }
}

/// Get the NUMA node for a CPU core.
pub fn cpu_numa_node(core: usize) -> Option<usize> {
    let cpu_dir = format!("/sys/devices/system/cpu/cpu{}", core);
    for entry in std::fs::read_dir(&cpu_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_str()?;
        if let Some(node) = name.strip_prefix("node") {
            return node.parse().ok();
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
