// Progress presentation 2026-03-31

#import "../template.typ": callout, card, deck, fit-badge, palette, panel, slide-title, zebra-fill

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
#let tbl-fill = zebra-fill

// ========================================================================
// TITLE
// ========================================================================

#align(center + horizon)[
  #text(size: 29pt, weight: "bold", fill: c-title)[Project Update: Last 2 Months]
  #v(0.8em)
  #text(size: 18pt)[From DSA microbenchmarks to accelerator-aware RPC decomposition]
  #v(0.8em)
  #text(size: 16pt)[Hongtao Zhang]
  #v(0.3em)
  #text(size: 14pt, fill: luma(120))[Mar 31, 2026 · project meeting]
]

#v(1.5em)

#callout(fill: c-blue, stroke: c-accent)[
  *High-level story*: batching makes the hardware cheap enough that the
  software path becomes the real bottleneck, and that insight is now shaping
  both the DSA work and the next RPC/offload direction.
]

// ========================================================================
// OVERVIEW
// ========================================================================

#pagebreak()

#slide-title[What I got done]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [1. Built the DSA measurement platform],
    [
      - completed sender/receiver support for all 8 DSA ops \
      - built a benchmark framework across message size, concurrency, scheduling, and submission strategy \
      - added mock-hardware runs to separate software cost from hardware cost
    ],
    fill: c-blue,
  )],
  [#card(
    [2. Reduced software overhead],
    [
      - compared full stdexec path vs progressively stripped variants \
      - showed that removing framework layers gives large throughput gains \
      - clarified where the per-op cost is really going
    ],
    fill: c-green,
  )],

  [#card(
    [3. Stabilized the hardware side],
    [
      - validated real DSA transfer of the gains \
      - brought up and fixed the IAX `hw-eval` path \
      - confirmed stable scaling for hardware CRC64 sweeps
    ],
    fill: c-orange,
  )],
  [#card(
    [4. Expanded toward RPC],
    [
      - finished a repo-grounded literature review \
      - decomposed the Tonic gRPC stack by component \
      - mapped the concrete interception points for copy / CRC / compression offload
    ],
    fill: rgb("#eef2ff"),
  )],
)

#v(0.9em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  The work now has both a *measured low-level result* and a *clear next application target*.
]

// ========================================================================
// MAIN RESULT
// ========================================================================

#pagebreak()

#slide-title[Main technical result]

#callout(fill: c-blue, stroke: c-accent)[
  *Main finding*: after batching, hardware is no longer the dominant cost for
  small DSA operations; the async software framework is.
]



#table(
  columns: (1.7fr, 1fr, 1fr, 1fr),
  fill: tbl-fill,
  table.header([*Path*], [*Mock throughput*], [*Per-op*], [*Real DSA*]),
  [Full stdexec baseline], [26 Mpps], [38 ns], [18 Mpps],
  [Direct path (remove scope/then)], [42 Mpps], [24 ns], [28 Mpps],
  [Reusable ops (remove connect/start)], [60--84 Mpps], [16.7--11.9 ns], [34 Mpps],
)

#v(0.8em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#card(
    [Measured takeaway],
    [~21 ns of the 38 ns baseline path is framework overhead, not hardware work.],
    fill: c-green,
  )],
  [#card(
    [Method takeaway],
    [Layer-removal plus mock hardware was much more reliable than speculative per-phase accounting.],
    fill: c-blue,
  )],
  [#card(
    [Systems takeaway],
    [Reducing software overhead also improved real-hardware utilization, not just synthetic throughput.],
    fill: c-orange,
  )],
)

// ========================================================================
// WHY IT MATTERS
// ========================================================================

#pagebreak()

#slide-title[Why this matters]

#panel[
  #set text(size: 18pt)
  + *Batching changes the regime.* One expensive doorbell is amortized across many operations.
  + Once that happens, *software overhead becomes visible* and can dominate the end-to-end cost.
  + So the research question is no longer just “is offload fast?” but “when does the software path erase the benefit?”
]

#v(1.0em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [What I now believe],
    [
      The important contribution is broader than one DSA implementation:
      batching exposes a nanosecond-scale software design problem that likely
      appears in RDMA, io_uring, NVMe, and other accelerator/NIC paths.
    ],
    fill: c-blue,
  )],
  [#card(
    [What changed in the plan],
    [
      stdexec is now the measurement vehicle, not the end target.
      The next step is to transfer the methodology and bottleneck analysis
      into production-facing software stacks.
    ],
    fill: c-green,
  )],
)

// ========================================================================
// RPC DIRECTION
// ========================================================================

#pagebreak()

#slide-title[Where the RPC work stands]

#callout(fill: c-blue, stroke: c-accent)[
  We could turn the low-level result into a concrete
  accelerator-aware RPC plan around the Rust/Tonic stack.
]

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#card(
    [Promising offload targets],
    [
      #fit-badge([HIGH], fill: rgb("#16a34a")) #h(0.4em) compression / decompression \
      #fit-badge([HIGH], fill: rgb("#16a34a")) #h(0.4em) copy + CRC \
      #fit-badge([MED], fill: rgb("#f59e0b")) #h(0.4em) framing-adjacent buffer movement
    ],
    fill: c-green,
  )],
  [#card(
    [Likely CPU-resident],
    [
      Tokio scheduling \
      Tower control logic \
      HTTP/2 state machines \
      most protobuf object traversal
    ],
    fill: c-red,
  )],
  [#card(
    [Practical output],
    [
      - identified codec/body boundaries as the best payload hooks \
      - separated metadata interceptors from real payload interception \
      - mapped repo crates for codec, middleware, DSA, and IAX integration
    ],
    fill: c-blue,
  )],
)


#callout(fill: c-orange, stroke: rgb("#f97316"))[
  The near-term RPC goal is *decomposition and crossover analysis*.
]

// ========================================================================
// KB / AI WORKFLOW
// ========================================================================

#pagebreak()

#slide-title[Knowledge base design and new AI workflow]

#callout(fill: c-blue, stroke: c-accent)[
  Shengkai is really interesting in making a AI Research Assistant (do literature review, propose hypothesis, conduct experiment etc.).
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [AI workflow changes],
    [
      - Interaction is important
      - the AI should start from *local evidence first*, not generic web knowledge \
      - `AGENTS.md` is designed as a hierarchy so the AI sees global workflow at the root and only local, path-specific rules in each subtree \
      - repo-local skills + KB lookup + `codemogger` make the workflow structured rather than ad hoc
    ],
    fill: c-blue,
  )],
  [#card(
    [Knowledge base design],
    [
      - moved plans, reports, and specs into one tracked `docs/` layout \
      - built a repo-local Turso KB (Vector DB) under `.turso/knowledge.db` \
      - supports hybrid, FTS-only, and vector search
    ],
    fill: c-green,
  )],
)

#v(0.9em)


// ========================================================================
// TAKEAWAYS / NEXT
// ========================================================================

#pagebreak()

#slide-title[Takeaways and next steps]

#grid(
  columns: (1fr, 1fr),
  gutter: 16pt,
  [#card(
    [Main takeaways],
    [
      - I now have a strong local result: batching exposes software as the bottleneck. \
      - I have a validated methodology: mock hardware + layer removal. \
      - I have a concrete next target: Tonic/gRPC payload-path offload, especially compression and copy/CRC.
    ],
    fill: c-blue,
  )],
  [#card(
    [Immediate next steps],
    [
      - shrink working set and completion-path overhead in the DSA path \
      - profile end-to-end RPC cost by stage \
      - build crossover maps for when accelerator offload helps vs hurts \
      - prototype codec/middleware hooks with CPU fallback
    ],
    fill: c-green,
  )],
)

#v(1.0em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Bottom line*: over the last two months, the project moved from “optimize a DSA path”
  to “use DSA as evidence for a broader batching-regime story, then apply that lens to RPC.”
]
