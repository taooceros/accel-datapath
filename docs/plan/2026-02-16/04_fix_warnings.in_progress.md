# Compile Warnings Analysis

Two unique warnings across the codebase. Both are trivial fixes.

## Warning 1: `stdexec::empty_env` deprecated

```
include/dsa_stdexec/sync_wait.hpp:19:65: warning: 'using stdexec::empty_env = ...'
  is deprecated: stdexec::empty_env is now spelled stdexec::env<> [-Wdeprecated-declarations]
```

**Location:** `sync_wait.hpp` lines 19 and 105 — used in `value_types_of_t` and in
`get_env()` return (lines 57 and 99).

**Fix:** Replace `stdexec::empty_env` with `stdexec::env<>` (4 occurrences).

**Should fix: YES.** Trivial rename. The deprecated alias could be removed in a future
stdexec release, breaking the build. Zero risk — it's an exact rename per the
deprecation message.

## Warning 2: volatile dereference in `dsa.ipp`

```
src/dsa/dsa.ipp:179:24: warning: implicit dereference will not access object of
  type 'volatile char' in statement
```

**Location:** `dsa.ipp:179` — `wr ? *t = *t : *t;`

The problem: the read-side `*t` in the ternary is a discarded value expression.
GCC 15 warns that the volatile read has no effect because the result is unused in
a discarded-value context. The intent is to touch the faulting page (force a read
to trigger the page fault handler), but the compiler may optimize it away.

**Fix:** Same pattern we already used in `adjust_for_page_fault()`:
```cpp
if (wr) { *t = *t; } else { (void)*t; }
```
The `(void)*t` cast forces the volatile read to be sequenced as a side effect per
the standard (C++23 [expr.static.cast]/6).

**Should fix: YES.** This is a correctness concern, not just cosmetic. If the
compiler optimizes away the volatile read, the page fault won't be triggered and
the retry will fail with the same fault. The `if/else` form is unambiguous.

## Summary

| Warning | File | Fix | Risk | Verdict |
|---------|------|-----|------|---------|
| `empty_env` deprecated | `sync_wait.hpp` | `s/empty_env/env<>/g` | None | Fix |
| volatile discard | `dsa.ipp` | `if/else` instead of ternary | None | Fix |

Both are one-line changes with zero behavioral risk.
