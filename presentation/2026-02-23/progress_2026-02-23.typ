// Progress presentation 2026-02-23
// Plain Typst with manual page breaks (no slide framework)

#set page(
  paper: "presentation-16-9",
  margin: (x: 40pt, y: 36pt),
)
#set text(font: "New Computer Modern", size: 16pt)
#set table(stroke: 0.5pt, inset: 6pt)

// Slide title helper
#let slide-title(body) = {
  text(size: 22pt, weight: "bold")[#body]
  v(8pt)
  line(length: 100%, stroke: 0.5pt + gray)
  v(8pt)
}

// ========================================================================
// TITLE
// ========================================================================

#align(center + horizon)[
  #text(size: 28pt, weight: "bold")[stdexec + DSA: Progress Update]

  #v(1em)
  #text(size: 18pt)[Hongtao Zhang]

  #text(size: 14pt, fill: gray)[
    Feb 23, 2026 \
    Covering Jan 21 -- Feb 22 (~4.5 weeks)
  ]
]

// ========================================================================
// CENTRAL QUESTION
// ========================================================================
#pagebreak()

#slide-title[The Question]

#align(center)[
  #block(inset: 16pt, fill: luma(245), radius: 4pt, width: 90%)[
    #text(size: 20pt, weight: "bold")[
      How much do stdexec's abstractions actually cost \
      when you're talking to real hardware?
    ]
  ]
]

#v(1em)

*System*: Intel DSA with C++ P2300 stdexec sender/receiver bindings

*Workload*: Small-message memory ops (8 B transfers), maximizing ops/sec

*Context*: UCX and OpenSHMEM generate streams of small, independent
DMA requests --- exactly what DSA is designed for

// ========================================================================
// WHAT WAS BUILT
// ========================================================================
#pagebreak()

#slide-title[What I Built]

#table(
  columns: (1fr, 2.5fr),
  table.header([*Component*], [*What it does*]),
  [8 DSA op senders], [stdexec sender/receiver for every DSA opcode, with page fault retry],
  [Benchmark framework], [7 sweep dimensions, 3 metric classes, interactive Plotly dashboards],
  [4 submission strategies], [Direct, staging, fixed-ring, MirroredRing (VM-aliased ring buffer)],
  [Mock DSA], [Instant-completion mock --- isolates software cost from hardware],
  [3 optimization strategies], [Progressive layer-removal to measure what each abstraction costs],
)

// ========================================================================
// MOCK HARDWARE
// ========================================================================
#pagebreak()

#slide-title[Mock Hardware Tells You Where the Time Goes]

Swap real DSA for a mock that completes instantly:

- Same benchmark code, same sender/receiver chains --- just no real hardware
- Anything you measure is pure software overhead
- No bistable noise --- clean, reproducible numbers

#v(0.5em)

#table(
  columns: (2fr, 1fr, 1fr),
  table.header([*Mode*], [*Throughput*], [*Per-op*]),
  [Mock DSA (pure software)], [26 Mpps], [38 ns],
  [Real DSA (baseline)], [18 Mpps], [55 ns],
  [Real DSA (cold regime)], [9--11 Mpps], [~100 ns],
)

#v(0.5em)

#text(size: 18pt, weight: "bold")[Most of the per-op cost is software, not hardware.]

// ========================================================================
// WHAT STDEXEC DOES PER OP
// ========================================================================
#pagebreak()

#slide-title[What Happens Per Operation (Full stdexec Path)]

Every 8-byte DSA transfer goes through this pipeline:

#v(0.5em)

#align(center)[
  #block(inset: 10pt, fill: luma(245), radius: 4pt, width: 95%)[
    #set text(size: 14pt)
    #table(
      columns: (auto, 1fr),
      stroke: none,
      inset: 4pt,
      [1.], [*`scope.nest()`* --- register this op with the async scope for lifetime tracking],
      [2.], [*`then(record)`* --- chain a completion callback onto the sender],
      [3.], [*`connect(sender, receiver)`* --- construct a 448-byte operation state object (placement new)],
      [4.], [*`start(op)`* --- memset descriptor + completion record, fill HW fields, submit to device],
      [5.], [*`poll()`* --- walk the task queue, check each op for completion],
      [6.], [*`set_value()`* --- propagate completion back through the receiver chain],
    )
  ]
]

#v(0.5em)

Steps 1--3 are stdexec machinery. Steps 4--6 are actual work.

The question is: how much do 1--3 cost?

// ========================================================================
// THREE STRATEGIES
// ========================================================================
#pagebreak()

#slide-title[Three Strategies: Peel Off Layers One at a Time]

#set text(size: 15pt)

#table(
  columns: (1fr, 3fr, 2fr),
  inset: 5pt,
  table.header([*Strategy*], [*What it does*], [*What's removed*]),
  [`noalloc`],
  [Full stdexec: `scope.nest(sender | then(record))` $arrow.r$ connect $arrow.r$ start],
  [nothing --- baseline],
  [`direct`],
  [Skip scope + then; connect raw DSA sender directly to a receiver],
  [scope.nest, then-adapter (448 $arrow.r$ 384 B op state)],
  [`reusable`],
  [Skip connect + start; pre-allocate op states, just refill descriptors],
  [connect, start, placement-new (no stdexec in hot path)],
)

#set text(size: 16pt)
#v(0.5em)

All three are real benchmarks --- same workload, same buffers, same polling.

The *throughput delta* between adjacent strategies = *measured cost of that layer*.

// ========================================================================
// METHOD
// ========================================================================
#pagebreak()

#slide-title[Why This Works]

No instrumentation needed --- just run the benchmark at each level:

#v(0.5em)

#align(center)[
  #block(inset: 12pt, fill: luma(245), radius: 4pt)[
    #text(size: 16pt)[
      `noalloc` (38 ns/op)
      #h(0.3em) $arrow.r^(- 14 "ns")$ #h(0.3em)
      `direct` (24 ns/op)
      #h(0.3em) $arrow.r^(- 7 "ns")$ #h(0.3em)
      `reusable` (16.7 ns/op)
    ]
  ]
]

#v(1em)

Because each step removes a known set of abstractions, the delta
tells us what those abstractions cost --- without timing individual
function calls.

This avoids the pitfall of analytical cost models (more on that later).

// ========================================================================
// MOCK RESULTS
// ========================================================================
#pagebreak()

#slide-title[Mock DSA Results]

`data_move`, msg_size=8, 3-run average (Mpps --- higher is better):

#table(
  columns: (2fr, 1fr, 1fr, 1fr, 1fr),
  table.header([*Strategy*], [*c=32*], [*c=1024*], [*c=2048*], [*c=4096*]),
  [`noalloc` (baseline)], [28.9], [26.7], [26.3], [25.0],
  [`direct` (no scope/then)], [46.2], [41.4], [41.6], [37.8],
  [`reusable` (no connect/start)], [*83.9*], [62.5], [59.9], [61.3],
)

#v(1em)

Stable across 3 runs (stdev < 1 Mpps). All 8 DSA ops show similar speedups.

// ========================================================================
// DELTAS
// ========================================================================
#pagebreak()

#slide-title[So What Does Each Layer Cost?]

#table(
  columns: (2fr, 2fr, 1fr),
  table.header([*Transition*], [*What's removed*], [*Cost*]),
  [`noalloc` $arrow.r$ `direct`], [`scope.nest()` + `then()` adapters], [*14 ns/op*],
  [`direct` $arrow.r$ `reusable`], [`connect()` + `start()` per-op], [*7 ns/op*],
  [Total stdexec overhead], [All of the above], [*21 ns/op*],
)

#v(1em)

These are measured deltas, not guesses.

The remaining 16.7 ns in `reusable` is the actual per-op work: \
memset descriptors $arrow.r$ fill fields $arrow.r$ submit $arrow.r$ poll $arrow.r$ bookkeeping.

// ========================================================================
// REAL DSA
// ========================================================================
#pagebreak()

#slide-title[Same Story on Real Hardware]

`data_move`, msg_size=8, batch_size=32, real DSA (Mpps):

#table(
  columns: (2fr, 1fr, 1fr, 1fr),
  table.header([*Strategy*], [*c=64*], [*c=256*], [*c=1024*]),
  [`noalloc` (baseline)], [12.5], [15.4], [18.2],
  [`direct`], [14.3], [24.7], [27.5],
  [`reusable`], [15.0], [29.5], [*34.0*],
)

#v(0.5em)

`reusable` at c=1024: *34 Mpps* --- 1.87x over baseline.

With batch_size=64: peaks at *35.3 Mpps* (28.3 ns/op).

Gains are actually *bigger* on real DSA, because less software overhead
also means tighter batching --- the hardware stays busier.

// ========================================================================
// SUMMARY TABLE
// ========================================================================
#pagebreak()

#slide-title[The Tradeoff Space]

#table(
  columns: (1.5fr, 1fr, 1fr, 1fr, 2fr),
  table.header([*Strategy*], [*Mock*], [*Real DSA*], [*Per-op*], [*What's bypassed*]),
  [`noalloc`], [26], [18], [38 ns], [nothing --- full stdexec],
  [`direct`], [42], [28], [24 ns], [scope.nest, then-adapter],
  [`reusable`], [60], [34], [16.7 ns], [+ connect, start],
  [`reusable` c=32], [84], [---], [11.9 ns], [+ cache effects],
)

#v(0.5em)

Units: Mpps. Three design points you can offer users:
- *Safe*: `noalloc` --- full stdexec composability, error handling, lifetime tracking
- *Fast*: `direct` --- still uses stdexec connect/start, 1.6x faster
- *Fastest*: `reusable` --- bypasses stdexec entirely, 2.3x faster

// ========================================================================
// CACHE
// ========================================================================
#pagebreak()

#slide-title[Why c=32 Is So Much Faster]

Each slot is 384--512 bytes. Working set = slots $times$ concurrency:

#v(0.3em)

#table(
  columns: (1fr, 1fr, 1fr, 1fr, 1.5fr),
  inset: 5pt,
  table.header([*Concurrency*], [*reusable*], [*direct*], [*noalloc*], [*Where it lives*]),
  [32], [12 KB], [14 KB], [16 KB], [*L1d* (48 KB)],
  [1024], [384 KB], [464 KB], [528 KB], [L2 (2 MB)],
  [2048], [768 KB], [928 KB], [1056 KB], [L2],
  [4096], [1536 KB], [1856 KB], [2112 KB], [L2/L3 boundary],
)

#v(0.3em)
#text(size: 14pt)[
  Xeon Gold 6438M --- L1d = 48 KB (12-way), L2 = 2 MB (16-way), L3 = 60 MB (15-way)
]

#v(0.3em)

Going from L1 to L2 adds ~4 ns/op --- matches L2 hit latency on Sapphire Rapids.

At c=32 everything fits in L1: 84 Mpps. At c=2048, ~30% of per-op time is cache misses.

// ========================================================================
// AUTO-BATCHING
// ========================================================================
#pagebreak()

#slide-title[Auto-Batching: Free Throughput]

Each DSA submission needs an MMIO doorbell --- that caps you at ~6 Mpps for 8 B messages.

*Fix*: batch 32 descriptors behind one doorbell, transparently.

#v(0.5em)

#table(
  columns: (2fr, 1fr, 1.5fr),
  table.header([*Submission*], [*Doorbells/N ops*], [*Throughput*]),
  [Direct (1 doorbell/op)], [N], [~6 Mpps],
  [MirroredRing (batch=32)], [N/32], [18--35 Mpps],
)

#v(0.5em)

The nice part: *scheduling code doesn't know batching exists*. \
Same sliding-window code works on both backends.

Maps directly to UCX/OpenSHMEM: submit individual RMA ops, \
get batch amortization for free.

// ========================================================================
// BISTABLE
// ========================================================================
#pagebreak()

#slide-title[A Weird Thing: Bistable Throughput]

Same config on real DSA can give ~20 Mpps *or* ~10 Mpps. Depends on luck.

#v(0.5em)

It's a feedback loop:

#block(inset: (left: 16pt))[
  Fewer completions/poll #h(0.3em) $arrow.r$ #h(0.3em)
  more wasted scan time #h(0.3em) $arrow.r$ #h(0.3em)
  longer submission gap \
  $arrow.r$ #h(0.3em)
  smaller effective batch #h(0.3em) $arrow.r$ #h(0.3em)
  fewer completions #h(0.3em) (repeat)
]

#v(0.5em)

Mock DSA doesn't have this (instant completion) --- so it's a
*hardware-software interaction*, not a software bug.

The `arena` strategy falls into the low regime more often than `noalloc`
(10--12 vs 15--18 Mpps), even though the algorithms are similar ---
their memory access patterns tickle the feedback loop differently.

// ========================================================================
// METHODOLOGY LESSON
// ========================================================================
#pagebreak()

#slide-title[Don't Trust Cost Models, Measure Instead]

An earlier analytical breakdown predicted ~11 ns savings from three targeted optimizations.

We got *~2--3 ns*. Off by 4x.

#v(0.5em)

#table(
  columns: (2fr, 1fr, 1fr, 2fr),
  inset: 5pt,
  table.header([*Optimization*], [*Predicted*], [*Actual*], [*Why wrong*]),
  [Proxy $arrow.r$ fn pointers], [4 ns], [~1 ns], [Proxy SBO was already cheap],
  [Indexed queue], [3--5 ns], [~0 ns], [100% completion on mock = no waste],
  [SlotArena free-list], [3 ns], [~1 ns], [Only matters at c=4096],
)

#v(0.5em)

The layer-removal approach got it right because it measures actual throughput. \
Reasoning about code structure is fine for hypotheses, not for predictions.

// ========================================================================
// NEXT STEPS
// ========================================================================
#pagebreak()

#slide-title[What's Next]

#table(
  columns: (2fr, 2fr, 1fr),
  table.header([*Question*], [*How*], [*Status*]),
  [Do gains hold on real DSA?], [Run real-hardware benchmarks], [*Done*],
  [Can IndexedQueue fix bistable?], [Real DSA: indexed vs linked-list], [Next],
  [Why does `arena` regress?], [Compare memory access patterns], [Planned],
  [What exactly costs 16.7 ns?], [`rdtsc` microbenchmarks per phase], [Planned],
  [Multi-device scaling?], [Add second DSA device], [Future],
  [Can stdexec itself be cheaper?], [Profile connect/start internals], [Open],
)

// ========================================================================
// END
// ========================================================================
#pagebreak()

#align(center + horizon)[
  #text(size: 24pt, weight: "bold")[
    26 Mpps $arrow.r$ 84 Mpps (mock) \
    18 Mpps $arrow.r$ 34 Mpps (real DSA)
  ]

  #v(1.5em)

  #text(size: 18pt)[
    stdexec adds 21 ns/op. We know where it goes. \
    And we have three clear design points for users.
  ]
]
