# Test Coverage Audit & Plan

**Status**: cancelled on 2026-04-15

Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

## Coverage Audit

### Existing Tests

| Test File | Target | HW Required | What It Covers |
|-----------|--------|-------------|----------------|
| `test/test_task_queues.cpp` | `test_task_queues` | No (MockHwContext) | All 7 TaskQueue implementations: basic push/poll, empty queue, multiple ops, concurrent push, concurrent push+poll, capacity limits, FIFO ordering, stress cycles, callbacks |
| `test/test_mirrored_ring.cpp` | `test_mirrored_ring` | Yes (DSA) | MirroredRing memory aliasing (no HW), data_move through MirroredRingSubmitter, cross-boundary batch, sustained ring cycling |

### Build Targets Declared But Source Missing

| Target | Expected File | Status |
|--------|---------------|--------|
| `test_stdexec_integration` | `test/test_stdexec_integration.cpp` | **MISSING** |
| `test_utilities` | `test/test_utilities.cpp` | **MISSING** |

### Coverage Gap Matrix

| Module | Header | Key Exports | Test Coverage | Priority |
|--------|--------|-------------|---------------|----------|
| **PollingRunLoop** | `run_loop.hpp` | `PollingRunLoop`, `Scheduler`, `Task` | **ZERO** (used indirectly by mirrored_ring test) | **P0** |
| **DsaScheduler** | `scheduler.hpp` | `DsaScheduler`, `ScheduleSender`, `ScheduleOperation` | **ZERO** | **P0** |
| **sync_wait** | `sync_wait.hpp` | `sync_wait_threaded()`, `wait_start()` | **ZERO** (used indirectly) | **P0** |
| **DsaError** | `error.hpp` | `DsaError`, `DsaSubmitError`, `DsaInitError`, `dsa_status_to_string()`, `dsa_opcode_to_string()` | **ZERO** | **P0** |
| **descriptor_fill** | `descriptor_fill.hpp` | `fill_data_move`, `fill_mem_fill`, `fill_compare`, `fill_compare_value`, `fill_dualcast`, `fill_crc_gen`, `fill_copy_crc`, `fill_cache_flush` | **ZERO** | **P1** |
| **OperationBase** | `operation_base.hpp` | `OperationFacade`, `OperationBase`, `HwContext` concept | Partial (via test_task_queues) | P2 |
| **DsaOperationMixin** | `operations/operation_base_mixin.hpp` | `DsaOperationMixin`, `adjust_for_page_fault()`, `DsaOpSender`, page fault counter | **ZERO** | **P0** |
| **Operation senders** | `operations/*.hpp` | 8 operation types (data_move, mem_fill, compare, compare_value, dualcast, crc_gen, copy_crc, cache_flush) | data_move only (via mirrored_ring, requires HW) | P2 (HW-dependent) |
| **DsaSink** | `dsa_sink.hpp` | `DsaSink` concept | N/A (concept, tested via concrete types) | - |
| **DsaFacade** | `dsa_facade.hpp` | `DsaFacade`, `DsaProxy`, `make_dsa_proxy()` | **ZERO** | **P1** |
| **submit** | `submit.hpp` | `DetachedReceiver`, `submit()` | **ZERO** | **P1** |
| **batch** | `batch.hpp` | `BatchOperation`, `BatchSender`, `dsa_batch()` | Partial (via mirrored_ring, requires HW) | P2 |
| **MirroredRing** | `mirrored_ring.hpp` | `MirroredRing` class | Covered (test_mirrored_ring) | Done |
| **TaskQueues** | `task_queue.hpp` | 7 queue types, concepts, locks | Covered (test_task_queues) | Done |
| **MockDsa** | `mock_dsa.hpp` | `MockHwContext`, `MockOperation`, `MockDsaBase`, aliases | Partial (used by tests, not directly tested) | P2 |
| **DsaOperationBase** | `dsa_operation_base.hpp` | `DsaOperationBase`, alignment logic | **ZERO** | **P1** |
| **descriptor_submitter** | `descriptor_submitter.hpp` | `DescriptorSubmitter` concept, `DirectSubmitter`, `StagingSubmitter`, `FixedRingSubmitter`, `RingSubmitter`, `MirroredRingSubmitter`, `alloc_aligned`, `round_up_pow2` | **ZERO** (unit-level; integration via mirrored_ring test) | **P1** |

## Test Plan

### Test File 1: `test/test_utilities.cpp` (No HW Required)

Tests pure utility functions and error types with no hardware dependency.

#### 1.1 `round_up_pow2`

| Case | Input | Expected | Edge Case |
|------|-------|----------|-----------|
| Zero | 0 | 1 | Boundary |
| One | 1 | 1 | Identity |
| Power of 2 | 16 | 16 | Identity |
| Non-power | 17 | 32 | Round up |
| Large | 255 | 256 | Near boundary |
| Already max 32-bit | 4096 | 4096 | Identity |

#### 1.2 `alloc_aligned<T>`

| Case | Description | Edge Case |
|------|-------------|-----------|
| Basic allocation | Allocate 10 ints, 64-byte aligned | Verify alignment |
| Zero-initialized | All bytes should be 0 | Memory safety |
| Large alignment | 4096-byte alignment | Page alignment |
| Single element | count=1 | Minimum allocation |

#### 1.3 `dsa_status_to_string`

| Case | Input | Expected |
|------|-------|----------|
| Success | `DSA_COMP_SUCCESS` | "Success" |
| Page fault | `DSA_COMP_PAGE_FAULT_NOBOF` | "Page fault without block-on-fault" |
| Bad opcode | `DSA_COMP_BAD_OPCODE` | "Invalid opcode" |
| Unknown | 0xFF | "Unknown error" |
| None | `DSA_COMP_NONE` | "No status (operation not complete)" |

#### 1.4 `dsa_opcode_to_string`

| Case | Input | Expected |
|------|-------|----------|
| Memmove | `DSA_OPCODE_MEMMOVE` | "MEMMOVE" |
| Batch | `DSA_OPCODE_BATCH` | "BATCH" |
| CRC | `DSA_OPCODE_CRCGEN` | "CRCGEN" |
| Unknown | 0xFF | "UNKNOWN" |

#### 1.5 `DsaError` construction and accessors

| Case | Description | Edge Case |
|------|-------------|-----------|
| From completion record | Status + comp + opcode + context | Full error info |
| From message string | General error | String-only constructor |
| From status only | Status code only | Minimal constructor |
| `what()` contains status | Error string includes hex status | Message formatting |
| `full_report()` includes stacktrace | Report contains stacktrace text | Stacktrace capture |
| Accessors | `status()`, `opcode()`, `bytes_completed()`, `fault_addr()` | All return correct values |

#### 1.6 `DsaSubmitError` / `DsaInitError`

| Case | Description |
|------|-------------|
| Reason only | Message includes reason |
| Reason + error_code | error_code() returns correct value |
| Inherits DsaError | `what()` works via base |

#### 1.7 `DsaOperationBase` alignment

| Case | Description | Edge Case |
|------|-------------|-----------|
| desc_ptr() 64-byte aligned | `reinterpret_cast<uintptr_t>(desc_ptr()) % 64 == 0` | Required by HW |
| comp_ptr() 32-byte aligned | `reinterpret_cast<uintptr_t>(comp_ptr()) % 32 == 0` | Required by HW |
| Multiple instances | Each instance has independently aligned ptrs | No sharing |
| Stack allocated | Alignment holds for stack objects | Stack alignment |
| Heap allocated | Alignment holds for `new` objects | Heap alignment |

### Test File 2: `test/test_stdexec_integration.cpp` (No HW Required)

Tests stdexec sender/receiver plumbing using MockDsa (no real DSA hardware).

#### 2.1 `PollingRunLoop` — task scheduling

| Case | Description | Edge Case |
|------|-------------|-----------|
| Schedule + run | Schedule a task, run loop, verify it executes | Basic functionality |
| Multiple tasks | Schedule 3 tasks, all execute in order | FIFO ordering |
| Finish stops loop | `finish()` causes `run()` to return | Shutdown |
| Reset after finish | `reset()` allows `run()` again | Reuse |
| Poll function called | Custom poll function invoked during run | DSA integration point |
| Default-constructible PollFunc | `PollingRunLoop<>` with no poll func | Edge case |
| Stop token respected | Receiver with stop token gets `set_stopped` | Cancellation |

#### 2.2 `PollingRunLoop::Scheduler` — stdexec scheduler concept

| Case | Description |
|------|-------------|
| Satisfies scheduler concept | `static_assert(stdexec::scheduler<...>)` |
| `schedule()` returns sender | Sender completes with `set_value()` |
| Equality comparison | Same loop -> equal schedulers |
| Forward progress guarantee | Returns `parallel` |

#### 2.3 `DsaScheduler` — with MockDsa

| Case | Description | Edge Case |
|------|-------------|-----------|
| Schedule completes on poll | `schedule()` sender completes after `poll()` | Immediate completion |
| Equality | Same DSA -> equal schedulers | Identity |
| Completion scheduler env | Sender env advertises correct scheduler | stdexec integration |

#### 2.4 `sync_wait_threaded` — background completion

| Case | Description | Edge Case |
|------|-------------|-----------|
| Value sender | `sync_wait_threaded(just(42))` returns 42 | Basic |
| Void sender | `sync_wait_threaded(just())` returns empty tuple | Void result |
| Error sender | Sender that calls `set_error` | Exception propagation |
| Cross-thread | Completion on different thread | Semaphore signaling |

#### 2.5 `wait_start` — inline polling completion

| Case | Description | Edge Case |
|------|-------------|-----------|
| Value completion | `wait_start(sender, loop)` returns result | Basic |
| Loop runs until done | Loop polls until sender completes | Polling integration |

#### 2.6 `DsaProxy` (type-erased DSA)

| Case | Description | Edge Case |
|------|-------------|-----------|
| Construct from MockDsa | `make_dsa_proxy<MockDsa>()` | Type erasure |
| `submit()` dispatches | Calls through to concrete submit | Proxy dispatch |
| `poll()` dispatches | Calls through to concrete poll | Proxy dispatch |
| Bool conversion | Empty proxy is false, populated is true | Default state |

#### 2.7 `DetachedReceiver` / `submit()`

| Case | Description |
|------|-------------|
| Accepts set_value | No crash on value completion |
| Accepts set_error | No crash on error completion |
| Accepts set_stopped | No crash on stopped |

#### 2.8 `descriptor_fill` functions

| Case | Description | Verification |
|------|-------------|-------------|
| `fill_data_move` | Fills opcode, flags, src, dst, size | Check all fields |
| `fill_mem_fill` | Fills opcode, flags, dst, size, pattern | Check pattern field |
| `fill_compare` | Fills src1, src2, size | Check src2_addr |
| `fill_compare_value` | Fills src, size, comp_pattern | Check comp_pattern |
| `fill_dualcast` | Fills src, dst1, dst2, size | Check dest2 field |
| `fill_crc_gen` | Fills src, size, seed | Check crc_seed |
| `fill_copy_crc` | Fills src, dst, size, seed | Check all CRC fields |
| `fill_cache_flush` | Fills dst, size | Check opcode |
| Flags correct | Each op has RCR+CRAV (some +CC) | Bit verification |

#### 2.9 `adjust_for_page_fault`

| Case | Description | Edge Case |
|------|-------------|-----------|
| MEMMOVE adjustment | src_addr, dst_addr advanced; xfer_size reduced | Partial completion |
| MEMFILL adjustment | dst_addr advanced; xfer_size reduced | Write-only op |
| COMPARE adjustment | Both src_addr and src2_addr advanced | Dual-source op |
| COMPVAL adjustment | src_addr advanced | Single-source op |
| DUALCAST adjustment | src, dst, dest2 all advanced | Three-pointer op |
| CRCGEN adjustment | src_addr advanced, crc_seed updated from comp.crc_val | Seed continuation |
| COPY_CRC adjustment | Both addresses advanced, crc_seed updated | Combined op |
| CFLUSH adjustment | dst_addr advanced | Flush op |
| Write fault | `comp.status & DSA_COMP_STATUS_WRITE` set | Write vs read |
| Zero bytes_completed | No adjustment needed | Edge case |
| Unknown opcode | No crash, no-op | Default case |

#### 2.10 Page fault retry counter

| Case | Description |
|------|-------------|
| Initial value is 0 | `get_page_fault_retries() == 0` |
| Increment works | After increment, counter advances |
| Reset works | `reset_page_fault_retries()` resets to 0 |

#### 2.11 Operation senders via MockDsa (end-to-end, no HW)

MockDsa's `submit()` pushes to task queue; MockHwContext always returns completion.
This tests the full sender/receiver wiring without real DSA hardware.

| Case | Description | Edge Case |
|------|-------------|-----------|
| `dsa_data_move` sender connects + starts | Operation pushed to queue, notified on poll | Void result |
| `dsa_mem_fill` sender | Same pattern | Void result |
| `dsa_compare` sender | Returns bool result | Result extraction |
| `dsa_compare_value` sender | Returns bool result | Result extraction |
| `dsa_crc_gen` sender | Returns uint32_t | Result extraction |
| `dsa_copy_crc` sender | Returns uint32_t | Result extraction |
| `dsa_cache_flush` sender | Void result | Simplest op |
| `dsa_dualcast` sender | Void result | pre_start_validate |
| Dualcast alignment error | dst1/dst2 bits[11:0] differ | set_error path |

## Implementation Order

1. **`test/test_utilities.cpp`** — Sections 1.1-1.7 (pure functions, no mocking)
2. **`test/test_stdexec_integration.cpp`** — Sections 2.1-2.11 (MockDsa + stdexec wiring)
3. Build both via existing xmake targets (`test_utilities`, `test_stdexec_integration`)
4. Run and fix failures
5. Verify all tests pass

## Notes

- Both test files use **doctest** (consistent with existing tests)
- No DSA hardware required for any new tests (all use MockDsa/MockHwContext)
- xmake targets already declared in `xmake.lua` for both files
- `test_helpers.hpp` provides `TestOpWrapper` and `make_test_op()` utilities
- Operation sender tests (section 2.11) require MockDsa to accept `submit(op, desc)` — may need a small adapter since MockDsa only has `submit(op)`. Plan: add `submit(op, desc)` overload to MockDsaBase that ignores the descriptor.
