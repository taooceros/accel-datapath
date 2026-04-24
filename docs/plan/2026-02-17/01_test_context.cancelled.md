# Test Coverage Improvement - Shared Context

**Project**: dsa-stdexec (Intel DSA + stdexec sender/receiver bindings)
**Date Created**: 2026-02-17
**Purpose**: Centralized knowledge base for agents implementing unit tests

Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

---

## Test Framework Configuration

### Framework: doctest

```cpp
#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include <doctest/doctest.h>
```

**Key macros**:
- `TEST_CASE("description")` - Define test case
- `SUBCASE("description")` - Define subtest within a test case
- `CHECK(expr)` - Non-fatal assertion
- `REQUIRE(expr)` - Fatal assertion
- `CHECK_THROWS_AS(expr, exception_type)` - Exception verification
- `CHECK_EQ(a, b)` / `CHECK_NE(a, b)` - Equality checks
- `CHECK_LT(a, b)` / `CHECK_GT(a, b)` - Comparison checks

---

## Build System: xmake

### Missing Test Targets

Two test executables are defined in `xmake.lua` but their source files are **MISSING**:

1. **`test_utilities`** → `test/test_utilities.cpp`
   - Pure unit tests, no hardware or mock DSA
   - Dependencies: `fmt`, `stdexec`
   - Compiler flags: `-menqcmd`, `-mmovdir64b`
   - Include dirs: `test/`

2. **`test_stdexec_integration`** → `test/test_stdexec_integration.cpp`
   - stdexec integration tests with MockDsa
   - Dependencies: `fmt`, `stdexec`
   - Compiler flags: `-menqcmd`, `-mmovdir64b`
   - Include dirs: `test/`

**Build commands**:
```bash
xmake build test_utilities
xmake build test_stdexec_integration
xmake run test_utilities
xmake run test_stdexec_integration
```

**Note**: Neither test requires hardware (no libdsa, no libaccel-config packages).

---

## Existing Test Infrastructure

### 1. Existing Test Files

| File | Purpose | Hardware Required |
|------|---------|-------------------|
| `test/test_task_queues.cpp` | Task queue implementations with MockHwContext | No |
| `test/test_mirrored_ring.cpp` | MirroredRing aliasing + DSA data_move | Yes |
| `test/test_helpers.hpp` | TestOpWrapper bridging MockOperation to OperationBase proxy | No |
| `test/dsa.cpp` | Placeholder hello world | No |

### 2. Mock Infrastructure (`src/dsa/mock_dsa.hpp`)

#### MockHwContext
```cpp
struct MockHwContext {
  static bool check_completion(dsa_completion_record* comp) {
    return true; // Always returns true (immediate completion)
  }
};
```

#### MockOperation
```cpp
struct MockOperation {
  MockOperation(std::chrono::milliseconds delay = 0ms)
  void set_callback(std::function<void()> cb)
  void complete()
  bool is_completed() const
}
```
- Configurable delay
- Immediate completion by default
- Supports completion callback

#### MockDsaBase<QueueTemplate>
```cpp
template <template <typename> class QueueTemplate>
class MockDsaBase {
  MockDsaBase(bool start_poller = false)
  void submit(OperationBase* op)
  void poll()
  void flush()
  auto get_scheduler()
}
```
- Full mock DSA with optional background poller thread
- `submit()` pushes to task queue
- `poll()` checks completions and calls `op->notify()`
- Background poller enabled via `MockDsaBase(true)`

#### Type Aliases
```cpp
using MockDsa = MockDsaBase<MutexTaskQueue>;
using MockDsaSingleThread = MockDsaBase<SingleThreadTaskQueue>;
using MockDsaTasSpinlock = MockDsaBase<TasSpinlockTaskQueue>;
using MockDsaSpinlock = MockDsaBase<SpinlockTaskQueue>;
using MockDsaBackoffSpinlock = MockDsaBase<BackoffSpinlockTaskQueue>;
using MockDsaLockFree = MockDsaBase<LockFreeTaskQueue>;
using MockDsaRingBuffer = MockDsaBase<RingBufferTaskQueue>;
```

### 3. Test Helpers (`test/test_helpers.hpp`)

#### TestOpWrapper
Bridges `MockOperation` to `OperationBase` proxy:
```cpp
struct TestOpWrapper {
  TestOpWrapper(MockOperation* mock_op, dsa_hw_desc* desc)
  void notify()
  dsa_hw_desc* get_descriptor()
  TestOpWrapper* next = nullptr;
}
```

---

## Components Requiring Test Coverage

### File 1: `test/test_utilities.cpp`

**Purpose**: Pure unit tests with no mock DSA dependency

#### 1. DsaOperationBase Alignment (`src/dsa/dsa_operation_base.hpp`)

**Test objectives**:
- Verify `desc_ptr()` returns 64-byte aligned pointer
- Verify `comp_ptr()` returns 32-byte aligned pointer
- Verify cached pointers match computed alignment
- Test multiple instantiations to ensure consistent alignment

**Key implementation detail**: Uses over-allocation + runtime computation (NOT `alignas()`) for coroutine frame compatibility.

```cpp
TEST_CASE("DsaOperationBase alignment") {
  SUBCASE("descriptor pointer is 64-byte aligned") {
    // Create derived operation, check desc_ptr() % 64 == 0
  }
  SUBCASE("completion pointer is 32-byte aligned") {
    // Create derived operation, check comp_ptr() % 32 == 0
  }
  SUBCASE("cached pointers match computed alignment") {
    // Verify descriptor_ptr_ == computed_desc_ptr
  }
}
```

#### 2. Descriptor Fill Functions (`include/dsa_stdexec/descriptor_fill.hpp`)

**Functions to test**:
- `fill_data_move(desc, src, dst, len, flags)`
- `fill_mem_fill(desc, dst, pattern, len, flags)`
- `fill_compare(desc, src1, src2, len, flags)`
- `fill_compare_value(desc, src, value, len, flags)`
- `fill_dualcast(desc, src, dst1, dst2, len, flags)`
- `fill_crc_gen(desc, src, len, crc_seed, flags)`
- `fill_copy_crc(desc, src, dst, len, crc_seed, flags)`
- `fill_cache_flush(desc, dst, len, flags)`

**Test objectives** (per function):
- Verify correct opcode set
- Verify flags applied correctly
- Verify addresses/pointers stored correctly
- Verify sizes/lengths stored correctly
- Verify operation-specific fields (pattern, value, crc_seed, dst2)

**Critical**: Descriptor must be zeroed before calling fill function.

```cpp
TEST_CASE("descriptor_fill functions") {
  SUBCASE("fill_data_move") {
    dsa_hw_desc desc{};
    uint8_t src[64], dst[64];
    fill_data_move(&desc, src, dst, 64, 0);
    CHECK_EQ(desc.opcode, DSA_OPCODE_MEMMOVE);
    CHECK_EQ(desc.src_addr, reinterpret_cast<uint64_t>(src));
    // ... verify all fields
  }
  // ... repeat for all 8 operations
}
```

#### 3. DsaError Hierarchy (`include/dsa_stdexec/error.hpp`)

**Classes to test**:
- `DsaError` - Base error with completion record
- `DsaSubmitError` - Submission failures
- `DsaInitError` - Initialization failures

**Test objectives**:
- Constructor with status, completion record, custom message
- `dsa_status_to_string()` - status code to string conversion
- `dsa_opcode_to_string()` - opcode to string conversion
- Accessors: `status()`, `opcode()`, `bytes_completed()`, `fault_addr()`
- Exception hierarchy (inherits from `std::runtime_error`)

```cpp
TEST_CASE("DsaError hierarchy") {
  SUBCASE("DsaError construction") {
    dsa_completion_record comp{};
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    DsaError err(DSA_COMP_PAGE_FAULT_NOBOF, &comp, "test error");
    CHECK_EQ(err.status(), DSA_COMP_PAGE_FAULT_NOBOF);
  }
  SUBCASE("status_to_string") {
    auto s = dsa_status_to_string(DSA_COMP_SUCCESS);
    CHECK_NE(s.find("SUCCESS"), std::string::npos);
  }
}
```

#### 4. OperationBase Proxy Dispatch (`include/dsa_stdexec/operation_base.hpp`)

**Test objectives**:
- `notify()` dispatches through `pro::proxy<OperationFacade>`
- `get_descriptor()` dispatches through proxy
- Intrusive `next` pointer linking works correctly

**Key design**: Type-erased operation using Microsoft's proxy library.

```cpp
TEST_CASE("OperationBase proxy dispatch") {
  SUBCASE("notify dispatches through proxy") {
    // Create TestOpWrapper, wrap in OperationBase, call notify()
  }
  SUBCASE("get_descriptor returns correct pointer") {
    // Verify descriptor pointer through proxy
  }
  SUBCASE("intrusive next pointer linking") {
    // Link multiple OperationBase nodes
  }
}
```

#### 5. Page Fault Adjustment (`include/dsa_stdexec/operations/operation_base_mixin.hpp`)

**Test objectives**:
- `adjust_for_page_fault()` correctly adjusts descriptor fields for each opcode
- `g_page_fault_retries` counter increments
- Handles `DSA_COMP_PAGE_FAULT_NOBOF` status
- Adjusts addresses and lengths based on `bytes_completed()`

**Opcodes supporting page fault retry**:
- `DSA_OPCODE_MEMMOVE` (data_move)
- `DSA_OPCODE_MEMFILL` (mem_fill)
- `DSA_OPCODE_COMPARE` (compare)
- `DSA_OPCODE_DUALCAST` (dualcast)
- `DSA_OPCODE_CRC_GEN` (crc_gen)
- `DSA_OPCODE_COPY_CRC` (copy_crc)

```cpp
TEST_CASE("Page fault adjustment") {
  SUBCASE("data_move page fault retry") {
    // Set comp.bytes_completed, call adjust_for_page_fault
    // Verify src_addr += bytes_completed, dst_addr += bytes_completed
    // Verify xfer_size -= bytes_completed
  }
  // ... repeat for all opcodes
}
```

#### 6. Lock Implementations (`src/dsa/task_queue.hpp`)

**Locks to test**:
- `NullLock` - No-op for single-threaded
- `MutexLock` - std::mutex wrapper
- `TasSpinlock` - Test-and-set spinlock
- `TtasSpinlock` - Test-and-test-and-set spinlock
- `TtasBackoffSpinlock` - TTAS with exponential backoff

**Test objectives**:
- `lock()` and `unlock()` semantics
- Thread safety (use std::thread for mutex/spinlock variants)
- NullLock is truly no-op
- RAII compatibility with `std::lock_guard`

```cpp
TEST_CASE("Lock implementations") {
  SUBCASE("NullLock is no-op") {
    NullLock lock;
    lock.lock();
    lock.unlock();
    // Should not throw or block
  }
  SUBCASE("MutexLock thread safety") {
    // Spawn threads, verify mutual exclusion
  }
}
```

---

### File 2: `test/test_stdexec_integration.cpp`

**Purpose**: stdexec integration tests using MockDsa

**Required includes**:
```cpp
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <dsa_stdexec/dsa_facade.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/operations/mem_fill.hpp>
// ... other operations
#include <src/dsa/mock_dsa.hpp>
```

#### 1. PollingRunLoop (`include/dsa_stdexec/run_loop.hpp`)

**Test objectives**:
- `get_scheduler()` returns valid scheduler
- `schedule()` sender completes via `run()`
- `finish()` stops the loop
- `reset()` allows re-running
- Multiple tasks execute in order

```cpp
TEST_CASE("PollingRunLoop") {
  SUBCASE("schedule completes via run") {
    MockDsa dsa;
    PollingRunLoop loop([&] { dsa.poll(); });

    bool executed = false;
    auto work = stdexec::schedule(loop.get_scheduler())
              | stdexec::then([&] { executed = true; });

    stdexec::start_detached(std::move(work));
    loop.run();

    CHECK(executed);
  }

  SUBCASE("finish stops the loop") {
    // Test that finish() causes run() to exit
  }

  SUBCASE("reset allows re-running") {
    // Run, finish, reset, run again
  }
}
```

#### 2. DsaScheduler (`include/dsa_stdexec/scheduler.hpp`)

**Test objectives**:
- `schedule()` returns sender that completes on poll
- Works with MockDsa
- Scheduler equality comparison

```cpp
TEST_CASE("DsaScheduler") {
  SUBCASE("schedule completes on poll") {
    MockDsa dsa;
    auto sched = dsa.get_scheduler();

    bool completed = false;
    auto work = stdexec::schedule(sched)
              | stdexec::then([&] { completed = true; });

    stdexec::start_detached(std::move(work));
    dsa.poll();

    CHECK(completed);
  }
}
```

#### 3. sync_wait Helpers (`include/dsa_stdexec/sync_wait.hpp`)

**Functions to test**:
- `sync_wait_threaded(sender)` - Background poller thread
- `wait_start(sender, loop)` - Inline polling

**Test objectives**:
- `sync_wait_threaded()` with MockDsa background poller
- `wait_start()` with PollingRunLoop
- Both correctly block until completion
- Return values propagate correctly

```cpp
TEST_CASE("sync_wait helpers") {
  SUBCASE("sync_wait_threaded with background poller") {
    MockDsa dsa(true); // Enable background poller

    auto work = stdexec::just(42);
    auto result = sync_wait_threaded(std::move(work));

    CHECK(result.has_value());
    CHECK_EQ(std::get<0>(*result), 42);
  }

  SUBCASE("wait_start with PollingRunLoop") {
    MockDsa dsa;
    PollingRunLoop loop([&] { dsa.poll(); });

    auto work = stdexec::just(99);
    auto result = wait_start(std::move(work), loop);

    CHECK(result.has_value());
    CHECK_EQ(std::get<0>(*result), 99);
  }
}
```

#### 4. Operation Senders with MockDsa

**Operations to test**:
- `dsa_data_move(dsa, src, dst, len)`
- `dsa_mem_fill(dsa, dst, pattern, len)`
- `dsa_compare(dsa, src1, src2, len)`
- `dsa_compare_value(dsa, src, value, len)`
- `dsa_dualcast(dsa, src, dst1, dst2, len)`
- `dsa_crc_gen(dsa, src, len, seed)`
- `dsa_copy_crc(dsa, src, dst, len, seed)`
- `dsa_cache_flush(dsa, dst, len)`

**Test objectives**:
- Sender connects and starts with MockDsa
- Completion signatures are correct
- `set_value_t(...)` called on success
- `set_error_t(exception_ptr)` on error (if applicable)
- MockDsa submit/poll cycle completes operation

```cpp
TEST_CASE("Operation senders with MockDsa") {
  SUBCASE("dsa_data_move completes successfully") {
    MockDsa dsa;
    uint8_t src[64] = {1, 2, 3};
    uint8_t dst[64] = {};

    auto work = dsa_data_move(dsa, src, dst, 64)
              | stdexec::then([&] {
                  // Verify completion
                  return true;
                });

    bool completed = false;
    auto op = stdexec::connect(std::move(work),
                               receiver_that_sets(completed));
    stdexec::start(op);
    dsa.poll();

    CHECK(completed);
  }

  // ... repeat for all 8 operations
}
```

#### 5. DsaProxy Type Erasure (`include/dsa_stdexec/dsa_facade.hpp`)

**Test objectives**:
- `DsaProxy` wraps MockDsa
- Dispatches `submit()`, `poll()`, `flush()`
- `make_dsa_proxy()` factory works
- `operator bool()` validity check

```cpp
TEST_CASE("DsaProxy type erasure") {
  SUBCASE("DsaProxy wraps MockDsa") {
    MockDsa dsa;
    auto proxy = make_dsa_proxy(dsa);

    CHECK(static_cast<bool>(proxy));

    // Test submit/poll through proxy
    MockOperation mock_op;
    TestOpWrapper wrapper(&mock_op, nullptr);
    OperationBase op = pro::make_proxy<OperationFacade>(&wrapper);

    proxy.submit(&op);
    proxy.poll();

    CHECK(mock_op.is_completed());
  }
}
```

---

## Key Design Principles for Test Authors

### 1. Type Erasure with Proxy
- `OperationBase` uses `pro::proxy<OperationFacade>` (NOT virtual dispatch)
- Enables heterogeneous operation types in intrusive linked lists
- Dispatch methods: `notify()`, `get_descriptor()`

### 2. Alignment Strategy
- **DO NOT** use `alignas()` (breaks coroutine frames)
- **DO** use over-allocation + runtime computation in `DsaOperationBase`
- Descriptor: 64-byte aligned
- Completion record: 32-byte aligned

### 3. Mock Hardware Behavior
- `MockHwContext::check_completion()` always returns `true` (immediate completion)
- `MockDsa::submit()` pushes to task queue
- `MockDsa::poll()` checks completions and calls `op->notify()`
- Background poller thread enabled via `MockDsaBase(true)`

### 4. Page Fault Handling
- Hardware status `DSA_COMP_PAGE_FAULT_NOBOF` triggers retry logic
- `adjust_for_page_fault()` updates descriptor addresses/lengths
- Page fault counter: `g_page_fault_retries` (global)

### 5. Descriptor Fill Protocol
- **CRITICAL**: Descriptor MUST be zeroed before calling fill function
- Use `dsa_hw_desc desc{};` or `memset(&desc, 0, sizeof(desc))`
- Fill functions set opcode, flags, addresses, sizes

### 6. Sender/Receiver Patterns
- Each operation has:
  - `<Op>Operation` (inherits `DsaOperationBase`)
  - `<Op>Sender` (stdexec sender)
  - Free function `dsa_<op>(dsa, ...)` (factory)
- Completion signatures:
  - `set_value_t(...)` - Success (operation-specific return values)
  - `set_error_t(exception_ptr)` - Error

---

## Test Execution Workflow

### Build and Run
```bash
# Build both test targets
xmake build test_utilities
xmake build test_stdexec_integration

# Run tests
xmake run test_utilities
xmake run test_stdexec_integration

# Or build+run all tests
xmake build && xmake run test_utilities && xmake run test_stdexec_integration
```

### Expected Output
```
[doctest] test cases: XX | XX passed | 0 failed | 0 skipped
[doctest] assertions: XX | XX passed | 0 failed |
[doctest] Status: SUCCESS!
```

---

## Dependencies Reference

### Required Headers (test_utilities.cpp)
```cpp
#include <dsa/dsa_operation_base.hpp>
#include <dsa_stdexec/descriptor_fill.hpp>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/operations/operation_base_mixin.hpp>
#include <dsa/task_queue.hpp>
```

### Required Headers (test_stdexec_integration.cpp)
```cpp
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <dsa_stdexec/dsa_facade.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/operations/mem_fill.hpp>
#include <dsa_stdexec/operations/compare.hpp>
#include <dsa_stdexec/operations/compare_value.hpp>
#include <dsa_stdexec/operations/dualcast.hpp>
#include <dsa_stdexec/operations/crc_gen.hpp>
#include <dsa_stdexec/operations/copy_crc.hpp>
#include <dsa_stdexec/operations/cache_flush.hpp>
#include <src/dsa/mock_dsa.hpp>
#include <test/test_helpers.hpp>
```

---

## Coverage Goals

### test_utilities.cpp
- [ ] 6 test cases (alignment, descriptor fills, errors, proxy, page fault, locks)
- [ ] ~40-50 subcases covering all descriptor fill functions, error types, lock variants
- [ ] 100% coverage of utility functions and data structures

### test_stdexec_integration.cpp
- [ ] 5 test cases (run loop, scheduler, sync_wait, operations, proxy)
- [ ] ~20-30 subcases covering all 8 operations, sync modes, scheduler behavior
- [ ] 100% coverage of stdexec integration layer

### Overall Success Criteria
- All tests pass without hardware dependency
- No memory leaks (verify with AddressSanitizer: `xmake f --policies=build.sanitizer.address && xmake`)
- Clear test names describing what is being tested
- Each subcase tests one specific behavior
- Mock infrastructure correctly simulates hardware behavior

---

## Additional Resources

- **DSA Hardware Spec**: `/home/hongtao/dsa-stdexec/dsa_architecture_spec.md` (637KB)
- **Project Instructions**: `/home/hongtao/dsa-stdexec/CLAUDE.md`
- **Existing Test Example**: `/home/hongtao/dsa-stdexec/test/test_task_queues.cpp`
- **Mock Infrastructure**: `/home/hongtao/dsa-stdexec/src/dsa/mock_dsa.hpp`
- **Test Helpers**: `/home/hongtao/dsa-stdexec/test/test_helpers.hpp`

---

## Notes for Agent Collaboration

### When Writing Tests
1. Start with `#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN`
2. Include doctest header: `#include <doctest/doctest.h>`
3. Use absolute paths in `#include` statements
4. Zero descriptors before filling: `dsa_hw_desc desc{};`
5. Verify alignment with modulo checks: `ptr % alignment == 0`
6. Use `MockDsa` for integration tests, avoid raw hardware dependencies

### When Reviewing Tests
1. Verify all assertions use doctest macros (not assert())
2. Check that descriptors are zeroed before fill
3. Ensure MockDsa is used correctly (submit → poll cycle)
4. Validate that type erasure uses `pro::proxy` correctly
5. Confirm thread safety tests for lock implementations

### When Extending Coverage
1. Consult this document for mock infrastructure patterns
2. Follow existing test structure in `test_task_queues.cpp`
3. Update coverage checklist when adding new subcases
4. Document any new test helpers in `test/test_helpers.hpp`
5. Share findings back to this context document

---

**Document Version**: 1.0
**Last Updated**: 2026-02-17
**Maintained By**: context-manager agent
**Status**: cancelled on 2026-04-15
