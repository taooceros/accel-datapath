# Plan: Presentation Revision — 2026-03-03

## Slide-by-slide changes

---

### Slide: "What I Built"
**Problem**: "8 DSA op senders", "7 sweep dimensions", "submitter backends" are opaque to an advisor.

**Fix**: Replace the table with plain English paragraphs + a three-layer diagram:
```
[ Application: copy 8 bytes from A to B ]
        ↓
[ Scheduling strategy: when to submit, how many at once ]
        ↓
[ Submission backend: how descriptors reach the hardware ]
        ↓
[ Intel DSA hardware ]
```
Explain each layer in 1–2 sentences before introducing the names.
Also rename "sweep dimensions" → "benchmark parameters (message size, concurrency, …)".
"DSA op senders" → "C++ wrappers for each of DSA's 8 operations (copy, fill, CRC, …)".

---

### Slide order change
**Move auto-batching BEFORE concurrency strategies.**

New order:
1. Title
2. Agenda
3. Undergraduate project
4. What is DSA / What I Built (system overview)
5. **The MMIO Doorbell Problem** (motivation)
6. **Auto-Batching: Transparent by Design**
7. **How the Mirrored Ring Works** (illustration)
8. **Results: Batching Impact**
9. Concurrency Strategy Families (illustration)
10. Sliding Window sub-strategies
11. The Cost of Composability
12. Closing

---

### Slide: Concurrency Strategy Families
**Problem**: Advisor has no mental model for how work is submitted.

**Fix**: Add a cetz diagram showing the three patterns side-by-side:
- Sliding window: timeline with C lanes, each lane always has one op in flight
- Batch: timeline showing C ops fired together, barrier, idle gap, next batch
- Scoped workers: N horizontal coroutine lanes, each with submit → await → submit

Keep the table as a reference below the diagram.
Remove the separate "polling modes" table from this slide (mention inline/threaded in a footnote or fold into the table).

---

### Slide: Cost of Composability
**Problem**: "scope.nest() + then() adapters" is meaningless without seeing the code.

**Fix**: Add a code snippet showing the full stdexec pipeline:
```cpp
// noalloc — full pipeline
scope.nest(dsa_data_move(dsa, src, dst, 8) | then([&]{ record(); }))
// connect → start → poll → set_value → chain propagates back

// direct — bypass scope + then
connect(dsa_data_move(dsa, src, dst, 8), DirectReceiver{slot})
// No async_scope lifetime tracking; no then-adapter allocation

// reusable — bypass connect + start entirely
slot.refill_descriptor(src, dst, 8);
slot.submit();   // raw MOVDIR64B; poll separately
```
Then explain: "composability" = the ability to write `op | then(f)` and have
lifetime, cancellation, and error propagation handled automatically. Each adapter
adds a wrapper type; the cost is constructing and destroying those wrapper objects
per operation.

Keep the delta table but add a callout: "These are measured throughput deltas,
not guesses — the mock hardware removes all real-work variance."

---

### Slide: Mirrored Ring
**Problem**: Explanation is all words; no illustration; page-alignment constraint missing.

**Fix**: Draw a cetz diagram showing:
- Top row: plain ring buffer — batch spanning slots 480–511 crosses the boundary,
  gets split (early seal shown in red)
- Bottom row: mirrored ring — the same physical pages appear twice in VA,
  so the batch spans 480–511 + mirror[0–31] contiguously (green)
- Label: "VA = [base, base+2R); same physical pages mapped at base+R"

Add a footnote: "Ring size must be a multiple of the OS page size (4 KB),
since `mmap` maps at page granularity."

Fix the broken math formula: replace `$ceil(N \/ 32)$` with
`$lceil N/32 rceil$` (Typst ceiling function syntax).

---

### Slide: Results
**Problem**: "Mpps numbers seem too slow" and "general principle" explanation is confusing.

**Fix for results**:
- Add context sentence: "These benchmarks use concurrency=16, which is conservative.
  The hardware supports up to 128 in-flight ops per work queue."
- Highlight the best number: peak 34 Mpps at concurrency=1024 with reusable strategy.
- Remove the "general principle" comparison table (RDMA, io_uring, NVMe).
  The advisor cannot verify those numbers and they distract from the DSA story.

**Fix for "general principle" message**:
Replace the confusing framing with a direct claim:
> "Without batching, each op costs ~160 ns of MMIO overhead, so 21 ns of
>  framework cost is only 13% of total. With batching, per-op HW cost drops to ~5 ns,
>  making the 21 ns framework cost the dominant term — 4× the HW cost.
>  Batching is what makes optimizing the framework worth doing."

---

### Remove: "What's Next"
Delete entirely.

---

### Fix: Math formula
The `$ceil(N \/ 32)$` syntax does not render correctly in Typst.
Use `$⌈N/32⌉$` (Unicode ceiling chars) or `$ceil(N\/32)$`.
Audit all math blocks and fix any rendering issues.

---

## Summary of structural changes

| # | Old slide | New slide |
|---|-----------|-----------|
| 1 | Title | Title (unchanged) |
| 2 | Agenda | Agenda (unchanged) |
| 3 | Undergrad project | Undergrad project (unchanged) |
| 4 | What I Built | What I Built — layered diagram, plain English |
| 5 | Concurrency Families | **Doorbell Problem** (moved up) |
| 6 | Sub-strategies | **Auto-Batching** (moved up) |
| 7 | Cost of Composability | **Mirrored Ring** — cetz illustration |
| 8 | Doorbell Problem | **Results** |
| 9 | Auto-Batching | Concurrency Families — cetz diagram |
| 10 | Mirrored Ring | Sub-strategies |
| 11 | Results | Cost of Composability — code snippet |
| 12 | General Principle | Closing |
| 13 | What's Next | ~~removed~~ |
| 14 | Closing | |
