#set document(title: "Concurrency Scheduling Strategies for Intel DSA", date: auto)
#set page(margin: (top: 2.5cm, bottom: 2.5cm, left: 2.5cm, right: 2.5cm))
#set text(font: "New Computer Modern", size: 11pt)
#set heading(numbering: "1.1")
#set par(justify: true, leading: 0.7em)

#import "@preview/cetz:0.3.4"

// ── Color palette ──
#let col-submit = rgb("#3b82f6")
#let col-hw     = rgb("#f59e0b")
#let col-done   = rgb("#22c55e")
#let col-idle   = rgb("#e5e7eb")
#let col-alloc  = rgb("#ef4444")
#let col-free   = rgb("#a3e635")
#let col-used   = rgb("#60a5fa")
#let col-coro   = rgb("#a78bfa")
#let col-poll   = rgb("#f97316")

// ── Reusable callout box ──
#let keypoint(body) = block(
  width: 100%,
  inset: (x: 12pt, y: 10pt),
  radius: 4pt,
  fill: rgb("#f0f9ff"),
  stroke: (left: 3pt + col-submit),
  body,
)

// ── Title ──
#align(center)[
  #text(size: 18pt, weight: "bold")[Concurrency Scheduling Strategies\ for Intel DSA]
  #v(0.3em)
  #text(size: 12pt, fill: luma(80))[Work Scheduling in the dsa-stdexec Framework]
  #v(1em)
]

// ══════════════════════════════════════════════════════════════
= Overview

The dsa-stdexec framework implements five scheduling strategies for dispatching operations to Intel DSA hardware. Each strategy makes different trade-offs in allocation cost, concurrency control, and programming model.

#figure(
  table(
    columns: (auto, auto, auto, auto, auto),
    align: (left, center, center, center, left),
    stroke: 0.5pt,
    inset: 8pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Strategy*][*Allocs/op*][*Acquire*][*Overlap*][*Key idea*],
    [Sliding Window],     [1 (heap)],  [---],          [Full],  [Atomic `in_flight` counter],
    [No-Alloc],           [0],         [$O(C)$ scan],  [Full],  [Pre-sized slots + `SlotReceiver`],
    [Arena],              [0],         [$O(1)$ pop],   [Full],  [Intrusive free-list + `ArenaReceiver`],
    [Batch],              [1 (heap)],  [---],          [None],  [Barrier: submit $N$, wait all, repeat],
    [Scoped Workers],     [$approx$0], [---],          [Full],  [$N$ coroutines with `co_await`],
  ),
  caption: [Strategy overview. $C$ = concurrency level. "Overlap" indicates whether new operations can start before previous ones complete.]
) <overview-fig>

All five strategies share the same function signature and are dispatched through a $5 times 2$ strategy table (5 patterns $times$ 2 polling modes):

```cpp
using StrategyFn = void(*)(DsaProxy &, exec::async_scope &,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &, LatencyCollector &, OperationType);
```


// ══════════════════════════════════════════════════════════════
= Polling Modes

Every strategy must work with two polling modes. The choice affects threading, latency, and how completions drive progress.

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    // ── Inline (left) ──
    let x0 = 0
    content((x0 + 3.5, 4.2), text(weight: "bold", size: 9pt)[Inline Polling])

    rect((x0, 0), (x0 + 7, 3.8), stroke: 0.8pt, radius: 4pt)
    content((x0 + 3.5, 3.5), text(size: 8pt, fill: luma(100))[Single Thread])

    rect((x0 + 0.3, 2.4), (x0 + 1.8, 3.0), fill: col-submit, radius: 2pt)
    content((x0 + 1.05, 2.7), text(fill: white, size: 7pt)[submit])
    rect((x0 + 1.8, 2.4), (x0 + 2.8, 3.0), fill: col-poll, radius: 2pt)
    content((x0 + 2.3, 2.7), text(fill: white, size: 7pt)[poll])
    rect((x0 + 2.8, 2.4), (x0 + 3.8, 3.0), fill: col-done, radius: 2pt)
    content((x0 + 3.3, 2.7), text(fill: white, size: 7pt)[task])
    rect((x0 + 3.8, 2.4), (x0 + 4.6, 3.0), fill: col-poll, radius: 2pt)
    content((x0 + 4.2, 2.7), text(fill: white, size: 7pt)[poll])
    rect((x0 + 4.6, 2.4), (x0 + 6.1, 3.0), fill: col-submit, radius: 2pt)
    content((x0 + 5.35, 2.7), text(fill: white, size: 7pt)[submit])
    rect((x0 + 6.1, 2.4), (x0 + 6.7, 3.0), fill: col-poll, radius: 2pt)
    content((x0 + 6.4, 2.7), text(fill: white, size: 7pt)[poll])

    rect((x0 + 0.3, 0.8), (x0 + 6.7, 1.4), stroke: (paint: col-hw, thickness: 0.5pt), radius: 2pt)
    content((x0 + 3.5, 1.1), text(size: 7pt, fill: col-hw)[DSA Hardware (async)])

    line((x0 + 1.05, 2.4), (x0 + 1.05, 1.4), stroke: (paint: col-submit, dash: "dashed", thickness: 0.5pt), mark: (end: ">", fill: col-submit))
    line((x0 + 2.3, 1.4), (x0 + 2.3, 2.4), stroke: (paint: col-done, dash: "dashed", thickness: 0.5pt), mark: (end: ">", fill: col-done))
    content((x0 + 3.5, 0.3), text(size: 6.5pt, fill: luma(120))[`wait_start(scope.on_empty(), loop)`])

    // ── Threaded (right) ──
    let x1 = 8.5
    content((x1 + 3.5, 4.2), text(weight: "bold", size: 9pt)[Threaded Polling])

    rect((x1, 2.0), (x1 + 7, 3.8), stroke: 0.8pt, radius: 4pt)
    content((x1 + 3.5, 3.5), text(size: 8pt, fill: luma(100))[Submission Thread])
    for i in range(4) {
      let xb = x1 + 0.3 + i * 1.5
      rect((xb, 2.4), (xb + 1.5, 3.0), fill: col-submit, radius: 2pt)
      content((xb + 0.75, 2.7), text(fill: white, size: 7pt)[submit])
    }

    rect((x1, 0.0), (x1 + 7, 1.8), stroke: 0.8pt, radius: 4pt)
    content((x1 + 3.5, 1.5), text(size: 8pt, fill: luma(100))[Background Poll Thread])
    for i in range(3) {
      let xb = x1 + 0.3 + i * 2.0
      rect((xb, 0.4), (xb + 1.0, 1.0), fill: col-poll, radius: 2pt)
      content((xb + 0.5, 0.7), text(fill: white, size: 7pt)[poll])
      rect((xb + 1.0, 0.4), (xb + 2.0, 1.0), fill: col-done, radius: 2pt)
      content((xb + 1.5, 0.7), text(fill: white, size: 7pt)[task])
    }

    line((x1 + 1.05, 2.4), (x1 + 0.8, 1.0), stroke: (paint: col-submit, dash: "dashed", thickness: 0.5pt), mark: (end: ">", fill: col-submit))
    line((x1 + 2.55, 2.4), (x1 + 2.8, 1.0), stroke: (paint: col-submit, dash: "dashed", thickness: 0.5pt), mark: (end: ">", fill: col-submit))
  }),
  caption: [Inline vs. threaded polling. Inline interleaves submit/poll/task on one thread. Threaded decouples them across two threads, allowing the submission thread to run ahead.]
) <polling-modes>

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [
    === Inline (`PollingRunLoop`)
    A single thread alternates between running stdexec tasks and polling DSA completions. Driven by `wait_start(scope.on_empty(), loop)`.

    - No cross-thread synchronization
    - Submission blocks while polling
  ],
  [
    === Threaded (`DsaScheduler`)
    A background thread polls for completions and runs continuation tasks. The submission thread spawns into an `async_scope` and waits with `sync_wait()`.

    - Submission can run ahead of completion
    - Adds cross-thread synchronization cost
  ],
)

// ══════════════════════════════════════════════════════════════
= Scheduling Strategies

== Sliding Window <sliding-window>

#keypoint[
  *Idea:* Maintain at most $C$ in-flight operations using an atomic counter. When an operation completes, immediately dispatch the next one. Every `scope.spawn()` heap-allocates the operation state.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let gap = 0.08

    content((-0.8, -0.3), text(size: 7pt, fill: luma(100))[time #sym.arrow.r])

    // Lane labels
    for i in range(4) {
      let y = 2.5 - i * 0.8
      content((-1.2, y), text(size: 7.5pt, weight: "bold")[#str(i)])
    }

    // Slot 0: op0 → op4 → op8
    rect((0, 2.25), (1.2, 2.75), fill: col-submit, radius: 2pt)
    content((0.6, 2.5), text(fill: white, size: 6.5pt)[op 0])
    rect((1.2, 2.25), (2.8, 2.75), fill: col-hw, radius: 2pt)
    rect((2.8 + gap, 2.25), (3.2 + gap, 2.75), fill: col-done, radius: 2pt)
    rect((3.3, 2.25), (4.5, 2.75), fill: col-submit, radius: 2pt)
    content((3.9, 2.5), text(fill: white, size: 6.5pt)[op 4])
    rect((4.5, 2.25), (6.1, 2.75), fill: col-hw, radius: 2pt)
    rect((6.1 + gap, 2.25), (6.5 + gap, 2.75), fill: col-done, radius: 2pt)
    rect((6.6, 2.25), (7.8, 2.75), fill: col-submit, radius: 2pt)
    content((7.2, 2.5), text(fill: white, size: 6.5pt)[op 8])

    // Slot 1: op1 → op5
    rect((0.3, 1.45), (1.5, 1.95), fill: col-submit, radius: 2pt)
    content((0.9, 1.7), text(fill: white, size: 6.5pt)[op 1])
    rect((1.5, 1.45), (3.3, 1.95), fill: col-hw, radius: 2pt)
    rect((3.3 + gap, 1.45), (3.7 + gap, 1.95), fill: col-done, radius: 2pt)
    rect((3.8, 1.45), (5.0, 1.95), fill: col-submit, radius: 2pt)
    content((4.4, 1.7), text(fill: white, size: 6.5pt)[op 5])
    rect((5.0, 1.45), (7.0, 1.95), fill: col-hw, radius: 2pt)

    // Slot 2: op2 → op6
    rect((0.6, 0.65), (1.8, 1.15), fill: col-submit, radius: 2pt)
    content((1.2, 0.9), text(fill: white, size: 6.5pt)[op 2])
    rect((1.8, 0.65), (3.0, 1.15), fill: col-hw, radius: 2pt)
    rect((3.0 + gap, 0.65), (3.4 + gap, 1.15), fill: col-done, radius: 2pt)
    rect((3.5, 0.65), (4.7, 1.15), fill: col-submit, radius: 2pt)
    content((4.1, 0.9), text(fill: white, size: 6.5pt)[op 6])
    rect((4.7, 0.65), (6.5, 1.15), fill: col-hw, radius: 2pt)

    // Slot 3: op3 → op7
    rect((0.9, -0.15), (2.1, 0.35), fill: col-submit, radius: 2pt)
    content((1.5, 0.1), text(fill: white, size: 6.5pt)[op 3])
    rect((2.1, -0.15), (3.6, 0.35), fill: col-hw, radius: 2pt)
    rect((3.6 + gap, -0.15), (4.0 + gap, 0.35), fill: col-done, radius: 2pt)
    rect((4.1, -0.15), (5.3, 0.35), fill: col-submit, radius: 2pt)
    content((4.7, 0.1), text(fill: white, size: 6.5pt)[op 7])
    rect((5.3, -0.15), (7.3, 0.35), fill: col-hw, radius: 2pt)

    // Bracket
    line((8.5, -0.15), (8.5, 2.75), stroke: (paint: luma(80), thickness: 0.8pt))
    line((8.4, 2.75), (8.5, 2.75), stroke: (paint: luma(80), thickness: 0.8pt))
    line((8.4, -0.15), (8.5, -0.15), stroke: (paint: luma(80), thickness: 0.8pt))
    content((9.4, 1.3), text(size: 7pt, fill: luma(80))[C = 4])

    // Legend
    let ly = -0.9
    rect((0, ly), (0.5, ly + 0.35), fill: col-submit, radius: 2pt)
    content((1.1, ly + 0.18), text(size: 6.5pt)[submit])
    rect((1.8, ly), (2.3, ly + 0.35), fill: col-hw, radius: 2pt)
    content((3.2, ly + 0.18), text(size: 6.5pt)[hw in-flight])
    rect((4.2, ly), (4.7, ly + 0.35), fill: col-done, radius: 2pt)
    content((5.5, ly + 0.18), text(size: 6.5pt)[completed])
  }),
  caption: [Sliding window (C=4). Each completed operation immediately frees a slot for the next one, keeping the hardware pipeline full.]
) <sliding-window-fig>

*Pseudocode:*
```
for each operation:
    while in_flight >= concurrency: poll()
    in_flight++
    scope.spawn(op_sender | then([&]{ in_flight-- }))
```

*Trade-offs:*
- Simplest strategy (~30 lines); full pipeline overlap
- One heap allocation per `scope.spawn()` call; at >1M ops/sec, allocation cost becomes visible

// ──────────────────────────────────────────────────────────────
== Sliding Window: No-Alloc <noalloc>

#keypoint[
  *Idea:* Pre-allocate $C$ operation slots at startup, each sized at compile time to exactly fit `connect_result_t<Sender, SlotReceiver>`. Use placement `new` instead of heap allocation. A `ready` flag gates reuse.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let sw = 2.2
    let sh = 2.0

    for i in range(4) {
      let x = i * (sw + 0.4)
      let busy = i == 1 or i == 3
      let bg = if busy { col-used.lighten(80%) } else { col-free.lighten(70%) }
      let border = if busy { (paint: col-used, thickness: 1pt) } else { (paint: col-free.darken(20%), thickness: 1pt) }

      rect((x, 0), (x + sw, sh + 0.8), fill: bg, stroke: border, radius: 3pt)
      content((x + sw/2, sh + 0.5), text(size: 7.5pt, weight: "bold")[Slot #str(i)])

      // Storage
      rect((x + 0.15, 0.55), (x + sw - 0.15, sh + 0.05),
        fill: if busy { col-used.lighten(40%) } else { white },
        stroke: 0.3pt, radius: 2pt)
      content((x + sw/2, sh/2 + 0.3), text(size: 6.5pt, fill: if busy { white } else { luma(160) })[
        #if busy [op state] else [empty]
      ])

      // Ready flag
      let fc = if busy { col-alloc } else { col-done }
      rect((x + 0.15, 0.12), (x + sw - 0.15, 0.45), fill: fc.lighten(60%), stroke: 0.3pt, radius: 2pt)
      content((x + sw/2, 0.28), text(size: 6pt)[ready=#if busy [false] else [true]])
    }

    // Scan arrow
    content((-0.9, sh/2 + 0.3), text(size: 7pt, fill: luma(100))[scan #sym.arrow.r])

    // Lifecycle
    let ly = -0.8
    content((5.3, ly), text(size: 7pt, fill: luma(100))[
      Lifecycle: #text(fill: col-free.darken(20%))[ready] #sym.arrow.r
      #text(fill: col-used)[placement new] #sym.arrow.r
      #text(fill: col-hw)[in-flight] #sym.arrow.r
      #text(fill: col-done)[SlotReceiver sets ready] #sym.arrow.r
      #text(fill: col-free.darken(20%))[ready]
    ])
  }),
  caption: [No-alloc slot pool. Slots 0, 2 are available (green); slots 1, 3 hold active operations (blue). The submission loop scans for `ready == true`.]
) <noalloc-fig>

The slot size is computed at compile time by tracing the full sender type chain:

```cpp
constexpr size_t SlotSize = sizeof(connect_result_t<
    async_scope::nest_result_t<Sender | then(Record)>,
    SlotReceiver>);
```

`SlotReceiver` is a minimal receiver that sets `slot->ready = true` on any completion signal (`set_value`, `set_error`, `set_stopped`).

*Trade-offs:*
- Zero heap allocations in the hot path
- $O(C)$ linear scan to find a free slot; at $C <= 32$, this is a few atomic loads
- Different slot sizes for inline vs. threaded (threaded wraps in `schedule() | let_value(...)`)

// ──────────────────────────────────────────────────────────────
== Sliding Window: Arena <arena>

#keypoint[
  *Idea:* Replace the $O(C)$ scan with an $O(1)$ intrusive free-list. On completion, `ArenaReceiver` pushes the slot back onto the list. Inspired by RDMA (ibverbs) and UCX buffer pool designs.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let bw = 1.5
    let bh = 1.0

    // Free list
    content((-1.3, 3.7), text(size: 7.5pt, fill: luma(80), style: "italic")[free_head])
    line((-0.3, 3.7), (0.15, 3.7), mark: (end: ">"), stroke: 0.8pt)

    for i in range(3) {
      let x = i * (bw + 0.7) + 0.2
      rect((x, 3.1), (x + bw, 3.1 + bh), fill: col-free.lighten(70%),
        stroke: (paint: col-free.darken(20%), thickness: 0.8pt), radius: 3pt)
      content((x + bw/2, 3.75), text(size: 7pt, weight: "bold")[Slot #str(i)])
      content((x + bw/2, 3.3), text(size: 6pt, fill: luma(100))[free])
      if i < 2 {
        line((x + bw + 0.05, 3.6), (x + bw + 0.65, 3.6), mark: (end: ">"),
          stroke: (paint: luma(120), thickness: 0.5pt))
      }
    }

    // In-use
    for i in range(2) {
      let x = i * (bw + 0.7) + 0.2
      rect((x, 1.5), (x + bw, 1.5 + bh), fill: col-used.lighten(60%),
        stroke: (paint: col-used, thickness: 0.8pt), radius: 3pt)
      content((x + bw/2, 2.15), text(size: 7pt, weight: "bold")[Slot #str(i + 3)])
      content((x + bw/2, 1.7), text(size: 6pt, fill: white)[in-use])
    }

    // acquire / release labels
    let rx = 7.0
    content((rx, 4.3), text(size: 8.5pt, weight: "bold", fill: col-submit)[acquire() --- $O(1)$ pop])
    line((rx, 4.05), (rx, 3.7), stroke: (paint: col-submit, thickness: 0.7pt), mark: (end: ">", fill: col-submit))
    rect((rx - 0.75, 3.1), (rx + 0.75, 3.1 + bh), fill: col-free.lighten(70%),
      stroke: (paint: col-submit, thickness: 1pt, dash: "dashed"), radius: 3pt)

    content((rx, 1.1), text(size: 8.5pt, weight: "bold", fill: col-done)[release() --- $O(1)$ push])
    line((rx, 1.35), (rx, 1.5), stroke: (paint: col-done, thickness: 0.7pt), mark: (end: ">", fill: col-done))
    rect((rx - 0.75, 1.5), (rx + 0.75, 1.5 + bh), fill: col-used.lighten(60%),
      stroke: (paint: col-done, thickness: 1pt, dash: "dashed"), radius: 3pt)

    // Return arrow
    bezier((rx + 0.75, 2.0), (rx + 2.5, 2.0), (rx + 1.8, 2.8),
      stroke: (paint: col-done, thickness: 0.5pt, dash: "dashed"))
    bezier((rx + 2.5, 2.0), (rx + 0.75, 3.6), (rx + 2.8, 3.0),
      stroke: (paint: col-done, thickness: 0.5pt, dash: "dashed"), mark: (end: ">", fill: col-done))
    content((rx + 2.8, 2.8), text(size: 6pt, fill: col-done)[on\ completion])
  }),
  caption: [Arena free-list. `acquire()` pops from the head; `ArenaReceiver::set_value()` pushes back. No atomics on the list --- safe because both paths execute on the same thread in inline mode.]
) <arena-fig>

*Key difference from No-Alloc:* The custom `ArenaReceiver` replaces `SlotReceiver`. Instead of setting a boolean flag, the receiver directly returns the slot to the pool:

```cpp
struct ArenaReceiver {
    SlotArena *arena;
    OperationSlot *slot;
    void set_value(auto&&...) && noexcept { arena->release(slot); }
};
```

*Trade-offs:*
- $O(1)$ slot acquisition and release via pointer manipulation
- No atomics on the free list itself (single-threaded)
- Slightly different slot size than No-Alloc (different receiver type in `connect_result_t`)

// ──────────────────────────────────────────────────────────────
== Batch <batch>

#keypoint[
  *Idea:* Submit $C$ operations, wait for all to complete (barrier), then submit the next $C$. No pipeline overlap between batches.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let bw = 2.5
    let bh = 2.6

    for bi in range(3) {
      let bx = bi * (bw + 1.2)

      // Batch box
      rect((bx, 0.3), (bx + bw, bh + 0.5), stroke: (paint: luma(160), thickness: 0.8pt, dash: "dashed"), radius: 4pt)
      content((bx + bw/2, bh + 0.2), text(size: 7.5pt, fill: luma(100))[Batch #str(bi)])

      // Ops
      let n = if bi == 2 { 2 } else { 4 }
      for j in range(4) {
        let oy = bh - 0.15 - j * 0.6
        let fill = if j >= n { col-idle } else if bi < 2 { col-done } else { col-hw }
        rect((bx + 0.2, oy - 0.2), (bx + bw - 0.2, oy + 0.2), fill: fill, radius: 2pt)
        if j < n {
          content((bx + bw/2, oy), text(fill: white, size: 6.5pt)[op #str(bi * 4 + j)])
        }
      }

      // Barrier
      if bi < 2 {
        let bx2 = bx + bw + 0.15
        line((bx2, 0.1), (bx2, bh + 0.7), stroke: (paint: col-alloc, thickness: 1.5pt))
        content((bx2 + 0.4, 0.0), text(size: 6pt, fill: col-alloc, weight: "bold")[barrier])
      }
    }

    // Idle annotation
    let ax = 2 * (bw + 1.2)
    line((ax + bw + 0.3, 1.0), (ax + bw + 1.5, 1.0), stroke: (paint: luma(120), thickness: 0.5pt), mark: (end: ">"))
    content((ax + bw + 2.7, 1.0), text(size: 6.5pt, fill: luma(100))[idle slots])

    // Time
    content((-0.5, -0.2), text(size: 7pt, fill: luma(100))[time #sym.arrow.r])
  }),
  caption: [Batch strategy (C=4). Red barriers force all operations to complete before the next batch starts. Grey slots show idle capacity when the batch has fewer than $C$ operations.]
) <batch-fig>

*Pseudocode:*
```
for op_idx = 0 to num_ops step C:
    for i in op_idx .. min(op_idx + C, num_ops):
        scope.spawn(op_sender | then(record))
    wait(scope.on_empty())      // barrier
```

*Trade-offs:*
- Simplest mental model: submit $N$, wait, repeat
- No pipeline overlap -- hardware idles at barriers waiting for the slowest operation
- Requires `loop.reset()` between batches (inline mode sets a stop flag)

// ──────────────────────────────────────────────────────────────
== Scoped Workers <scoped-workers>

#keypoint[
  *Idea:* Spawn $N$ persistent coroutines. Each worker sequentially `co_await`s DSA operations in a round-robin pattern: worker $k$ handles ops $k, k+N, k+2N, ...$
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let lane_h = 0.55
    let lane_gap = 0.25
    let nw = 4

    content((5.2, 3.9), text(size: 9pt, weight: "bold")[4 workers, 12 operations])

    content((-0.6, -0.4), text(size: 7pt, fill: luma(100))[time #sym.arrow.r])

    for w in range(nw) {
      let y = (nw - 1 - w) * (lane_h + lane_gap)

      content((-0.9, y + lane_h/2), text(size: 7.5pt, weight: "bold")[W#str(w)])
      rect((-0.1, y - 0.03), (10.5, y + lane_h + 0.03), fill: luma(248), stroke: 0.3pt, radius: 2pt)

      for r in range(3) {
        let op = w + r * nw
        let xs = 0.15 + r * 3.4

        rect((xs, y), (xs + 0.4, y + lane_h), fill: col-submit, radius: 2pt)
        content((xs + 0.2, y + lane_h/2), text(fill: white, size: 5pt)[s])

        rect((xs + 0.4, y), (xs + 1.7, y + lane_h), fill: col-hw, radius: 2pt)
        content((xs + 1.05, y + lane_h/2), text(fill: white, size: 5.5pt)[co_await])

        rect((xs + 1.7, y), (xs + 2.1, y + lane_h), fill: col-done, radius: 2pt)

        content((xs + 1.05, y + lane_h + 0.2), text(size: 5.5pt, fill: luma(130))[op#str(op)])

        if r < 2 {
          line((xs + 2.1, y + lane_h/2), (xs + 3.4, y + lane_h/2),
            stroke: (paint: col-coro, thickness: 0.4pt, dash: "dashed"),
            mark: (end: ">", fill: col-coro))
        }
      }
    }

    // Legend
    let ly = -1.0
    rect((0, ly), (0.4, ly + 0.3), fill: col-submit, radius: 2pt)
    content((0.9, ly + 0.15), text(size: 6pt)[submit])
    rect((1.5, ly), (1.9, ly + 0.3), fill: col-hw, radius: 2pt)
    content((2.7, ly + 0.15), text(size: 6pt)[co_await])
    rect((3.5, ly), (3.9, ly + 0.3), fill: col-done, radius: 2pt)
    content((4.5, ly + 0.15), text(size: 6pt)[resume])
    line((5.2, ly + 0.15), (6.0, ly + 0.15), stroke: (paint: col-coro, thickness: 0.4pt, dash: "dashed"), mark: (end: ">", fill: col-coro))
    content((6.8, ly + 0.15), text(size: 6pt)[next iter])
  }),
  caption: [Scoped workers with round-robin stride. Each coroutine suspends on `co_await` during hardware execution and resumes to process its next operation.]
) <scoped-workers-fig>

*Core pattern:*
```cpp
exec::task<void> worker(DsaProxy &dsa, ..., size_t worker_id) {
    for (size_t op = worker_id; op < num_ops; op += num_workers)
        co_await dsa_data_move(dsa, src + op*sz, dst + op*sz, sz);
}
```

*Trade-offs:*
- Most natural code -- a simple sequential loop with `co_await`
- One coroutine frame allocation per worker (amortized over all ops, not per-op)
- Only strategy that directly `co_await`s DSA senders; all others use `scope.spawn()`

// ══════════════════════════════════════════════════════════════
= Pipeline Overlap

The critical performance difference between strategies is whether new operations can start before previous ones complete.

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let tw = 11.5

    content((tw/2, 5.0), text(size: 9pt, weight: "bold")[Hardware Utilization: Sliding Window vs. Batch])

    // ── Sliding Window ──
    content((-2.0, 3.8), text(size: 8pt, weight: "bold")[Sliding\ Window])
    rect((-0.3, 3.0), (tw, 4.6), stroke: 0.5pt, radius: 3pt)

    for i in range(8) {
      let x = i * 1.4
      let lane = calc.rem(i, 3)
      let y = 3.15 + lane * 0.45
      rect((x, y), (x + 1.3, y + 0.35), fill: col-hw.lighten(calc.rem(i, 2) * 20%), radius: 2pt)
      content((x + 0.65, y + 0.175), text(fill: white, size: 6pt)[op #str(i)])
    }

    rect((0, 2.65), (tw - 0.3, 2.82), fill: col-done.lighten(40%), radius: 2pt)
    content((tw/2, 2.55), text(size: 6pt, fill: col-done.darken(30%))[continuously utilized])

    // ── Batch ──
    content((-2.0, 1.3), text(size: 8pt, weight: "bold")[Batch])
    rect((-0.3, 0.0), (tw, 2.1), stroke: 0.5pt, radius: 3pt)

    // Batch 0
    for i in range(3) {
      let y = 0.15 + i * 0.45
      rect((0, y), (2.3, y + 0.35), fill: col-hw.lighten(i * 15%), radius: 2pt)
      content((1.15, y + 0.175), text(fill: white, size: 6pt)[op #str(i)])
    }
    line((2.5, -0.1), (2.5, 2.2), stroke: (paint: col-alloc, thickness: 1pt))
    rect((2.3, 0.15), (3.5, 0.50), fill: col-idle, radius: 2pt)
    content((2.9, 0.33), text(size: 5pt, fill: luma(150))[idle])

    // Batch 1
    for i in range(3) {
      let y = 0.15 + i * 0.45
      rect((3.5, y), (5.8, y + 0.35), fill: col-hw.lighten(i * 15%), radius: 2pt)
      content((4.65, y + 0.175), text(fill: white, size: 6pt)[op #str(i + 3)])
    }
    line((6.0, -0.1), (6.0, 2.2), stroke: (paint: col-alloc, thickness: 1pt))
    rect((5.8, 0.15), (7.0, 0.50), fill: col-idle, radius: 2pt)
    content((6.4, 0.33), text(size: 5pt, fill: luma(150))[idle])

    // Batch 2
    for i in range(2) {
      let y = 0.15 + i * 0.45
      rect((7.0, y), (9.3, y + 0.35), fill: col-hw.lighten(i * 15%), radius: 2pt)
      content((8.15, y + 0.175), text(fill: white, size: 6pt)[op #str(i + 6)])
    }

    // Utilization bars
    rect((0, -0.2), (tw - 0.3, -0.04), stroke: 0.3pt, radius: 2pt)
    rect((0, -0.2), (2.3, -0.04), fill: col-done.lighten(40%), radius: 2pt)
    rect((2.3, -0.2), (3.5, -0.04), fill: col-idle, radius: 2pt)
    rect((3.5, -0.2), (5.8, -0.04), fill: col-done.lighten(40%), radius: 2pt)
    rect((5.8, -0.2), (7.0, -0.04), fill: col-idle, radius: 2pt)
    rect((7.0, -0.2), (9.3, -0.04), fill: col-done.lighten(40%), radius: 2pt)
    content((tw/2, -0.45), text(size: 6pt, fill: luma(120))[idle gaps = wasted hardware cycles])
  }),
  caption: [The sliding window keeps the DSA pipeline full. The batch strategy wastes cycles at barriers waiting for the slowest operation.]
) <pipeline-fig>

All sliding-window variants (standard, no-alloc, arena) and scoped workers achieve full pipeline overlap. The batch strategy is the only one with zero overlap --- it trades throughput for simplicity.

// ══════════════════════════════════════════════════════════════
= Orthogonality with Submission Backends

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    // Scheduling column
    let sx = 0
    content((sx + 1.8, 4.6), text(size: 8pt, weight: "bold")[Scheduling])
    let strats = ("Sliding Window", "No-Alloc", "Arena", "Batch", "Scoped Workers")
    for (i, name) in strats.enumerate() {
      let y = 3.9 - i * 0.65
      rect((sx, y - 0.22), (sx + 3.5, y + 0.22), fill: col-submit.lighten(75%), stroke: 0.4pt, radius: 3pt)
      content((sx + 1.75, y), text(size: 7pt)[#name])
    }

    // DsaProxy
    let mx = 5.0
    rect((mx, 0.55), (mx + 2.0, 4.3), fill: luma(242), stroke: (paint: luma(140), thickness: 1pt), radius: 4pt)
    content((mx + 1.0, 4.0), text(size: 7.5pt, weight: "bold")[DsaProxy])
    content((mx + 1.0, 3.4), text(size: 6pt, fill: luma(100))[type erasure])
    for (i, m) in ("submit()", "poll()").enumerate() {
      content((mx + 1.0, 2.6 - i * 0.6), text(size: 6.5pt, font: "DejaVu Sans Mono")[#m])
    }

    // Submission column
    let rx = 8.8
    content((rx + 1.4, 4.6), text(size: 8pt, weight: "bold")[Submission])
    let backs = ("Immediate", "Double-Buffered", "Ring-Buffer")
    let bcols = (col-done, col-hw, col-coro)
    for (i, name) in backs.enumerate() {
      let y = 3.55 - i * 0.85
      rect((rx, y - 0.25), (rx + 2.8, y + 0.25), fill: bcols.at(i).lighten(75%), stroke: 0.4pt, radius: 3pt)
      content((rx + 1.4, y), text(size: 7pt)[#name])
    }

    // Arrows
    for i in range(5) {
      let y = 3.9 - i * 0.65
      line((sx + 3.5, y), (mx, calc.clamp(y, 0.7, 4.1)), stroke: (paint: luma(190), thickness: 0.35pt), mark: (end: ">"))
    }
    for i in range(3) {
      let y = 3.55 - i * 0.85
      line((mx + 2.0, calc.clamp(y, 0.7, 4.1)), (rx, y), stroke: (paint: luma(190), thickness: 0.35pt), mark: (end: ">"))
    }
  }),
  caption: [Any scheduling strategy combines with any submission backend via `DsaProxy` type erasure. The two dimensions are fully independent.]
) <orthogonal-fig>

The `DsaProxy` layer erases the concrete DSA type, so all strategy functions accept the same interface regardless of the submission backend. Batching is handled transparently by the submission backend's `submit()` and `poll()` methods --- scheduling strategies do not need to be aware of whether operations are batched.

// ══════════════════════════════════════════════════════════════
= Zero-Allocation Design Pattern

The no-alloc and arena strategies demonstrate a general pattern for eliminating heap allocation in stdexec-based systems:

#block(
  inset: (x: 16pt, y: 12pt),
  radius: 4pt,
  fill: luma(248),
  stroke: 0.5pt + luma(220),
  width: 100%,
)[
  + *Compute* `sizeof(connect_result_t<Sender, CustomReceiver>)` at compile time.
  + *Pre-allocate* a buffer of that size, aligned to 64 bytes.
  + *Placement-new* the operation state: `new (storage) Op(connect(sender, receiver))`.
  + *Custom receiver* recycles the buffer on completion (flag or free-list).
  + *Destroy* the previous operation state before reusing the slot.
]

This pattern applies to any P2300 sender pipeline where allocation overhead matters.

// ══════════════════════════════════════════════════════════════
= Future Work

- *Lock-free arena* -- Treiber stack variant of `SlotArena` for safe use when submission and completion run on different threads
- *Adaptive concurrency* -- dynamically adjust window size based on observed throughput, similar to TCP congestion control
- *Work stealing* -- replace round-robin partitioning in scoped workers with a work-stealing scheduler for uneven operation latencies
- *Hybrid coroutine + arena* -- combine `co_await` ergonomics with pre-allocated coroutine frames for zero allocation and natural syntax
