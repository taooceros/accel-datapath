# Code Context

## Files Retrieved
1. `idxd-sys/src/lib.rs` (lines 1-52) - public idxd-sys exports; shows raw UAPI plus helper modules being re-exported.
2. `idxd-sys/src/descriptor.rs` (lines 1-387) - DSA aligned wrappers, accessors, opcode/status re-exports, and descriptor fill helpers.
3. `idxd-sys/src/completion.rs` (lines 1-82) - DSA completion polling/reset/drain/fault-page helpers.
4. `idxd-sys/src/iax.rs` (lines 1-349) - IAX aligned wrappers, constants, fill/accessor helpers, completion helpers, and software CRC helpers.
5. `idxd-sys/src/portal.rs` (lines 1-286) - portal mmap, WQ mode detection, raw descriptor submission, and typed DSA/IAX submission wrappers.
6. `idxd-sys/src/cache.rs` (lines 1-22) - cache-line flush helper.
7. `idxd-sys/src/timing.rs` (lines 1-64) - timing/instruction and TSC conversion helpers.
8. `idxd-sys/src/topology.rs` (lines 1-48) - CPU affinity and NUMA sysfs helpers.
9. `idxd-rust/src/direct_memmove.rs` (lines 1-220) - primary DSA memmove call sites for descriptor/completion helpers.
10. `idxd-rust/src/async_direct.rs` (lines 119-155) - async backend uses `DsaHwDesc`, `EnqcmdSubmission`, `WqPortal` typed submission.
11. `idxd-rust/src/async_direct/operation.rs` (lines 90-144) - async retry path uses `touch_fault_page` and descriptor access.
12. `idxd-rust/src/iax_crc64.rs` (lines 1-90, 360-489) - IAX CRC64 operation owns descriptor/completion, polling/reset/fault helpers, and status constants.
13. `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs` (lines 1-190, 220-234) - benchmark hot path uses raw-ish helpers directly.
14. `idxd-rust/src/validation.rs` (lines 1-16, 480-573) - completion status constants used in higher-level classification.

## Key Code

- Keep in `idxd-sys`: generated UAPI module/re-export (`idxd-sys/src/lib.rs:6-21`) and ABI-preserving aligned wrapper storage (`BindgenDsaHwDesc`, `BindgenDsaCompletionRecord`, `BindgenIaxHwDesc`, `BindgenIaxCompletionRecord`) because they wrap bindgen layout/alignment (`idxd-sys/src/descriptor.rs:4-41`, `idxd-sys/src/iax.rs:4-42`).
- Move candidates to `idxd-rust`:
  - DSA descriptor operation builders/accessors: `DsaHwDesc::{opcode, flags, completion_addr, src_addr, dst_addr, xfer_size, set_completion, fill_memmove, fill_memmove_to_memory, fill_crc_gen, fill_copy_crc, fill_memfill, fill_compare, fill_batch, fill_noop}` (`idxd-sys/src/descriptor.rs:143-386`). Suggested destination: `idxd_rust::raw_dsa` or `idxd_rust::descriptor::dsa`; keep type name `DsaHwDesc` if wrapper moves wholesale, or expose `DsaDescriptorExt` if wrapper stays in sys.
  - DSA completion helpers: `poll_completion`, `reset_completion`, `reset_completion_status`, `drain_completions`, `touch_fault_page` (`idxd-sys/src/completion.rs:4-82`). Suggested destination: `idxd_rust::completion::dsa` (`poll_dsa_completion`, `reset_dsa_completion`, `touch_dsa_fault_page`). Current call sites include `idxd-rust/src/direct_memmove.rs:3-6, 88-104, 168-187` and `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs:8-11, 138-188, 228-234`.
  - DSA opcode/status/flag aliases (`idxd-sys/src/descriptor.rs:105-124`). Suggested destination: `idxd_rust::raw_dsa::constants`; call sites currently import status constants in `idxd-rust/src/direct_memmove.rs:3-6`, `idxd-rust/src/validation.rs:4-6, 496-568`, and benchmark `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs:8-10, 165-172`.
  - IAX descriptor builders/accessors and CRC64 raw offset helpers: `IaxHwDesc::{opcode, flags, completion_addr, src1_addr, src1_size, crc64_flags, crc64_poly, set_completion, fill_noop, fill_crc64}` plus `IAX_OPCODE_CRC64`, CRC64 offsets/poly (`idxd-sys/src/iax.rs:59-70, 80-210`). Suggested destination: `idxd_rust::raw_iax` or `idxd_rust::iax_crc64::descriptor`.
  - IAX completion helpers/accessors: `IaxCompletionRecord::{status,error_code,invalid_flags,fault_addr,crc64}`, `poll_iax_completion`, `reset_iax_completion`, `drain_iax_completions`, `touch_iax_fault_page` (`idxd-sys/src/iax.rs:213-326`). Suggested destination: `idxd_rust::completion::iax` or `idxd_rust::iax_crc64::completion`. Current call site: `idxd-rust/src/iax_crc64.rs:3-6, 367-390, 409-489`.
  - Software CRC helpers `crc16_t10dif`, `crc64_t10dif_field` (`idxd-sys/src/iax.rs:328-345`). Suggested destination: `idxd_rust::iax_crc64::software` or `idxd_rust::crc::t10dif`. Current call sites: `idxd-rust/src/bin/idxd_representative_bench.rs:12,420,1030` and `idxd-rust/src/bin/live_idxd_op.rs:12,269,585`.
  - Typed portal wrappers and policy: `EnqcmdSubmission`, `WqPortal::open`, `is_dedicated`, typed `submit_*(&DsaHwDesc)`, `submit_iax`, spin-until-accepted `submit_desc64`, and `detect_wq_mode` (`idxd-sys/src/portal.rs:8-18, 33-67, 122-242, 256-286`). Suggested destination: `idxd_rust::portal`/`idxd_rust::submission`; leave only raw unsafe `submit_movdir64b_desc64` and `submit_enqcmd_desc64` in sys if desired (`idxd-sys/src/portal.rs:69-120`). Current call sites: `idxd-rust/src/async_direct.rs:119-155`, `idxd-rust/src/direct_memmove.rs:79-90,117-127`, `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs:36-50`.
  - Host utility helpers: `flush_range` (`idxd-sys/src/cache.rs:1-22`), timing helpers `rdtscp`, `lfence`, `tsc_frequency_hz`, `cycles_to_ns` (`idxd-sys/src/timing.rs:1-64`), topology helpers `pin_to_core`, `current_core`, `cpu_numa_node`, `device_numa_node` (`idxd-sys/src/topology.rs:3-48`). Suggested destination: `idxd_rust::host::{cache,timing,topology}` or benchmark-local modules if only bins use them.

## Architecture

`idxd-sys` currently mixes three layers: generated `linux/idxd.h` UAPI, ABI/alignment wrappers around bindgen records, and higher-level Rust operation conveniences. `idxd-rust` constructs operation state (`DirectMemmoveState`, `IaxCrc64State`) with `DsaHwDesc`/`IaxHwDesc` and `*CompletionRecord`, uses sys fill/reset/poll helpers to submit via `WqPortal`, then classifies completions in `idxd-rust` validation code. This means operation semantics (memmove, CRC64, retry/fault policy, spin timeout marker 0xFF, benchmark polling loops) are already in `idxd-rust`, but many building blocks live in `idxd-sys`.

## Start Here

Open `idxd-sys/src/lib.rs` first to see the exported boundary, then `idxd-sys/src/descriptor.rs` and `idxd-rust/src/direct_memmove.rs` together: they show the clearest sys-vs-rust split for DSA descriptor construction, polling, retry, and completion classification.

## Pi-intercom handoff

No safe intercom target was provided/available; findings written locally only.
