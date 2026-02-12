#set document(title: "DSA Completion Record Alignment Bug")
#set page(margin: 1.5cm)
#set text(font: "New Computer Modern", size: 10pt)
#set heading(numbering: "1.1")

#align(center)[
  #text(size: 16pt, weight: "bold")[DSA Completion Record Alignment Bug]
  #v(0.3em)
  #text(size: 10pt, fill: gray)[dsa-stdexec -- January 2026]
]

= Summary

DSA operations hang when using scoped workers with inline polling. The root cause is misaligned completion records within coroutine frames. The fix uses runtime alignment via over-allocated buffers.

= Symptoms

- Worker 0: completes normally
- Worker 1: submits operation, never completes
- Completion status stays `0` forever
- ASAN masks the bug (stricter allocator alignment)

= Code Structure

#block(breakable: false)[
*Worker coroutine* (`benchmark/dsa_benchmark.cpp:236`):
```cpp
exec::task<void> worker_coro(DsaType &dsa, ...) {
  while (current_op < num_ops) {
    co_await dsa_data_move(dsa, src, dst, size);  // suspends here
  }
}
```

*DataMoveOperation* (`include/dsa_stdexec/data_move.hpp:31`):
```cpp
class DataMoveOperation : public dsa::DsaOperationBase {
  // Inherits desc_buffer_ and comp_buffer_ from DsaOperationBase
};
```

*Call chain*:
+ `worker_coro()` calls `co_await dsa_data_move()`
+ `dsa_data_move()` creates `DataMoveSender`
+ `connect()` creates `DataMoveOperation` *inside coroutine frame*
+ `DataMoveOperation` contains completion record that must be 32B aligned
]

= How C++20 Coroutines Work

*Coroutine = function that can suspend and resume*

#block(breakable: false)[
```
┌─────────────────────────────────────────────────────────┐
│ Coroutine Frame (heap-allocated via operator new)       │
├─────────────────────────────────────────────────────────┤
│ • promise object (return value handling)                │
│ • function parameters (copied)                          │
│ • local variables that live across suspend points       │
│ • current suspension point (resume address)             │
└─────────────────────────────────────────────────────────┘
```

*Key points:*
- Frame allocated via `operator new(size_t)` -- returns 16B aligned memory
- Objects crossing `co_await` are stored in frame (not stack)
- Compiler ignores `alignas()` when laying out frame -- *the bug*
]

#block(breakable: false)[
*Execution flow*:
```
worker_coro() called
  ├── Frame allocated (malloc → 16B aligned)
  ├── Local vars initialized in frame
  │
  ├── co_await dsa_data_move(...)
  │     ├── DataMoveOperation created IN FRAME ← alignment lost here
  │     ├── DSA descriptor submitted to hardware
  │     └── Coroutine SUSPENDS (returns to caller)
  │
  │   ... hardware processes, poll loop runs ...
  │
  ├── Coroutine RESUMES (completion detected)
  │     └── co_await returns
  │
  └── co_return (frame deallocated)
```
]

= DSA Hardware Requirements

#table(
  columns: (auto, auto, auto),
  stroke: 0.5pt,
  [*Structure*], [*Size*], [*Alignment*],
  [Descriptor], [64B], [64-byte],
  [Completion record], [32B], [32-byte],
)

*Critical*: Misaligned completion records cause silent failure -- hardware completes but never writes status.

= Root Cause

#table(
  columns: (auto, auto, auto),
  stroke: 0.5pt,
  [*Worker*], [*Address*], [*Aligned?*],
  [0], [`0x...520`], [Yes],
  [1], [`0x...990`], [No (off by 16)],
)

Coroutine frame allocators ignore `alignas()`. This is a known compiler bug (see @compiler-bugs).

= Memory Layout

#block(breakable: false)[
```
┌─────────────────────────────────────────────────────────────┐
│  exec::task coroutine frame (16B aligned by malloc)         │
├─────────────────────────────────────────────────────────────┤
│  promise, locals...                                         │
├─────────────────────────────────────────────────────────────┤
│  DataMoveOperation (data_move.hpp:31)                       │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ DsaOperationBase (dsa_operation_base.hpp:30)            ││
│  │ ┌─────────────────────────────────────────────────────┐ ││
│  │ │ OperationBase (proxy, next ptr)  ~32B               │ ││
│  │ ├─────────────────────────────────────────────────────┤ ││
│  │ │ desc_ (64B) - may be misaligned                     │ ││
│  │ ├─────────────────────────────────────────────────────┤ ││
│  │ │ comp_ (32B) - MISALIGNED → DSA writes fail silently │ ││
│  │ └─────────────────────────────────────────────────────┘ ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```
]

= The Fix

#block(breakable: false)[
*Before* (broken):
```cpp
struct alignas(64) DsaOperationBase {
  dsa_hw_desc desc_ __attribute__((aligned(64)));
  dsa_completion_record comp_ __attribute__((aligned(32)));
};
```

*After* (works):
```cpp
struct DsaOperationBase {
  char desc_buffer_[64 + 63];  // over-allocate
  char comp_buffer_[32 + 31];

  dsa_hw_desc* desc_ptr() {
    return align_to<64>(desc_buffer_);
  }
  dsa_completion_record* comp_ptr() {
    return align_to<32>(comp_buffer_);
  }
};
```

Alignment computed at runtime: `(addr + N-1) & ~(N-1)`
]

= Key Takeaways

#block(breakable: false)[
#box(stroke: 0.5pt + blue, fill: blue.lighten(90%), inset: 8pt, width: 100%)[
  1. *Don't trust `alignas()` in coroutines* - compilers don't honor it
  2. *DSA fails silently* on misaligned completion records
  3. *ASAN can mask alignment bugs* - different allocator behavior
  4. *Use runtime alignment* for hardware-critical structures
]
]

= Compiler Bug References <compiler-bugs>

#figure(
  table(
    columns: (auto, auto, auto),
    stroke: 0.5pt,
    inset: 6pt,
    [*Compiler*], [*Issue*], [*Description*],
    [LLVM/Clang], [#link("https://github.com/llvm/llvm-project/issues/53148")[\#53148]], [Coroutine frame wrong alignment],
    [LLVM/Clang], [#link("https://github.com/llvm/llvm-project/issues/56671")[\#56671]], [Misaligned variables in coroutine frames],
    [LLVM/Clang], [#link("https://reviews.llvm.org/D97915")[D97915]], [Handle overaligned frame allocation],
    [GCC], [#link("https://gcc.gnu.org/bugzilla/show_bug.cgi?id=104177")[Bug 104177]], [Coroutine frame alignment],
  ),
  caption: [Related compiler bugs.],
)

*Root cause*: `malloc` returns 16-byte aligned memory; compilers don't use `operator new(size_t, align_val_t)` for coroutine frames.

= Files Changed

- `src/dsa/dsa_operation_base.hpp` -- over-allocated buffers + runtime alignment
- `src/dsa/dsa.hpp` -- use `comp_ptr()`
- `include/dsa_stdexec/data_move.hpp` -- use `desc_ptr()`, `comp_ptr()`
- `include/dsa_stdexec/scheduler.hpp` -- use `comp_ptr()`
