// Progress presentation 2026-03-03
// Plain Typst with manual page breaks (no slide framework)

#import "../template.typ": deck, palette, slide-title, zebra-fill, callout, note, panel

#show: deck.with(
  margin: (x: 52pt, y: 44pt),
  leading: 0.85em,
  spacing: 1.1em,
  table-stroke: 0.4pt + luma(200),
)

#import "@preview/cetz:0.3.4"
#import "@preview/timeliney:0.4.0"

// ── Colors ─────────────────────────────────────────────────────
#let c-title = palette.title
#let c-accent = palette.accent
#let c-head = palette.head
#let c-row = palette.row
#let c-blue = palette.blue
#let c-orange = palette.orange
// diagram palette
#let c-app = rgb("#dbeafe")
#let c-sched = rgb("#bfdbfe")
#let c-sub = rgb("#bbf7d0")
#let c-hw = rgb("#fef08a")
#let c-submit = rgb("#3b82f6")
#let c-inflt = rgb("#f59e0b")
#let c-done = rgb("#22c55e")
#let c-coro = rgb("#a78bfa")
#let c-barrier = rgb("#ef4444")

// ── Helpers ────────────────────────────────────────────────────
#let tbl-fill = zebra-fill

// ========================================================================
// TITLE
// ========================================================================
#align(center + horizon)[
  #text(size: 30pt, weight: "bold", fill: c-title)[stdexec + DSA: Progress Update]
  #v(1.2em)
  #text(size: 19pt)[Hongtao Zhang]
  #v(0.4em)
  #text(size: 14pt, fill: luma(120))[Mar 3, 2026 · Advisor meeting]
]

// ========================================================================
// AGENDA
// ========================================================================
#pagebreak()

#slide-title[Today's Agenda]
#v(0.4em)
#panel(width: 88%)[
  #set text(size: 18pt)
  #set par(leading: 1.1em)
  + *Undergraduate project* --- Delegation-Style Lock: a brief overview
  + *Auto-batching* --- transparent, framework-layer batching as the central thesis
  + *Concurrency strategy landscape* --- the benchmark suite and the design space
]
#v(1em)
#note[
  #text(size: 16pt)[*Heads-up*: I will be back around 3/15.]
  #v(0.3em)
  #text(size: 16pt)[*Meeting with Shu Ran* (last week, with Shengkai & Yibo): discussed their RDMA modeling — could generalize to peripheral accelerator modeling. Also: Shu Ran has a friend using Intel DSA for Soft-RoCE.]
  #v(0.3em)
  #text(size: 16pt)[*Questions*: Should I attend ASPLOS this year? · Claude subscription for research use?]
]
#v(0.8em)
#text(size: 15pt, fill: luma(130))[
  Builds on the Feb 23 update. Focus today: the batching insight first,
  then the concurrency strategies that measure what it costs.
]

// ========================================================================
// UNDERGRADUATE PROJECT
// ========================================================================
#pagebreak()

#slide-title[Undergrad Project: Usage-Fair Delegation Locks]

#callout[
  *Thesis*: traditional locks force a tradeoff — fairness costs
  throughput. Delegation locks can break this tradeoff because shared
  data stays in the combiner's L1 cache regardless of serving order.
]

#v(0.7em)

- *Background*: in a _delegation lock_, waiters enqueue a closure;
  a _combiner_ executes critical sections on their behalf

- *Problem*: fair traditional locks (MCS, CFL) pay for fairness with
  expensive cross-core cache migration — ~30% throughput loss

- *Our approach*: reorder waiters by cumulative usage inside the
  combiner (FC-PQ) — fairness is essentially free because the shared
  data never moves between cores

- *Next*: full micro-benchmark suite on a 128-thread machine
  — CS ratio sweeps, response-time CDFs, asymmetric contention —
  targeting PPoPP 2027 (Aug deadline)


// ========================================================================
// SYSTEM OVERVIEW
// ========================================================================
#pagebreak()

#slide-title[System Overview]

#text(size: 15pt, fill: luma(90))[
  The project wraps Intel DSA in a C++ async framework. Two layers sit
  between user code and the hardware.
]

#v(0.5em)

#align(center)[
  #cetz.canvas(length: 1.3cm, {
    import cetz.draw: *

    let w = 13.0
    let h = 1.55 // taller boxes — only 3 layers now
    let gap = 0.72
    let layers = (
      (
        c-app,
        "Application / UCX transport layer",
        "scheduling policy (sliding window, batch, …)  ·  co_await dsa_data_move(src, dst, 8 bytes)",
      ),
      (
        c-sub,
        "Submission Backend",
        "how descriptors reach the hardware:  immediate doorbell  /  ring-buffer  /  mirrored-ring",
      ),
      (c-hw, "Intel DSA Hardware", "async DMA engine — executes operations in hardware"),
    )
    for (i, (fc, title, sub)) in layers.enumerate() {
      let y = -(i * (h + gap))
      rect((0, y - h), (w, y), fill: fc, stroke: 0.6pt + luma(155), radius: 4pt)
      content((w / 2, y - h * 0.30), text(size: 12pt, weight: "bold")[#title])
      content((w / 2, y - h * 0.70), text(size: 9pt, fill: luma(55))[#sub])
      if i < 2 {
        line(
          (w / 2, y - h - 0.04),
          (w / 2, y - h - gap + 0.08),
          stroke: (paint: luma(110), thickness: 0.8pt),
          mark: (end: ">", fill: luma(110)),
        )
      }
    }
  })
]

// ========================================================================
// THE MMIO DOORBELL PROBLEM
// ========================================================================
#pagebreak()

#slide-title[The MMIO Doorbell Problem]

Submitting a descriptor to Intel DSA requires an MMIO write — a special
instruction (`MOVDIR64B`) that sends 64 bytes directly to the device's
work-queue portal. This transaction is expensive.

#v(0.7em)

#callout[
  *One doorbell per operation* caps throughput at roughly *6 million ops/sec*
  for 8-byte copies, no matter how fast the software is.
  The hardware is starved by submission overhead, not execution time.
]

#v(0.7em)

*Fix*: DSA has a built-in batch opcode. Submit one batch descriptor that
points to an array of up to 32 regular descriptors — *one doorbell amortized
across 32 operations*:

#table(
  columns: (2.2fr, 1.2fr, 1.5fr),
  fill: tbl-fill,
  table.header([*Mode*], [*Doorbells for N ops*], [*Throughput*]),
  [Immediate — 1 doorbell per op], [$N$], [~6 Mpps],
  [Batch — batch size 32], [$⌈N/32⌉$], [*18–35 Mpps*],
)

#v(0.5em)
*3–6× higher throughput* from one design decision in the submission layer,
with zero changes to the scheduling logic above it.

// ========================================================================
// AUTO-BATCHING: TRANSPARENT BY DESIGN
// ========================================================================
#pagebreak()

#slide-title[Auto-Batching: Transparent by Design]

#callout[
  *The scheduling code doesn't know batching exists.*
  It calls `submit(descriptor)` one at a time.
  The backend decides whether to fire immediately or accumulate.
  Swap one backend for another — throughput changes, strategy unchanged.
]

#v(0.6em)

#align(center)[
  #cetz.canvas(length: 1.2cm, {
    import cetz.draw: *

    let w = 14.0 // total width
    let bw = 4.2 // backend box width
    let bg = 0.35 // gap between backend boxes
    let b0 = (w - 3 * bw - 2 * bg) / 2 // left margin for backends

    // ── Scheduling box ──
    rect((0, 2.2), (w, 3.5), fill: c-sched, stroke: 0.6pt + luma(155), radius: 4pt)
    content((w / 2, 3.02), text(size: 12pt, weight: "bold")[Scheduling Strategy])
    content((w / 2, 2.48), text(size: 9pt)[sliding window / batch / scoped workers — same code regardless of backend])

    // ── Fan-out arrows ──
    let centers = (b0 + bw / 2, b0 + bw + bg + bw / 2, b0 + 2 * (bw + bg) + bw / 2)
    for cx in centers {
      line((w / 2, 2.2), (cx, 1.72), stroke: (paint: luma(110), thickness: 0.7pt), mark: (end: ">", fill: luma(110)))
    }
    content((w / 2 + 1.0, 1.95), text(size: 8pt, fill: luma(90))[submit(desc)])

    // ── Backend boxes ──
    let backends = (
      (rgb("#fce7f3"), "DirectSubmitter", "1 doorbell / op", "~6 Mpps"),
      (rgb("#e0f2fe"), "RingSubmitter", "1 doorbell / 32 ops", "~18 Mpps"),
      (rgb("#d1fae5"), "MirroredRingSubmitter", "no wrap-around", "~35 Mpps"),
    )
    for (i, (fc, name, detail, mpps)) in backends.enumerate() {
      let x = b0 + i * (bw + bg)
      rect((x, 0.2), (x + bw, 1.65), fill: fc, stroke: 0.6pt + luma(155), radius: 4pt)
      content((x + bw / 2, 1.25), text(size: 10pt, weight: "bold")[#name])
      content((x + bw / 2, 0.80), text(size: 8.5pt)[#detail])
      content((x + bw / 2, 0.38), text(size: 11pt, fill: c-accent, weight: "bold")[#mpps])
    }

    // ── DSA hardware ──
    rect((0, -1.3), (w, -0.15), fill: c-hw, stroke: 0.6pt + luma(155), radius: 4pt)
    content((w / 2, -0.68), text(size: 12pt, weight: "bold")[Intel DSA Hardware])

    // ── Arrows down to hardware ──
    for cx in centers {
      line((cx, 0.2), (cx, -0.13), stroke: (paint: luma(120), thickness: 0.7pt), mark: (end: ">", fill: luma(120)))
    }
  })
]

#v(0.4em)
#text(size: 15pt, fill: luma(80))[
  Analogy: TCP Nagle — application writes individual bytes; TCP
  accumulates them into packets. The application doesn't change.
]

// ========================================================================
// HOW THE MIRRORED RING WORKS
// ========================================================================
#pagebreak()

#slide-title[How the Mirrored Ring Eliminates Wrap-Around]

A hardware batch requires a *contiguous* descriptor array. A plain ring
buffer must seal early when a batch crosses the ring boundary.

#v(0.5em)

#align(center)[
  #cetz.canvas(length: 1.1cm, {
    import cetz.draw: *

    let sw = 1.15 // slot width
    let sh = 0.70 // slot height
    let n = 8 // slots in ring

    // ── TOP: Plain ring ──────────────────────────────────────
    content((n * sw / 2, 4.65), text(
      size: 11pt,
      weight: "bold",
      fill: rgb("#991b1b"),
    )[Plain ring — batch at boundary gets split])

    for i in range(n) {
      let x = i * sw
      let fc = if i >= 6 { rgb("#fecaca") } else { luma(232) }
      rect((x, 3.2), (x + sw - 0.1, 3.2 + sh), fill: fc, stroke: 0.5pt + luma(155), radius: 3pt)
      content((x + sw / 2 - 0.05, 3.56), text(size: 9pt)[#str(i)])
    }
    // Wrap boundary
    line((n * sw - 0.05, 3.0), (n * sw - 0.05, 4.05), stroke: (paint: rgb("#dc2626"), thickness: 1.6pt, dash: "dashed"))
    content((n * sw + 0.1, 2.86), text(size: 8pt, fill: rgb("#dc2626"), weight: "bold")[wrap])

    // Slots 0, 1 (overflow — shown faded, would not be contiguous)
    for i in range(2) {
      let x = (n + i) * sw
      rect(
        (x, 3.2),
        (x + sw - 0.1, 3.2 + sh),
        fill: rgb("#fecaca"),
        stroke: (paint: rgb("#ef4444"), thickness: 0.6pt, dash: "dashed"),
        radius: 3pt,
      )
      content((x + sw / 2 - 0.05, 3.56), text(size: 9pt, fill: luma(110))[#str(i)])
    }

    // Red brackets
    let brk(x1, x2, y, label) = {
      line((x1, y - 0.05), (x1, y + 0.15), stroke: (paint: rgb("#dc2626"), thickness: 1pt))
      line((x1, y + 0.15), (x2, y + 0.15), stroke: (paint: rgb("#dc2626"), thickness: 1pt))
      line((x2, y + 0.15), (x2, y - 0.05), stroke: (paint: rgb("#dc2626"), thickness: 1pt))
      content(((x1 + x2) / 2, y + 0.38), text(size: 8pt, fill: rgb("#dc2626"))[#label])
    }
    brk(6 * sw, n * sw - 0.12, 3.2 + sh, "slots 6, 7")
    brk(n * sw + 0.03, (n + 2) * sw - 0.12, 3.2 + sh, "slots 0, 1 — NOT contiguous!")

    // ── BOTTOM: Mirrored ring ─────────────────────────────────
    content((n * sw / 2, 2.08), text(
      size: 11pt,
      weight: "bold",
      fill: rgb("#166534"),
    )[Mirrored ring — batch crosses boundary naturally])

    // First mapping
    for i in range(n) {
      let x = i * sw
      let fc = if i >= 6 { rgb("#bbf7d0") } else { luma(232) }
      rect((x, 1.1), (x + sw - 0.1, 1.1 + sh), fill: fc, stroke: 0.5pt + luma(155), radius: 3pt)
      content((x + sw / 2 - 0.05, 1.46), text(size: 9pt)[#str(i)])
    }
    // Mirror mapping
    for i in range(n) {
      let x = (n + i) * sw
      let fc = if i < 2 { rgb("#bbf7d0") } else { luma(244) }
      rect(
        (x, 1.1),
        (x + sw - 0.1, 1.1 + sh),
        fill: fc,
        stroke: (paint: luma(185), thickness: 0.4pt, dash: "dashed"),
        radius: 3pt,
      )
      content((x + sw / 2 - 0.05, 1.46), text(size: 9pt, fill: luma(105))[#str(i)])
    }

    // VA region labels
    let lab_y = 0.65
    let tick_h = 0.2
    line((0, lab_y + tick_h), (0, lab_y), stroke: 0.5pt + luma(130))
    line((0, lab_y), (n * sw, lab_y), stroke: 0.5pt + luma(130))
    line((n * sw, lab_y), (n * sw, lab_y + tick_h), stroke: 0.5pt + luma(130))
    content((n * sw / 2, lab_y - 0.28), text(size: 8pt, fill: luma(75))[VA: base … base+R  (first mmap)])

    line((n * sw, lab_y + tick_h), (n * sw, lab_y), stroke: (paint: luma(160), thickness: 0.5pt, dash: "dashed"))
    line((n * sw, lab_y), (2 * n * sw, lab_y), stroke: (paint: luma(160), thickness: 0.5pt, dash: "dashed"))
    line((2 * n * sw, lab_y), (2 * n * sw, lab_y + tick_h), stroke: (
      paint: luma(160),
      thickness: 0.5pt,
      dash: "dashed",
    ))
    content((n * sw + n * sw / 2, lab_y - 0.28), text(
      size: 8pt,
      fill: luma(125),
    )[VA: base+R … base+2R  (mirror mmap — same physical pages)])

    // Green bracket
    let bx1 = 6 * sw
    let bx2 = (n + 2) * sw - 0.10
    let by = 1.1 + sh + 0.12
    line((bx1, 1.1 + sh), (bx1, by), stroke: (paint: rgb("#16a34a"), thickness: 1pt))
    line((bx1, by), (bx2, by), stroke: (paint: rgb("#16a34a"), thickness: 1pt))
    line((bx2, by), (bx2, 1.1 + sh), stroke: (paint: rgb("#16a34a"), thickness: 1pt))
    content(((bx1 + bx2) / 2, by + 0.28), text(
      size: 9pt,
      fill: rgb("#166534"),
      weight: "bold",
    )[slots 6, 7, 0, 1 — contiguous in VA])
  })
]

#v(0.3em)
#text(size: 14pt, fill: luma(90))[
  *Constraint*: ring size must be a multiple of the OS page size (4 KB),
  since `mmap` maps at page granularity. No early batch seal. No conditional branch.
]

// ========================================================================
// RESULTS
// ========================================================================
#pagebreak()

#slide-title[Results: Batching Impact]

#text(size: 14pt, fill: luma(110))[
  Sliding window · inline polling · concurrency 16 · 8-byte messages
  #h(1em) _(concurrency 16 is conservative; hardware supports up to 128 in-flight)_
]

#v(0.5em)

#table(
  columns: (2fr, 1.2fr, 1.2fr, 0.9fr),
  fill: tbl-fill,
  table.header([*Operation*], [*Immediate (Mpps)*], [*Ring-Buffer (Mpps)*], [*Speedup*]),
  [`data_move`], [2.60], [4.29], [*1.65×*],
  [`mem_fill`], [4.13], [8.41], [*2.04×*],
  [`compare`], [4.61], [8.09], [*1.76×*],
  [`compare_value`], [4.69], [8.08], [*1.72×*],
  [`dualcast`], [4.62], [7.99], [*1.73×*],
  [`crc_gen`], [4.68], [8.13], [*1.74×*],
  [`copy_crc`], [2.36], [2.96], [*1.25×*],
  [`cache_flush`], [2.41], [2.94], [*1.22×*],
)

#v(0.5em)

At concurrency 1024 (near-peak utilization): *34 Mpps* with the `reusable`
strategy — 1.87× over the full-framework baseline.

#v(0.3em)

#callout[
  Without batching, each op costs ~160 ns of MMIO overhead — so the 21 ns
  framework cost is only 13% of total and barely matters. With batching, the
  MMIO cost drops to ~5 ns per op, making framework overhead *4× larger than
  the hardware cost*. *Batching is what makes optimizing the framework worth doing.*
]

// ========================================================================
// CONCURRENCY STRATEGY FAMILIES  (slide A — overview)
// ========================================================================
#pagebreak()

#slide-title[Concurrency Strategies: Three Families]

#table(
  columns: (1.3fr, 3fr, 1.5fr),
  fill: tbl-fill,
  table.header([*Family*], [*How it works*], [*HW utilization*]),
  [*Sliding Window*],
  [Keeps _C_ ops in-flight at all times. As soon as one completes, submit the next immediately.],
  [Always busy — highest throughput],

  [*Batch*],
  [Fires all _C_ ops together in one batch descriptor. Waits at a barrier for all to finish, then fires the next batch.],
  [Idle gap between batches],

  [*Scoped Workers*],
  [Spawns _N_ coroutines; each `co_await`s its own op. Structured concurrency — coroutines cancel cleanly on scope exit.],
  [Depends on _N_ and op latency],
)

#v(0.7em)

#text(size: 15pt, fill: luma(80))[
  All three families work with both inline polling (submitter thread polls)
  and threaded polling (dedicated poller thread) — a separate axis of variation.
]

// ========================================================================
// CONCURRENCY STRATEGY FAMILIES  (slide B — timelines)
// ========================================================================
#pagebreak()

#slide-title[Concurrency Strategies: Timelines (C = 3)]

// ── legend ──────────────────────────────────────────────────────
#let swatch(fc) = box(width: 18pt, height: 9pt, fill: fc, radius: 2pt, baseline: -1pt)
#grid(
  columns: (auto, auto) * 5,
  column-gutter: (4pt, 16pt) * 4 + (4pt,),
  align: horizon,
  swatch(c-submit),
  text(size: 13pt)[submit],
  swatch(c-inflt),
  text(size: 13pt)[HW in-flight],
  swatch(c-done),
  text(size: 13pt)[done / polled],
  swatch(c-coro),
  text(size: 13pt)[co_await],
  swatch(luma(212)),
  text(size: 13pt)[idle],
)

#v(0.3em)

// ── shared timeline config ──────────────────────────────────────
#let tl-sw = 8pt
#let tl-si(fc) = (style: (stroke: tl-sw + fc))
#let tl-opts = (
  show-grid: true,
  line-style: (stroke: tl-sw + luma(150)),
  spacing: 1pt,
  heading-spacing: 2pt,
  tasks-vline: false,
  grid-style: (stroke: (dash: "dashed", thickness: 0.4pt, paint: luma(210))),
  cell-line-style: (stroke: 0.6pt + luma(170)),
)

#set text(size: 10pt)

// ── Sliding Window ──────────────────────────────────────────────
// 6 rows: each op is one-shot. As ops 0–2 complete, ops 3–5 fill their slots.
// Shows the "window" sliding forward — always 3 in-flight.
#text(size: 13pt, weight: "bold")[Sliding Window]
#v(0.1em)
#timeliney.timeline(..tl-opts, {
  import timeliney: *
  headerline(group(([← time →], 9)))
  task(
    [op 0],
    (from: 0.0, to: 0.5, ..tl-si(c-submit)),
    (from: 0.5, to: 3.0, ..tl-si(c-inflt)),
    (from: 3.0, to: 3.25, ..tl-si(c-done)),
  )
  task(
    [op 1],
    (from: 0.5, to: 1.0, ..tl-si(c-submit)),
    (from: 1.0, to: 3.5, ..tl-si(c-inflt)),
    (from: 3.75, to: 4.0, ..tl-si(c-done)),
  )
  task(
    [op 2],
    (from: 1.0, to: 1.5, ..tl-si(c-submit)),
    (from: 1.5, to: 4.0, ..tl-si(c-inflt)),
    (from: 4.5, to: 4.75, ..tl-si(c-done)),
  )
  task([op 3], (from: 3.25, to: 3.75, ..tl-si(c-submit)), (from: 3.75, to: 6.25, ..tl-si(c-inflt)))
  task([op 4], (from: 4.0, to: 4.5, ..tl-si(c-submit)), (from: 4.5, to: 7.0, ..tl-si(c-inflt)))
  task([op 5], (from: 4.75, to: 5.25, ..tl-si(c-submit)), (from: 5.25, to: 7.75, ..tl-si(c-inflt)))
})

#v(0.2em)

// ── Batch ───────────────────────────────────────────────────────
// 6 rows like sliding window, but barrier between batches.
// Ops 0–2 = first batch, ops 3–5 = second batch.
#text(size: 13pt, weight: "bold")[Batch]
#v(0.1em)
#timeliney.timeline(..tl-opts, {
  import timeliney: *
  headerline(group(([← time →], 9)))
  task(
    [op 0],
    (from: 0.0, to: 0.5, ..tl-si(c-submit)),
    (from: 0.5, to: 3.0, ..tl-si(c-inflt)),
    (from: 3.0, to: 3.25, ..tl-si(c-done)),
    (from: 3.25, to: 4.25, style: (stroke: tl-sw + luma(212))),
  )
  task(
    [op 1],
    (from: 0.5, to: 1.0, ..tl-si(c-submit)),
    (from: 1.0, to: 3.5, ..tl-si(c-inflt)),
    (from: 3.5, to: 3.75, ..tl-si(c-done)),
    (from: 3.75, to: 4.25, style: (stroke: tl-sw + luma(212))),
  )
  task(
    [op 2],
    (from: 1.0, to: 1.5, ..tl-si(c-submit)),
    (from: 1.5, to: 4.0, ..tl-si(c-inflt)),
    (from: 4.0, to: 4.25, ..tl-si(c-done)),
  )
  task([op 3], (from: 4.5, to: 5.0, ..tl-si(c-submit)), (from: 5.0, to: 7.5, ..tl-si(c-inflt)))
  task([op 4], (from: 5.0, to: 5.5, ..tl-si(c-submit)), (from: 5.5, to: 8.0, ..tl-si(c-inflt)))
  task([op 5], (from: 5.5, to: 6.0, ..tl-si(c-submit)), (from: 6.0, to: 8.5, ..tl-si(c-inflt)))
  milestone(
    at: 4.35,
    style: (stroke: (paint: c-barrier, thickness: 1.5pt, dash: "dashed")),
    text(size: 9pt, weight: "bold", fill: c-barrier)[barrier],
  )
})

#v(0.2em)

// ── Scoped Workers ──────────────────────────────────────────────
// 3 persistent coroutines — each worker loops: submit → co_await → done → repeat.
#text(size: 13pt, weight: "bold")[Scoped Workers]
#v(0.15em)
#timeliney.timeline(..tl-opts, {
  import timeliney: *
  headerline(group(([← time →], 9)))
  task(
    [w 0],
    (from: 0.0, to: 0.5, ..tl-si(c-submit)),
    (from: 0.5, to: 3.0, ..tl-si(c-coro)),
    (from: 3.0, to: 3.25, ..tl-si(c-done)),
    (from: 3.25, to: 3.75, ..tl-si(c-submit)),
    (from: 3.75, to: 6.25, ..tl-si(c-coro)),
  )
  task(
    [w 1],
    (from: 0.5, to: 1.0, ..tl-si(c-submit)),
    (from: 1.0, to: 3.5, ..tl-si(c-coro)),
    (from: 3.75, to: 4.0, ..tl-si(c-done)),
    (from: 4.0, to: 4.5, ..tl-si(c-submit)),
    (from: 4.5, to: 7.0, ..tl-si(c-coro)),
  )
  task(
    [w 2],
    (from: 1.0, to: 1.5, ..tl-si(c-submit)),
    (from: 1.5, to: 4.0, ..tl-si(c-coro)),
    (from: 4.5, to: 4.75, ..tl-si(c-done)),
    (from: 4.75, to: 5.25, ..tl-si(c-submit)),
    (from: 5.25, to: 7.75, ..tl-si(c-coro)),
  )
})

#set text(size: 17pt)

// SLIDING WINDOW SUB-STRATEGIES
// ========================================================================
#pagebreak()

#slide-title[Sliding Window: Peeling Off Abstraction Layers]

Five variants that progressively remove stdexec machinery — the performance
difference between adjacent rows measures the cost of the removed layer:

#table(
  columns: (1fr, 2.6fr, 1fr),
  fill: tbl-fill,
  table.header([*Strategy*], [*What changes vs. previous*], [*~ns/op (mock)*]),
  [`sliding_window`], [Baseline — full stdexec pipeline with heap alloc per op], [~35],
  [`noalloc`], [Pre-allocated slots; placement-new instead of heap alloc], [~35],
  [`arena`], [$O(1)$ free-list slot acquire instead of $O(C)$ linear scan], [~35],
  [`direct`], [Bypass `scope.nest()` and `then()` — no lifetime tracking], [~13],
  [`reusable`], [Bypass `connect()` and `start()` — no stdexec in hot path], [~8],
)

#v(0.7em)

#text(fill: luma(80))[
  All five use the same hardware, the same poll loop, and the same buffers.
  The only difference is how much stdexec machinery runs per operation.
]

// ========================================================================
// COST OF COMPOSABILITY
// ========================================================================
#pagebreak()

#slide-title[The Cost of Composability]

#text(size: 15pt, fill: luma(90))[
  Three variants — each removes one stdexec layer. Mock DSA hardware isolates
  pure software cost with zero real-work variance.
]

#v(0.5em)

#grid(columns: (1fr, 1fr, 1fr), column-gutter: 16pt)[
  // ── noalloc ──
  #block(width: 100%, fill: c-head, radius: 4pt, inset: (x: 12pt, y: 8pt))[
    #text(weight: "bold")[`noalloc`]
    #h(1fr)
    #text(fill: c-accent, weight: "bold")[38 ns]
    #linebreak()
    #text(size: 12pt, fill: luma(100))[full stdexec pipeline]
  ]
  #v(0.35em)
  #block(width: 100%, radius: 3pt, fill: luma(248), inset: (x: 10pt, y: 8pt), clip: true)[
    #set text(font: "Latin Modern Mono", size: 12pt)
    scope.nest(\
    #h(1em)dsa_data_move(\
    #h(2em)dsa, src, dst, 8)\
    #h(1em)| then([&]\{\
    #h(2em)record();\
    #h(1em)})\
    );
  ]
  #v(0.3em)
  #text(size: 13pt, fill: luma(75))[
    Constructs `NestSender` + `ThenSender` + 448 B op-state per op.
    Full lifetime tracking + error propagation.
  ]
][
  // ── direct ──
  #block(width: 100%, fill: c-head, radius: 4pt, inset: (x: 12pt, y: 8pt))[
    #text(weight: "bold")[`direct`]
    #h(1fr)
    #text(fill: c-accent, weight: "bold")[24 ns]
    #linebreak()
    #text(size: 12pt, fill: luma(100))[-14 ns: drop scope + then]
  ]
  #v(0.35em)
  #block(width: 100%, radius: 3pt, fill: luma(248), inset: (x: 10pt, y: 8pt), clip: true)[
    #set text(font: "Latin Modern Mono", size: 12pt)
    connect(\
    #h(1em)dsa_data_move(\
    #h(2em)dsa, src, dst, 8),\
    #h(1em)DirectReceiver\{slot}\
    ); // 384 B op-state
  ]
  #v(0.3em)
  #text(size: 13pt, fill: luma(75))[
    Removes `scope.nest()` and `then()`. No lifetime guard,
    no then-adapter wrapper object.
  ]
][
  // ── reusable ──
  #block(width: 100%, fill: c-head, radius: 4pt, inset: (x: 12pt, y: 8pt))[
    #text(weight: "bold")[`reusable`]
    #h(1fr)
    #text(fill: c-accent, weight: "bold")[16.7 ns]
    #linebreak()
    #text(size: 12pt, fill: luma(100))[-7 ns: drop connect + start]
  ]
  #v(0.35em)
  #block(width: 100%, radius: 3pt, fill: luma(248), inset: (x: 10pt, y: 8pt), clip: true)[
    #set text(font: "Latin Modern Mono", size: 12pt)
    slot.refill(\
    #h(1em)src, dst, 8);\
    slot.submit();\
    // raw MOVDIR64B\
    // poll checks record\
    // no set_value chain
  ]
  #v(0.3em)
  #text(size: 13pt, fill: luma(75))[
    Bypasses `connect()` and `start()`. No stdexec machinery
    in the hot path at all.
  ]
]

#v(0.5em)

#callout[
  *"Composability cost"*: each layer (`scope.nest`, `then`, `connect`, `start`)
  constructs and destroys a wrapper type on every operation.
  Each adds a safety feature — and costs cycles.
  Total stdexec overhead: *21 ns* out of 38 ns baseline.
]

// ========================================================================
// CLOSING
// ========================================================================
#pagebreak()

#align(center + horizon)[
  #text(size: 30pt, weight: "bold", fill: c-title)[
    6 Mpps #h(0.3em)$arrow.r$#h(0.3em) 35 Mpps
  ]

  #v(0.8em)

  #text(size: 19pt)[
    Transparent auto-batching: scheduling code unchanged, \
    submission backend swapped, throughput multiplied.
  ]

  #v(1.4em)

  #text(size: 15pt, fill: luma(120))[
    Once batching brings MMIO cost below framework overhead, \
    optimizing the framework has real leverage.
  ]
]
