use std::time::Instant;

use hw_eval::iax;
use hw_eval::submit::{cycles_to_ns, flush_range, lfence, rdtscp, WqPortal};

use crate::report::{compute_stats, LatencyResult, ThroughputResult};

pub(crate) fn run_iax_benchmarks(
    wq: &WqPortal,
    sizes: &[usize],
    iterations: usize,
    max_concurrency: usize,
    tsc_freq: u64,
    cold: bool,
    json: bool,
    latency_results: &mut Vec<LatencyResult>,
    throughput_results: &mut Vec<ThroughputResult>,
) {
    bench_noop_latency_iax(wq, iterations, tsc_freq, json, latency_results);

    bench_single_op_latency_iax_crc64(wq, sizes, iterations, tsc_freq, cold, json, latency_results);

    for &size in sizes {
        bench_burst_iax_crc64(
            wq,
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
        );
    }

    for &size in sizes {
        bench_sliding_window_iax_crc64(
            wq,
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
        );
    }
}

fn bench_noop_latency_iax(
    wq: &WqPortal,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    let mut desc = iax::IaxHwDesc::default();
    let mut comp = iax::IaxCompletionRecord::default();
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..100 {
        iax::reset_completion(&mut comp);
        desc.fill_noop();
        desc.set_completion(&mut comp);
        unsafe { wq.submit_iax(&desc) };
        iax::poll_completion(&comp);
    }

    for _ in 0..iterations {
        iax::reset_completion(&mut comp);
        desc.fill_noop();
        desc.set_completion(&mut comp);

        lfence();
        let start = rdtscp().0;
        unsafe { wq.submit_iax(&desc) };
        iax::poll_completion(&comp);
        let end = rdtscp().0;
        latencies.push(end - start);
    }

    latencies.sort_unstable();
    let cyc = compute_stats(&latencies);
    let ns_vec: Vec<u64> = latencies
        .iter()
        .map(|&c| cycles_to_ns(c, tsc_freq))
        .collect();
    let ns = compute_stats(&ns_vec);

    if !json {
        println!("\n=== Single-op latency: noop (iax) ===");
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8}",
            "min_cyc", "med_cyc", "mean_cyc", "min_ns", "p99_ns", "p999_ns", "cv"
        );
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8.3}",
            cyc.min, cyc.median, cyc.mean, ns.min, ns.p99, ns.p999, cyc.cv
        );
    }

    results.push(LatencyResult {
        benchmark: "noop".into(),
        size: None,
        batch_size: None,
        cycles: cyc,
        ns,
    });
}

fn panic_iax_failure(
    op_name: &str,
    comp: &iax::IaxCompletionRecord,
    status: u8,
    size: usize,
    context: &str,
) -> ! {
    panic!(
        "IAX {} failed: status={:#x} error={:#x} invalid_flags={:#x} size={} {}",
        op_name,
        status,
        iax::completion_error_code(comp),
        iax::completion_invalid_flags(comp),
        size,
        context
    );
}

fn bench_single_op_latency_iax_crc64(
    wq: &WqPortal,
    sizes: &[usize],
    iterations: usize,
    tsc_freq: u64,
    cold: bool,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    if !json {
        println!("\n=== Single-op latency: crc64 (iax) ===");
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8}",
            "size", "min_cyc", "med_cyc", "min_ns", "med_ns", "mean_ns", "p99_ns", "cv"
        );
    }

    for &size in sizes {
        let src = vec![0xABu8; size];
        let mut desc = iax::IaxHwDesc::default();
        let mut comp = iax::IaxCompletionRecord::default();
        let mut latencies = Vec::with_capacity(iterations);

        for _ in 0..100 {
            iax::reset_completion(&mut comp);
            desc.fill_crc64(src.as_ptr(), size as u32);
            desc.set_completion(&mut comp);
            unsafe { wq.submit_iax(&desc) };
            iax::poll_completion(&comp);
        }

        for _ in 0..iterations {
            if cold {
                flush_range(src.as_ptr(), size);
            }

            iax::reset_completion(&mut comp);
            desc.fill_crc64(src.as_ptr(), size as u32);
            desc.set_completion(&mut comp);

            lfence();
            let start = rdtscp().0;
            unsafe { wq.submit_iax(&desc) };
            let status = iax::poll_completion(&comp);
            if status == iax::IAX_COMP_PAGE_FAULT_IR {
                iax::touch_fault_page(&comp);
                iax::reset_completion(&mut comp);
                desc.fill_crc64(src.as_ptr(), size as u32);
                desc.set_completion(&mut comp);
                unsafe { wq.submit_iax(&desc) };
                let retry = iax::poll_completion(&comp);
                if retry != iax::IAX_COMP_SUCCESS {
                    panic_iax_failure("crc64", &comp, retry, size, "after page-fault retry");
                }
            } else if status != iax::IAX_COMP_SUCCESS {
                panic_iax_failure("crc64", &comp, status, size, "");
            }
            let end = rdtscp().0;
            latencies.push(end - start);
        }

        latencies.sort_unstable();
        let cyc = compute_stats(&latencies);
        let ns_vec: Vec<u64> = latencies
            .iter()
            .map(|&c| cycles_to_ns(c, tsc_freq))
            .collect();
        let ns = compute_stats(&ns_vec);

        if !json {
            println!(
                "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8.3}",
                size, cyc.min, cyc.median, ns.min, ns.median, ns.mean, ns.p99, cyc.cv
            );
        }

        results.push(LatencyResult {
            benchmark: "crc64".into(),
            size: Some(size),
            batch_size: None,
            cycles: cyc,
            ns,
        });
    }
}

fn bench_burst_iax_crc64(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_burst: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!("\n=== Burst throughput: crc64 (iax, size={}) ===", size);
        println!("{:>6} {:>14} {:>14}", "burst", "ops/sec", "bandwidth_MB/s");
    }

    for burst_size in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&b| b <= max_burst)
    {
        let mut descs: Vec<iax::IaxHwDesc> =
            (0..burst_size).map(|_| iax::IaxHwDesc::default()).collect();
        let mut comps: Vec<iax::IaxCompletionRecord> = (0..burst_size)
            .map(|_| iax::IaxCompletionRecord::default())
            .collect();
        let srcs: Vec<Vec<u8>> = (0..burst_size).map(|_| vec![0xABu8; size]).collect();

        let start = Instant::now();
        for _ in 0..iterations {
            for i in 0..burst_size {
                iax::reset_completion(&mut comps[i]);
                descs[i].fill_crc64(srcs[i].as_ptr(), size as u32);
                descs[i].set_completion(&mut comps[i]);
                unsafe { wq.submit_iax(&descs[i]) };
            }

            for i in 0..burst_size {
                let status = iax::poll_completion(&comps[i]);
                if status == iax::IAX_COMP_PAGE_FAULT_IR {
                    iax::touch_fault_page(&comps[i]);
                    iax::drain_completions(&comps[i + 1..]);
                    iax::reset_completion(&mut comps[i]);
                    descs[i].fill_crc64(srcs[i].as_ptr(), size as u32);
                    descs[i].set_completion(&mut comps[i]);
                    unsafe { wq.submit_iax(&descs[i]) };
                    let retry = iax::poll_completion(&comps[i]);
                    if retry != iax::IAX_COMP_SUCCESS {
                        iax::drain_completions(&comps);
                        panic!(
                            "IAX crc64 failed: status={:#x} error={:#x} invalid_flags={:#x} size={} burst={} after page-fault retry",
                            retry,
                            iax::completion_error_code(&comps[i]),
                            iax::completion_invalid_flags(&comps[i]),
                            size,
                            burst_size
                        );
                    }
                    continue;
                }
                if status != iax::IAX_COMP_SUCCESS {
                    iax::drain_completions(&comps[i + 1..]);
                    panic!(
                        "IAX crc64 failed: status={:#x} error={:#x} invalid_flags={:#x} size={} burst={}",
                        status,
                        iax::completion_error_code(&comps[i]),
                        iax::completion_invalid_flags(&comps[i]),
                        size,
                        burst_size
                    );
                }
            }
        }

        let elapsed = start.elapsed();
        let total_ops = iterations * burst_size;
        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
        let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!("{:>6} {:>14.0} {:>14.1}", burst_size, ops_per_sec, bw_mb);
        }

        results.push(ThroughputResult {
            benchmark: "burst_crc64".into(),
            size,
            concurrency: burst_size,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}

fn bench_sliding_window_iax_crc64(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!(
            "\n=== Sliding window throughput: crc64 (iax, size={}) ===",
            size
        );
        println!("{:>6} {:>14} {:>14}", "conc", "ops/sec", "bandwidth_MB/s");
    }

    for concurrency in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&c| c <= max_concurrency)
    {
        let window = concurrency.min(iterations);
        let mut descs: Vec<iax::IaxHwDesc> = (0..concurrency)
            .map(|_| iax::IaxHwDesc::default())
            .collect();
        let mut comps: Vec<iax::IaxCompletionRecord> = (0..concurrency)
            .map(|_| iax::IaxCompletionRecord::default())
            .collect();
        let srcs: Vec<Vec<u8>> = (0..concurrency).map(|_| vec![0xABu8; size]).collect();

        for i in 0..window {
            iax::reset_completion(&mut comps[i]);
            descs[i].fill_crc64(srcs[i].as_ptr(), size as u32);
            descs[i].set_completion(&mut comps[i]);
            unsafe { wq.submit_iax(&descs[i]) };
        }

        let start = Instant::now();
        let mut issued = window;
        let mut completed = 0usize;
        let mut slot = 0usize;

        while completed < iterations {
            let status = iax::poll_completion(&comps[slot]);
            if status == iax::IAX_COMP_PAGE_FAULT_IR {
                iax::touch_fault_page(&comps[slot]);
                iax::reset_completion(&mut comps[slot]);
                descs[slot].fill_crc64(srcs[slot].as_ptr(), size as u32);
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit_iax(&descs[slot]) };
                continue;
            }
            if status != iax::IAX_COMP_SUCCESS {
                iax::drain_completions(&comps);
                panic!(
                    "IAX crc64 failed: status={:#x} error={:#x} invalid_flags={:#x} size={} conc={}",
                    status,
                    iax::completion_error_code(&comps[slot]),
                    iax::completion_invalid_flags(&comps[slot]),
                    size,
                    concurrency
                );
            }
            completed += 1;

            if issued < iterations {
                iax::reset_completion(&mut comps[slot]);
                descs[slot].fill_crc64(srcs[slot].as_ptr(), size as u32);
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit_iax(&descs[slot]) };
                issued += 1;
            }

            slot = (slot + 1) % window;
        }

        iax::drain_completions(&comps);

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let bw_mb = (iterations * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!("{:>6} {:>14.0} {:>14.1}", concurrency, ops_per_sec, bw_mb);
        }

        results.push(ThroughputResult {
            benchmark: "crc64".into(),
            size,
            concurrency,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}
