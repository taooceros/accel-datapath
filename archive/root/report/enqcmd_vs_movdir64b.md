# ENQCMD vs MOVDIR64B: Shared vs Dedicated Work Queue Performance

**Date**: 2026-03-20
**Status**: Complete for single-process characterization

This report compares:

- **Dedicated WQ** `wq0.0` using **MOVDIR64B**
- **Shared WQ** `wq0.1` using **ENQCMD**

The older March 6 result that reported shared-WQ failure is no longer current. Fresh runs on March 20 completed successfully for both queue types.

## Configuration

| Parameter | Dedicated WQ (wq0.0) | Shared WQ (wq0.1) |
|-----------|----------------------|--------------------|
| Mode | dedicated | shared |
| Submission | MOVDIR64B | ENQCMD |
| WQ Size | 64 | 64 |
| Engines | 2 (engine0.0, engine0.1) | 2 (engine0.2, engine0.3) |
| Group | group0.0 | group0.1 |
| Threshold | 0 | 1 |
| Config | `dsa-config/enqcmd-vs-movdir64b.conf` | same |

- **CPU**: Sapphire Rapids
- **Pinned core**: 2
- **NUMA**: CPU node 0, DSA node 0
- **Iterations**: 5000 per data point
- **Sizes**: 8 B, 64 B, 4 KB
- **Mode**: cold-cache
- **Artifacts**:
  - `hw-eval/results_dedicated.json`
  - `hw-eval/results_shared.json`
  - `hw-eval/graphs/dashboard.html`
  - `hw-eval/graphs/heatmap.html`

## Headline Result

Shared-WQ `ENQCMD` is now operational and competitive with dedicated `MOVDIR64B` for:

- single-op latency
- pipelined batch throughput
- large-message peak bandwidth in batched mode

It still trails dedicated mode in the highest-concurrency non-batched sliding-window paths.

## Single-Op Latency

Median latency in ns:

| Benchmark | Size | Dedicated | Shared | Shared / Dedicated |
|-----------|------|-----------|--------|--------------------|
| noop | - | 675 | 694 | 1.03x |
| memmove | 8 B | 1083 | 1099 | 1.01x |
| memmove | 64 B | 1084 | 1077 | 0.99x |
| memmove | 4 KB | 1737 | 1776 | 1.02x |
| crc_gen | 4 KB | 1302 | 1256 | 0.97x |
| copy_crc | 4 KB | 1251 | 1150 | 0.92x |

Observations:

1. `ENQCMD` does not impose a large single-op penalty in this single-process setup.
2. `noop` overhead differs by only 19 ns between queue types.
3. At 4 KB, shared `crc_gen` and `copy_crc` are slightly better than dedicated in this run.

## Batch Amortization

4 KB `batch_memmove`, median per-op latency:

| Batch Size | Dedicated | Shared |
|------------|-----------|--------|
| 256 | 147.4 ns/op | 169.2 ns/op |
| 1024 | 145.2 ns/op | 157.5 ns/op |

Observations:

1. Both queue types show the same shape: batching crushes per-op submission overhead.
2. Dedicated still has the better floor once heavily amortized.
3. Shared mode pays a modest extra cost under deep batching, roughly 7% to 15% in this run.

## Throughput Comparison

Representative 4 KB points:

| Strategy | Dedicated | Shared |
|----------|-----------|--------|
| `pipelined_batch_b256`, `c=4` | 7.40 Mops/s, 30.30 GB/s | 7.39 Mops/s, 30.29 GB/s |
| `burst_batch_b256`, `c=32` | 6.68 Mops/s, 27.38 GB/s | 6.16 Mops/s, 25.24 GB/s |
| `memmove`, `c=32` | 7.32 Mops/s, 29.97 GB/s | 4.68 Mops/s, 19.16 GB/s |
| `copy_crc`, `c=32` | 6.63 Mops/s, 27.17 GB/s | 4.52 Mops/s, 18.52 GB/s |

Peak message-rate points from the full sweep:

| Queue Type | Best Point | Peak Rate |
|------------|------------|-----------|
| Dedicated | `pipelined_batch_b128`, 8 B, `c=4` | 53.86 Mops/s |
| Shared | `pipelined_batch_b64`, 8 B, `c=8` | 48.00 Mops/s |

Observations:

1. Batched pipelines are effectively tied at 4 KB. This is the strongest evidence that the fixed `ENQCMD` path is viable.
2. Shared mode is somewhat worse for `burst_batch`.
3. Shared mode is substantially worse for plain sliding-window `memmove` and `copy_crc` at high concurrency.
4. The highest peak rate still comes from batched submission, not from single-descriptor sliding windows.

## Interpretation

The updated picture is narrower and more useful than the original blocked/not-blocked question:

1. **Correctness**: shared `ENQCMD` is working end-to-end now.
2. **Latency**: shared and dedicated are nearly identical for single-process single-op latency.
3. **Best-case throughput**: shared can match dedicated when batching is aggressive enough.
4. **Contention/backpressure sensitivity**: shared falls behind in non-batched high-concurrency paths, which is consistent with `ENQCMD` retry behavior showing up under heavier queue pressure.

## Practical Takeaways

- Use **dedicated + MOVDIR64B** when you want the strongest peak throughput and the simplest submission path.
- Use **shared + ENQCMD** when you need queue sharing or isolation and can structure work around batching.
- If the workload is naturally batchable, the performance gap is small enough that shared queues look viable.
- If the workload depends on large non-batched sliding windows, dedicated mode still has a clear advantage on this machine.

## Next Questions

1. Measure shared-vs-dedicated under actual multi-process contention, not just one submitter.
2. Instrument `ENQCMD` retry counts so the throughput loss in sliding-window mode can be tied to accept/retry behavior directly.
3. Expand the sweep back to larger sizes to see whether the near-parity in batched 4 KB mode holds at 16 KB to 1 MB.
