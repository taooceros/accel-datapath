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

#v(1.3em)

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

#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card([Why this matters], [If the abstraction is too expensive, then a clean async interface may hide the very performance that makes offload attractive.], fill: c-blue)],
  [#card([Research lens], [This project studies both *programmability* and *cost*: can we make accelerator usage easier without losing the performance regime that batching unlocks?], fill: c-green)],
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

#v(1em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Better API*: accelerator work should look like normal structured async work — in C++ sender/receiver or Rust async style — rather than manual callback-and-poll control flow.
]

// ========================================================================
// HARDWARE CHANGED
// ========================================================================

#pagebreak()

#slide-title[3. But the hardware changed underneath us]

#grid(
  columns: (1fr, 1fr),
  gutter: 16pt,
  [#card(
    [When many async APIs were shaped],
    [I/O and network operations were relatively expensive, so software overhead was easier to hide behind long-latency operations.],
    fill: c-blue,
  )],
  [#card(
    [Today’s tension],
    [Modern on-chip accelerators such as Intel DSA/IAX can complete byte-oriented work fast enough that software assumptions from that older regime may no longer hold.],
    fill: c-orange,
  )],
)

#v(1em)

#note[
  *Conversational takeaway*: async was designed to help manage slow operations. The interesting question is what happens when the operation is no longer that slow.
]

// ========================================================================
// RDMA CLUE
// ========================================================================

#pagebreak()

#slide-title[4. A clue from earlier RDMA work]

#callout(fill: c-blue, stroke: c-accent)[
  Earlier RDMA work suggested that the submission path, not the device itself, could become the main bottleneck at very high message rates.
]

#v(0.8em)

#grid(
  columns: (1.2fr, 0.9fr),
  gutter: 16pt,
  [#card(
    [Observed issue],
    [
      - one MMIO doorbell costs roughly *100--200 ns* \
      - that submission cost dominates at high message rates \
      - batching amortizes one expensive doorbell across many operations
    ],
    fill: c-orange,
  )],
  [#panel[
    #align(center)[
      #text(size: 16pt, weight: "bold", fill: c-title)[One expensive submission]
      #v(0.4em)
      #text(size: 28pt)[1 MMIO]
      #v(0.4em)
      #text(size: 16pt)[driving many ops]
      #v(0.6em)
      #fit-badge([batching], fill: rgb("#16a34a"))
    ]
  ]],
)

#v(0.8em)

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  *Lesson*: batching can move the bottleneck from hardware submission cost to software overhead.
]

// ========================================================================
// HYPOTHESIS
// ========================================================================

#pagebreak()

#slide-title[5. The project hypothesis]

#panel[
  #set text(size: 18pt)
  + If batching changes the regime for RDMA, then Intel on-chip accelerators may show the same structure.
  + If that is true, then even *smaller operations* may become worth offloading once submission is amortized.
  + That would broaden accelerator coverage beyond only very large transfers or heavy kernels.
]

#v(1em)

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr),
  gutter: 10pt,
  [#stage-card([Question], [Do DSA/IAX show the same batching effect?], [research trigger], fill: c-blue, accent: c-accent)],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Method], [compare hardware, mock-hardware, and stripped software paths], [measurement], fill: c-green, accent: rgb("#16a34a"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Goal], [see whether software or hardware becomes the limiting cost], [main decision], fill: c-orange, accent: rgb("#f59e0b"))],
)

// ========================================================================
// WHAT I BUILT
// ========================================================================

#pagebreak()

#slide-title[6. What I built to test it]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Measurement platform],
    [
      - sender/receiver support for all 8 DSA operations \
      - benchmark sweeps across message size, concurrency, and submission strategy \
      - mock-hardware runs to isolate software cost from device cost
    ],
    fill: c-blue,
  )],
  [#card(
    [Controlled software variants],
    [
      - full stdexec baseline \
      - direct path removing `scope.nest()` + `then()` \
      - reusable-op path removing per-op `connect()` + `start()`
    ],
    fill: c-green,
  )],
)

#v(0.8em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  The point was not just to optimize one implementation, but to *measure the cost of the abstraction layers themselves*.
]

// ========================================================================
// MAIN RESULT
// ========================================================================

#pagebreak()

#slide-title[7. Main result: batching exposes the bottleneck shift]

#callout(fill: c-blue, stroke: c-accent)[
  *Main finding*: yes — once submission is amortized, Intel DSA shows the same pattern: hardware becomes cheap enough that the software path becomes a first-order cost.
]

#v(0.7em)

#table(
  columns: (1.7fr, 1fr, 1fr, 1fr),
  table.header([*Path*], [*Mock throughput*], [*Per-op*], [*Real DSA*]),
  [Full stdexec baseline], [26 Mpps], [38 ns], [18 Mpps],
  [Direct path], [42 Mpps], [24 ns], [28 Mpps],
  [Reusable ops], [60--84 Mpps], [16.7--11.9 ns], [34 Mpps],
)

#v(0.8em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#card([Measured delta], [Removing framework layers cuts per-op cost from 38 ns to as low as 11.9 ns.], fill: c-green)],
  [#card([Interpretation], [The dominant savings come from software abstraction layers rather than from changing the hardware.], fill: c-blue)],
  [#card([Why it matters], [This means smaller offloaded operations become plausible once submission is amortized.], fill: c-orange)],
)

// ========================================================================
// SURPRISE
// ========================================================================

#pagebreak()

#slide-title[8. The surprising part]

#grid(
  columns: (1fr, 1fr),
  gutter: 16pt,
  [#card(
    [What we expected],
    [The common assumption is that hardware offload is the expensive part, and software mostly exists to organize requests.],
    fill: c-blue,
  )],
  [#card(
    [What we found],
    [After batching, the control path can be slower than the hardware work itself. The async framework is no longer invisible overhead.],
    fill: c-red,
  )],
)

#v(1em)

#note[
  *Interview version*: this is the moment where the project changes from “how do I use an accelerator nicely?” to “is the software model fundamentally in the wrong performance regime?”
]

// ========================================================================
// CURRENT DIRECTION
// ========================================================================

#pagebreak()

#slide-title[9. How this changed the research direction]

#panel[
  #set text(size: 18pt)
  + The research question is no longer only about improving accelerator programmability.
  + It is now about whether asynchronous frameworks can *carry very high-rate offload requests efficiently* once hardware submission is amortized.
  + In other words: can the software control flow keep up with modern accelerators?
]

#v(0.9em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card([Broader claim], [This is likely not just a DSA result; batching may expose the same software-design problem in RDMA, io_uring, NVMe, RPC, and similar fast async paths.], fill: c-blue)],
  [#card([Practical shift], [stdexec became the measurement vehicle, not the end target. The deeper target is the software structure required for fast offload.], fill: c-green)],
)

// ========================================================================
// TONIC
// ========================================================================

#pagebreak()

#slide-title[10. Current status: moving into Tonic/RPC]

#callout(fill: c-blue, stroke: c-accent)[
  The next step is to ask the same question in a richer software stack: when low-level submission is amortized, where does the cost live in end-to-end RPC?
]

#v(0.8em)

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr, auto, 1fr),
  gutter: 8pt,
  [#stage-card([Codec], [encode/decode buffers], [promising], fill: c-green, accent: rgb("#16a34a"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Payload transforms], [copy, CRC, compression], [strongest fit], fill: c-green, accent: rgb("#16a34a"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Framing], [some byte-path work], [mixed], fill: c-orange, accent: rgb("#f59e0b"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Runtime], [tokio / control flow], [likely CPU], fill: c-red, accent: rgb("#dc2626"))],
)

#v(0.8em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card([Already done], [
    - Tonic component decomposition \
    - concrete interception-point mapping \
    - bounded profiling matrix showing copy/allocation/buffer growth dominate medium and large runs
  ], fill: c-blue)],
  [#card([Next questions], [
    - which payload-path stages are worth offloading? \
    - when does offload help vs hurt? \
    - how much of the remaining cost is fundamental control flow versus avoidable software structure?
  ], fill: c-orange)],
)

#v(0.9em)

#note[
  *Bottom line*: the broader contribution I want is a method for deciding when modern async abstractions help accelerator programming — and when they get in the way.
]
