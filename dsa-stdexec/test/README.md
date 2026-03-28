# Test Suite

Unit and integration tests for the dsa-stdexec project. Tests cover the low-level DSA engine, task queue implementations, ring buffer infrastructure, utility functions, and stdexec sender/receiver integration.

## Key Files

| File | Description |
|------|-------------|
| `dsa.cpp` | Core DSA engine tests -- submit, poll, and completion handling |
| `test_task_queues.cpp` | TaskQueue concept implementation tests (mutex, spinlock, lock-free, ring buffer) |
| `test_mirrored_ring.cpp` | MirroredRing double-mapped ring buffer tests |
| `test_utilities.cpp` | Utility function tests |
| `test_helpers.hpp` | Shared test helpers and fixtures used across test files |
| `test_stdexec_integration.cpp` | stdexec sender/receiver integration tests (new, in-progress) |

## Building

Tests are built as part of the default build:

```bash
xmake build
```

You can also build specific test targets by name if defined in `xmake.lua`.

## Running

DSA tests require `CAP_SYS_RAWIO` to access the hardware. Use the `dsa_launcher` tool to run test binaries with the required capability:

```bash
dsa_launcher ./build/<mode>/<platform>/test_binary
```

The `dsa_launcher` binary has `setcap cap_sys_rawio+eip` applied and passes the capability to child processes via ambient capabilities. See the main [../AGENTS.md](../AGENTS.md) for more details on the launcher and build modes.

## What Is Tested

- **DSA engine**: Descriptor submission, polling, and completion record handling via `DsaEngine`.
- **Task queues**: All `TaskQueue` concept implementations -- `MutexTaskQueue`, TTAS spinlock, backoff spinlock, lock-free, and ring buffer variants.
- **MirroredRing**: The double-mapped ring buffer used by batch submitters, verifying correct wrap-around semantics and RAII cleanup.
- **Utilities**: Shared utility functions used across the codebase.

## What Is Not Yet Tested

- **stdexec integration** (`test_stdexec_integration.cpp`): This file is new and in-progress. It will cover the sender/receiver integration layer including operation senders, `PollingRunLoop`, and `wait_start`.
- **Benchmark strategies**: The benchmark suite under `benchmark/dsa/strategies/` is exercised by running benchmarks directly rather than through unit tests.
- **Page fault retry logic**: The `DSA_COMP_PAGE_FAULT_NOBOF` handling path in operation senders is not covered by automated tests.

## Further Reading

See [../AGENTS.md](../AGENTS.md) for project architecture, build commands, and hardware requirements.
