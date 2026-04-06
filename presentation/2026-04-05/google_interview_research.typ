// Google interview research presentation

#import "../template.typ": callout, card, deck, fit-badge, note, palette, panel, slide-title, stage-card

#show: deck.with(
  margin: (x: 52pt, y: 42pt),
  leading: 0.88em,
  spacing: 0.95em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red

// ========================================================================
// TITLE
// ========================================================================

#align(center + horizon)[
  #text(size: 29pt, weight: "bold", fill: c-title)[Async APIs for Modern Hardware Accelerators]
  #v(0.75em)
  #text(size: 18pt)[When batching makes hardware cheap enough that software becomes the bottleneck]
  #v(0.8em)
  #text(size: 16pt)[Hongtao Zhang]
  #v(0.3em)
  #text(size: 14pt, fill: luma(120))[Apr 5, 2026 · interview talk]
]

#callout(fill: c-blue, stroke: c-accent)[
  *Thesis*: accelerator programming wants an async abstraction, but once submission cost is amortized,
  the async control path itself can become more expensive than the hardware work.
]

// ========================================================================
// QUESTION
// ========================================================================

#pagebreak()

#slide-title[1. The first-principles question]

#panel[
  #set text(size: 18pt)
  + Modern accelerators are *naturally asynchronous devices*.
  + That suggests async frameworks should be the right way to program them.
  + The real question is: *are today’s async APIs still efficient enough for modern fast accelerators?*
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Why this matters],
    [If the abstraction is too expensive, then a clean async interface may hide the very performance that makes offload attractive.],
    fill: c-blue,
  )],
  [#card(
    [Research lens],
    [This project studies both *programmability* and *cost*: can we make accelerator usage easier without losing the performance regime that batching unlocks?],
    fill: c-green,
  )],
)

// ========================================================================
// CALLBACKS
// ========================================================================

#pagebreak()

#slide-title[2. What kind of API do we actually want?]

#grid(
  columns: (1fr, 1fr),
  gutter: 16pt,
  [#card(
    [Low-level callback / polling style],
    [
      `accel_submit(desc, ctx, on_complete)` \
      `while !done { accel_poll() }` \
      \
      - submit descriptor \
      - completion handled through callback or polling \
      - control flow is rebuilt by hand \
      - batching and fallback logic leak into application code
    ],
    fill: c-red,
  )],
  [#card(
    [Structured async style we want],
    [
      *C++ `stdexec`* \
      `dsa.copy(src, dst, n) | then(check) | then(use)` \
      \
      *Rust `async/.await`* \
      `let bytes = accel.copy(src, dst, n).await?;` \
      `send_rpc(bytes).await?;` \
      \
      - composition, sequencing, and reuse \
      - easier integration with I/O / RPC / cancellation / fallback
    ],
    fill: c-green,
  )],
)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Better API*: accelerator work should look like normal structured async work — in C++ sender/receiver or Rust async style — rather than manual callback-and-poll control flow.
]

// ========================================================================
// REGIME CHANGE
// ========================================================================

#pagebreak()

#slide-title[3. Why this question matters now]

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Old async assumption],
    [
      I/O and network operations were expensive enough that software overhead was easier to hide behind long-latency work.
    ],
    fill: c-blue,
  )],
  [#card(
    [Batching clue from RDMA],
    [
      #fit-badge([~100--200 ns MMIO], fill: rgb("#f59e0b"))
      One doorbell can dominate at high message rates, but batching amortizes that cost across many operations.
    ],
    fill: c-orange,
  )],
  [#card(
    [Why this matters now],
    [
      If DSA/IAX shows the same structure, then submission becomes cheap enough that the software control path may become the visible bottleneck.
    ],
    fill: c-green,
  )],
)

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  *Lesson*: batching can make hardware submission cheap enough that software overhead becomes the next bottleneck.
]

// ========================================================================
// SETUP
// ========================================================================

#pagebreak()

#slide-title[4. How I tested it]

#callout(fill: c-blue, stroke: c-accent)[
  *Hypothesis*: if batching hides submission cost in RDMA, Intel DSA should show the same shift: hardware gets cheap, software overhead shows up.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Measurement design],
    [
      - *8 DSA ops* \
      - sweep: message size, concurrency, batching \
      - compare *mock vs real hardware*
    ],
    fill: c-blue,
  )],
  [#card(
    [Controlled software variants],
    [
      *baseline* \
      `scope.nest(dsa.copy(...) | then(record))` \
      *direct* \
      `dsa.copy(...)` \
      remove `scope.nest()` + `then()` \
      *reusable* \
      prebuild op-state, skip per-op `connect()` + `start()`
    ],
    fill: c-green,
  )],
)

#note[*Method*: control one layer at a time — baseline full stdexec, then remove `scope.nest()` + `then()`, then remove per-op `connect()` + `start()` — and measure the delta at each step.]

// ========================================================================
// MAIN RESULT
// ========================================================================

#pagebreak()

#slide-title[5. Main result: software control-path cost can dominate]

#callout(fill: c-blue, stroke: c-accent)[
  *Main finding*: yes — once submission is amortized, Intel DSA shows the same pattern: hardware becomes cheap enough that the software path becomes a first-order cost.
]

#table(
  columns: (1.7fr, 1fr, 1fr, 1fr),
  table.header([*Path*], [*Mock throughput*], [*Per-op*], [*Real DSA*]),
  [Full stdexec baseline], [26.3 Mpps], [38.0 ns], [18 Mpps],
  [Direct path], [41.6 Mpps], [24.0 ns], [28 Mpps],
  [Reusable ops], [59.9 Mpps], [16.7 ns], [34 Mpps],
)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Matched point shown above*: `c=2048`, `msg=8` on mock DSA. Separate hot-cache ceiling: reusable reaches *84 Mpps / 11.9 ns* at `c=32`.
]

#pagebreak()

#slide-title[5. Main result: software control-path cost can dominate]

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#card(
    [Measured delta],
    [At the matched comparison point, removing framework layers cuts per-op cost from 38.0 ns to 16.7 ns — a 56% reduction.],
    fill: c-green,
  )],
  [#card(
    [Interpretation],
    [The dominant savings come from software abstraction layers rather than from changing the hardware path.],
    fill: c-blue,
  )],
  [#card(
    [Why it matters],
    [Once submission is amortized, the software control path is no longer secondary overhead.],
    fill: c-orange,
  )],
)

// ========================================================================
// SURPRISE
// ========================================================================

#pagebreak()

#slide-title[6. What this result does — and does not — say]

#grid(
  columns: (1fr, 1fr),
  gutter: 16pt,
  [#card(
    [What the result supports],
    [In this measured DSA path, generic async control-path overhead can become a first-order cost once hardware submission is amortized.],
    fill: c-blue,
  )],
  [#card(
    [What it does *not* prove],
    [It does not prove that all async APIs are inefficient. It shows that this software stack and this performance regime expose control-path cost very clearly.],
    fill: c-red,
  )],
)

#note[
  *Interview version*: the research turn is from “how do I use an accelerator nicely?” to “when does the software model itself become too expensive for fast devices?”
]

// ========================================================================
// CURRENT DIRECTION
// ========================================================================

#pagebreak()

#slide-title[7. How this changed the research direction]

#panel[
  #set text(size: 18pt)
  + The research question is no longer only about improving accelerator programmability.
  + It is now about whether asynchronous frameworks can *carry very high-rate offload requests efficiently* once hardware submission is amortized.
  + In other words: can the software control flow keep up with modern accelerators?
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Broader hypothesis],
    [This may extend to other fast async paths — RDMA, io_uring, NVMe, RPC — wherever batching makes device submission cheap enough that software structure becomes visible.],
    fill: c-blue,
  )],
  [#card(
    [Practical shift],
    [stdexec became the measurement vehicle, not the end target. The deeper target is the software structure required for fast offload.],
    fill: c-green,
  )],
)

// ========================================================================
// TONIC
// ========================================================================

#pagebreak()

#slide-title[8. Current status: moving into Tonic/RPC]

#callout(fill: c-blue, stroke: c-accent)[
  The next step is to ask the same question in a richer software stack: when low-level submission is amortized, where does the cost live in end-to-end RPC?
]

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr, auto, 1fr),
  gutter: 8pt,
  [#stage-card([Codec], [encode/decode buffers], [architecture-guided], fill: c-green, accent: rgb("#16a34a"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card(
    [Payload transforms],
    [copy, CRC, compression],
    [best current target],
    fill: c-green,
    accent: rgb("#16a34a"),
  )],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Framing], [some byte-path work], [mixed / case-dependent], fill: c-orange, accent: rgb("#f59e0b"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Runtime], [tokio / control flow], [measured as important], fill: c-red, accent: rgb("#dc2626"))],
)

#pagebreak()

#slide-title[8. Current status: moving into Tonic/RPC]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Measured now],
    [
      - bounded profiling matrix across payload size, compression, concurrency, and runtime \
      - medium/large uncompressed runs are dominated by copy, allocation, and buffer growth \
      - runtime crossover depends on workload rather than one global best setting
    ],
    fill: c-blue,
  )],
  [#card(
    [Architecture-guided next steps],
    [
      - codec/body boundaries are the cleanest insertion points \
      - strongest current candidates are copy + CRC and compression / decompression \
      - next question: when does offload help vs hurt once the RPC software path is included?
    ],
    fill: c-orange,
  )],
)

#note[
  *Bottom line*: the broader contribution I want is a method for deciding when modern async abstractions help accelerator programming — and when they get in the way.
]
