# DSA Operation Examples

Quick-start examples for Intel Data Streaming Accelerator (DSA) operations
using the stdexec sender/receiver pattern. Each file demonstrates a single
DSA hardware operation end-to-end.

## Getting Started

Start with `data_move.cpp` -- it is the simplest example and covers the
core pattern used by all other operations.

## Build

Build a single example by target name (`example_<op>`):

```bash
xmake build example_data_move
xmake build example_mem_fill
xmake build example_crc_gen
# ... and so on for any example_<op>
```

## Run

DSA requires `CAP_SYS_RAWIO`. Use the `dsa_launcher` wrapper to provide
the capability at runtime:

```bash
dsa_launcher ./build/<mode>/example_data_move
```

Replace `<mode>` with your current xmake build mode directory (e.g.
`linux/x86_64/release`). If you are inside a `devenv shell`, the `run`
script handles capability setup automatically for the benchmark target,
but for individual examples use `dsa_launcher` directly.

## Examples

| File                | Operation       | Description                                      |
|---------------------|-----------------|--------------------------------------------------|
| `data_move.cpp`     | Memory copy     | Copy a memory region from source to destination  |
| `mem_fill.cpp`      | Memory fill     | Fill a memory region with a fixed pattern        |
| `compare.cpp`       | Compare         | Compare two memory regions for equality          |
| `compare_value.cpp` | Compare value   | Compare a memory region against a fixed value    |
| `dualcast.cpp`      | Dualcast        | Copy source to two destinations simultaneously   |
| `crc_gen.cpp`       | CRC generation  | Compute a CRC32 checksum over a memory region    |
| `copy_crc.cpp`      | Copy + CRC      | Copy memory and compute CRC32 in a single pass   |
| `cache_flush.cpp`   | Cache flush     | Flush a memory region from CPU caches            |

## API Pattern

Every example follows the same structure:

1. Create a `Dsa` engine and discover a DSA device.
2. Call `dsa_<op>(dsa, ...)` to obtain a stdexec sender.
3. Execute the sender (e.g. via `sync_wait` or `wait_start` with a polling loop).
4. Inspect the result delivered through the receiver.

## Further Reading

- [CLAUDE.md](../CLAUDE.md) -- project architecture overview and build reference.
- [include/dsa_stdexec/operations/](../include/dsa_stdexec/operations/) -- sender implementations backing each `dsa_<op>()` call.
