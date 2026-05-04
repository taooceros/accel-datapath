# Raw NUMA0 hw-eval thread sweep rerun

## Summary

Reran the raw `hw-eval` DSA shared-WQ NUMA0 thread sweep for `/dev/dsa/wq0.1` using the existing launcher pattern:

```bash
./tools/build/dsa_launcher numactl --cpunodebind=0 --membind=0 \
  ./target/release/hw-eval \
  --accel dsa \
  --device /dev/dsa/wq0.1 \
  --json \
  --iterations 3000 \
  --sizes 128,256,1024,4096 \
  --max-concurrency 128 \
  --threads <N>
```

- Built release binary first with `cargo build --release -p hw-eval`.
- Created missing NUMA0 raw artifacts for threads `1,2,4,8,16`.
- Refreshed existing `32` and `64` artifacts for consistency; these files were intentionally replaced.
- `docs/report/benchmarking/shared_thread_sweep_numa0/failures.log` is empty.

Note: `hw-eval` only emits `memmove_mt_t<N>` when `--threads > 1`; for `threads=1`, the equivalent single-submit-thread direct/no-batch row is `memmove`.

## Artifacts

All artifacts are under `docs/report/benchmarking/shared_thread_sweep_numa0/`:

| Threads | JSON | Raw log |
|---:|---|---|
| 1 | `hw_eval_wq0_1_numa0_threads1.json` | `hw_eval_wq0_1_numa0_threads1.raw` |
| 2 | `hw_eval_wq0_1_numa0_threads2.json` | `hw_eval_wq0_1_numa0_threads2.raw` |
| 4 | `hw_eval_wq0_1_numa0_threads4.json` | `hw_eval_wq0_1_numa0_threads4.raw` |
| 8 | `hw_eval_wq0_1_numa0_threads8.json` | `hw_eval_wq0_1_numa0_threads8.raw` |
| 16 | `hw_eval_wq0_1_numa0_threads16.json` | `hw_eval_wq0_1_numa0_threads16.raw` |
| 32 | `hw_eval_wq0_1_numa0_threads32.json` | `hw_eval_wq0_1_numa0_threads32.raw` |
| 64 | `hw_eval_wq0_1_numa0_threads64.json` | `hw_eval_wq0_1_numa0_threads64.raw` |

## c=128 summary

| Thread | Size | Benchmark row | Raw Mops/s | Raw GB/s |
|---:|---:|---|---:|---:|
| 1 | 128 | `memmove` | 5.049 | 0.646 |
| 1 | 256 | `memmove` | 5.057 | 1.294 |
| 1 | 1024 | `memmove` | 4.999 | 5.119 |
| 1 | 4096 | `memmove` | 4.576 | 18.742 |
| 2 | 128 | `memmove_mt_t2` | 8.367 | 1.071 |
| 2 | 256 | `memmove_mt_t2` | 8.499 | 2.176 |
| 2 | 1024 | `memmove_mt_t2` | 6.109 | 6.256 |
| 2 | 4096 | `memmove_mt_t2` | 5.273 | 21.597 |
| 4 | 128 | `memmove_mt_t4` | 15.820 | 2.025 |
| 4 | 256 | `memmove_mt_t4` | 10.579 | 2.708 |
| 4 | 1024 | `memmove_mt_t4` | 13.026 | 13.339 |
| 4 | 4096 | `memmove_mt_t4` | 6.883 | 28.191 |
| 8 | 128 | `memmove_mt_t8` | 29.229 | 3.741 |
| 8 | 256 | `memmove_mt_t8` | 31.001 | 7.936 |
| 8 | 1024 | `memmove_mt_t8` | 22.726 | 23.271 |
| 8 | 4096 | `memmove_mt_t8` | 7.030 | 28.795 |
| 16 | 128 | `memmove_mt_t16` | 44.259 | 5.665 |
| 16 | 256 | `memmove_mt_t16` | 41.640 | 10.660 |
| 16 | 1024 | `memmove_mt_t16` | 24.252 | 24.834 |
| 16 | 4096 | `memmove_mt_t16` | 6.951 | 28.472 |
| 32 | 128 | `memmove_mt_t32` | 39.415 | 5.045 |
| 32 | 256 | `memmove_mt_t32` | 39.370 | 10.079 |
| 32 | 1024 | `memmove_mt_t32` | 21.618 | 22.136 |
| 32 | 4096 | `memmove_mt_t32` | 6.770 | 27.730 |
| 64 | 128 | `memmove_mt_t64` | 32.864 | 4.207 |
| 64 | 256 | `memmove_mt_t64` | 34.554 | 8.846 |
| 64 | 1024 | `memmove_mt_t64` | 21.960 | 22.487 |
| 64 | 4096 | `memmove_mt_t64` | 6.665 | 27.299 |

## Validation

- `cargo build --release -p hw-eval` completed with 0 errors and 1 existing dead-code warning.
- All seven JSON files parse with `jq`.
- `memmove_mt_t<N>` c=128 rows exist for threads `2,4,8,16,32,64` at sizes `128,256,1024,4096`.
- Thread `1` has c=128 `memmove` rows at sizes `128,256,1024,4096`; `hw-eval` does not emit a `memmove_mt_t1` row by design.
