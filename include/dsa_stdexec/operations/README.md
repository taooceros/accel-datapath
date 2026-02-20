# dsa_stdexec/operations -- Per-Operation Senders

Each DSA hardware operation has a dedicated sender implementation in this directory.
These senders wrap the low-level descriptor submission and completion handling into
the stdexec sender/receiver model so that DSA work can be composed into sender
pipelines.

## The Pattern

Every operation follows the same three-part structure:

1. **`<Op>Operation`** -- inherits `DsaOperationBase`. Implements `start()` (fills
   the hardware descriptor and submits it) and `notify()` (handles completion status,
   including automatic page-fault retry via `DSA_COMP_PAGE_FAULT_NOBOF`).

2. **`<Op>Sender`** -- the stdexec sender. Declares completion signatures
   `set_value_t(...)` and `set_error_t(std::exception_ptr)`. Connects to a receiver
   to produce the operation state.

3. **`dsa_<op>(dsa, ...)`** -- free factory function that constructs and returns the
   sender. This is the public API callers use.

## Key Files

| File | Purpose |
|------|---------|
| `all.hpp` | Convenience header -- includes every operation sender. |
| `operation_base_mixin.hpp` | CRTP mixin providing common operation logic shared across all operations (descriptor setup, completion handling boilerplate). |
| `data_move.hpp` | Memory copy (memmove equivalent). |
| `mem_fill.hpp` | Fill a memory region with a pattern. |
| `compare.hpp` | Compare two memory regions, returns match/mismatch. |
| `compare_value.hpp` | Compare a memory region against a fixed value. |
| `dualcast.hpp` | Copy from one source to two destinations. |
| `crc_gen.hpp` | Generate CRC over a memory region. |
| `copy_crc.hpp` | Copy memory and generate CRC in a single operation. |
| `cache_flush.hpp` | Flush cache lines for a memory region. |

## How to Add a New Operation

1. **Copy an existing file** (e.g., `data_move.hpp`) as a starting point.
2. **Implement `start()`** -- fill the descriptor fields specific to your operation
   (opcode, flags, source/destination addresses, size). Use helpers from
   `descriptor_fill.hpp`.
3. **Implement `notify()`** -- handle the completion record. Check status, propagate
   `set_value` on success or `set_error` on failure. Handle page faults if the
   operation supports partial completion.
4. **Define the sender** with appropriate `completion_signatures`.
5. **Add a `dsa_<op>()` factory function**.
6. **Include the new header in `all.hpp`**.
7. **Add an example** in `examples/` (follow the `example_<op>.cpp` convention) and
   register it as a build target in `xmake.lua`.
8. **Build and test**: `xmake build` to verify compilation.

## See Also

- [examples/](../../../examples/) -- one example per operation demonstrating basic usage
- [Parent README](../README.md) -- stdexec integration layer overview
- [CLAUDE.md](../../../CLAUDE.md) -- project-wide conventions and architecture overview
