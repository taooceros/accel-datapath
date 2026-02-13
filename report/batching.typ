#set document(title: "Hardware Batch Submission Strategies for Intel DSA", date: auto)
#set page(margin: 2.5cm)
#set text(font: "New Computer Modern", size: 11pt)
#set heading(numbering: "1.1")
#set par(justify: true, leading: 0.7em)

#import "@preview/cetz:0.3.4"

// ── Color palette ──
#let col-desc   = rgb("#3b82f6")   // blue — descriptor
#let col-batch  = rgb("#f59e0b")   // amber — batch descriptor
#let col-hw     = rgb("#f97316")   // orange — hardware / doorbell
#let col-done   = rgb("#22c55e")   // green — completed / free
#let col-idle   = rgb("#e5e7eb")   // light gray — unused
#let col-alloc  = rgb("#ef4444")   // red — blocked / waste
#let col-fill   = rgb("#60a5fa")   // light blue — filling
#let col-inflt  = rgb("#a78bfa")   // violet — in-flight
#let col-ring   = rgb("#0ea5e9")   // sky — ring buffer

// ── Callout box ──
#let keypoint(body) = block(
  width: 100%,
  inset: (x: 12pt, y: 10pt),
  radius: 4pt,
  fill: rgb("#f0f9ff"),
  stroke: (left: 3pt + col-desc),
  body,
)

#align(center)[
  #text(size: 18pt, weight: "bold")[Hardware Batch Submission Strategies\ for Intel DSA]
  #v(0.3em)
  #text(size: 12pt, fill: luma(80))[Transparent Batching in the dsa-stdexec Framework]
  #v(1em)
]

// ══════════════════════════════════════════════════════════════
= Overview

DSA processes work by receiving 64-byte descriptors through MMIO. Each doorbell write (`MOVDIR64B`/`ENQCMD`) is expensive --- it triggers a cache-coherency transaction to the device. The DSA _batch opcode_ (`0x01`) lets software submit a pointer to a contiguous descriptor array with a single doorbell, amortizing this cost.

#figure(
  table(
    columns: (auto, auto, auto, auto, auto),
    align: (left, center, center, center, left),
    stroke: 0.5pt,
    inset: 8pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Strategy*][*Doorbells/$N$*][*In-flight*][*Blocks?*][*Key idea*],
    [Immediate],       [$N$],           [N/A],  [No],  [1 doorbell per descriptor],
    [Double-Buffered], [$ceil(N\/B)$],  [2],    [Yes], [Two fixed arrays, swap on submit],
    [Fixed-Ring],      [$ceil(N\/B)$],  [16],   [No],  [Ring of fixed-size batch entries],
    [Ring-Buffer],     [$ceil(N\/B)$],  [16],   [No],  [Shared descriptor ring + batch metadata ring],
  ),
  caption: [Strategy overview. $N$ = operations, $B$ = max batch size (typically 32).]
) <overview-fig>

#keypoint[
  *Key result:* Ring-buffer batching achieves *1.2--2.0x* higher message rates than double-buffered batching across all 8 DSA operations, by eliminating submission blocking and improving batch utilization. Fixed-ring batching (ablation study) confirms that most of the gain comes from deeper in-flight capacity (16 vs. 2 batches), not memory packing.
]

All four strategies present the same interface through `DsaProxy` type erasure --- scheduling strategies work identically across all backends.

// ══════════════════════════════════════════════════════════════
= How Batching Works

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    // ── Immediate: N doorbells ──
    content((3.5, 5.2), text(size: 9pt, weight: "bold")[Immediate: $N$ doorbells])

    for i in range(6) {
      let x = i * 1.1
      rect((x, 3.5), (x + 0.9, 4.2), fill: col-desc, radius: 2pt)
      content((x + 0.45, 3.85), text(fill: white, size: 6pt)[desc #str(i)])
      line((x + 0.45, 3.5), (x + 0.45, 2.7), stroke: (paint: col-hw, thickness: 0.7pt),
        mark: (end: ">", fill: col-hw))
    }

    rect((0, 2.2), (6.5, 2.65), fill: col-hw.lighten(75%), stroke: 0.5pt, radius: 2pt)
    content((3.25, 2.42), text(size: 7pt)[DSA Work Queue Portal])
    content((3.25, 1.8), text(size: 6.5pt, fill: luma(120))[6 descriptors = 6 MMIO writes])

    // ── Batch: 1 doorbell ──
    let x0 = 8.5
    content((x0 + 3.5, 5.2), text(size: 9pt, weight: "bold")[Batch: 1 doorbell])

    for i in range(6) {
      let x = x0 + i * 0.9
      rect((x, 3.5), (x + 0.85, 4.2), fill: col-desc, radius: 2pt)
      content((x + 0.425, 3.85), text(fill: white, size: 6pt)[desc #str(i)])
    }
    rect((x0 - 0.05, 3.4), (x0 + 5.45, 4.3), stroke: (paint: col-batch, thickness: 1pt, dash: "dashed"), radius: 3pt)

    rect((x0 + 1.5, 4.6), (x0 + 3.9, 5.05), fill: col-batch, radius: 2pt)
    content((x0 + 2.7, 4.82), text(fill: white, size: 6pt)[batch desc (opcode 0x01)])

    line((x0 + 2.7, 4.6), (x0 + 2.7, 4.35), stroke: (paint: col-batch, thickness: 0.6pt, dash: "dashed"),
      mark: (end: ">", fill: col-batch))

    line((x0 + 2.7, 3.4), (x0 + 2.7, 2.7), stroke: (paint: col-hw, thickness: 1pt),
      mark: (end: ">", fill: col-hw))

    rect((x0, 2.2), (x0 + 5.5, 2.65), fill: col-hw.lighten(75%), stroke: 0.5pt, radius: 2pt)
    content((x0 + 2.75, 2.42), text(size: 7pt)[DSA Work Queue Portal])
    content((x0 + 2.75, 1.8), text(size: 6.5pt, fill: luma(120))[6 descriptors = 1 MMIO write])
  }),
  caption: [Immediate submission writes each descriptor individually. Batch submission sends one batch descriptor pointing to a contiguous array.]
) <doorbell-fig>

Hardware constraints for the batch opcode:
- Descriptor array must be *contiguous* and *64-byte aligned*
- Array must remain valid until hardware finishes DMA-reading it
- Minimum 2 descriptors per batch (unless device supports `Batch1`)
- Max batch size is per-WQ configurable (`accfg_wq_get_max_batch_size`)

// ══════════════════════════════════════════════════════════════
= Submission Strategies

== Immediate (`DsaBase`)

#keypoint[
  *Idea:* Submit each descriptor immediately via doorbell. $N$ operations = $N$ MMIO writes. Best for large transfers where doorbell cost is amortized by DMA time.
]

- Lowest per-operation latency (no staging)
- Highest doorbell cost

== Double-Buffered (`DsaBatchBase`) <double-buffered>

#keypoint[
  *Idea:* Stage descriptors into one of two fixed-size arrays. When full, submit as a hardware batch. Double-buffering allows filling one buffer while hardware reads the other.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let bw = 5.5
    let sh = 0.45

    content((6.5, 5.5), text(size: 9pt, weight: "bold")[Double-Buffered Staging])

    // ── Buffer A (active, being filled) ──
    content((-1.8, 4.5), text(size: 8pt, weight: "bold", fill: col-fill)[Buffer A])
    content((-1.8, 4.1), text(size: 6.5pt, fill: col-fill)[(filling)])
    rect((0, 3.6), (bw, 4.9), stroke: (paint: col-fill, thickness: 1pt), radius: 3pt)

    for i in range(3) {
      let x = 0.1 + i * 1.1
      rect((x, 4.1), (x + 1.0, 4.1 + sh), fill: col-desc, radius: 2pt)
      content((x + 0.5, 4.1 + sh/2), text(fill: white, size: 6pt)[desc #str(i)])
    }
    for i in range(2) {
      let x = 0.1 + (i + 3) * 1.1
      rect((x, 4.1), (x + 1.0, 4.1 + sh), fill: col-idle, stroke: 0.3pt, radius: 2pt)
    }

    content((2.8, 3.75), text(size: 6pt, fill: luma(120))[staged\_count\_ = 3])

    // ── Buffer B (submitted, hardware reading) ──
    content((-1.8, 2.5), text(size: 8pt, weight: "bold", fill: col-inflt)[Buffer B])
    content((-1.8, 2.1), text(size: 6.5pt, fill: col-inflt)[(in-flight)])
    rect((0, 1.6), (bw, 2.9), stroke: (paint: col-inflt, thickness: 1pt), radius: 3pt)

    for i in range(4) {
      let x = 0.1 + i * 1.1
      rect((x, 2.1), (x + 1.0, 2.1 + sh), fill: col-inflt.lighten(30%), radius: 2pt)
      content((x + 0.5, 2.1 + sh/2), text(fill: white, size: 6pt)[desc #str(i)])
    }
    rect((0.1 + 4 * 1.1, 2.1), (0.1 + 4 * 1.1 + 1.0, 2.1 + sh), fill: col-alloc.lighten(70%), stroke: 0.3pt, radius: 2pt)
    content((0.1 + 4 * 1.1 + 0.5, 2.1 + sh/2), text(size: 5.5pt, fill: col-alloc)[wasted])

    line((bw + 0.3, 2.35), (bw + 1.8, 2.35), stroke: (paint: col-hw, thickness: 0.7pt),
      mark: (end: ">", fill: col-hw))
    content((bw + 2.8, 2.35), text(size: 7pt, fill: col-hw)[HW DMA\ reading])

    // Swap arrow
    let sx = 6.5
    bezier((sx, 4.25), (sx + 1.5, 3.5), (sx + 1.5, 4.25),
      stroke: (paint: luma(140), thickness: 0.6pt))
    bezier((sx + 1.5, 3.5), (sx, 2.35), (sx + 1.5, 2.35),
      stroke: (paint: luma(140), thickness: 0.6pt), mark: (end: ">"))
    content((sx + 2.0, 3.5), text(size: 6.5pt, fill: luma(100))[swap on\ submit])

    content((6.5, 1.1), text(size: 7pt, fill: col-alloc)[
      If Buffer A fills before B is released,\ `submit()` blocks (spin-wait).
    ])

    rect((0, 0.6), (2.0, 1.0), fill: col-inflt.lighten(70%), stroke: 0.3pt, radius: 2pt)
    content((1.0, 0.8), text(size: 6pt)[batch\_comp\[B\].status])
    line((2.1, 0.8), (3.0, 0.8), stroke: (paint: luma(140), thickness: 0.4pt), mark: (end: ">"))
    content((3.9, 0.8), text(size: 6pt, fill: luma(120))[polled until != 0])
  }),
  caption: [Double-buffered batching. Buffer A fills while hardware reads Buffer B. Fixed-size arrays waste unused slots. Submission blocks if both buffers are in use.]
) <double-buf-fig>

*Limitations:*
- *Submission blocks* on previous batch (only 2 buffers)
- *Effective batch size driven by poll frequency* -- at low concurrency, most batches are size 1--4
- *Fixed-size buffers* waste space (32 slots reserved regardless of usage)

== Fixed-Ring (`DsaFixedRingBatchBase`) <fixed-ring>

#keypoint[
  *Idea:* A ring of 16 fixed-size batch entries, each owning a contiguous 32-slot descriptor array plus a batch completion record. Eliminates the double-buffered blocking problem (16 slots vs. 2) without the wrap-around complexity of a shared descriptor ring.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    content((6, 6.5), text(size: 9pt, weight: "bold")[Fixed-Ring Batch Architecture])

    // ── Batch entry ring ──
    content((-1.8, 5.0), text(size: 8pt, weight: "bold")[Batch\ Entry Ring])
    content((-1.8, 4.4), text(size: 6pt, fill: luma(100))[16 entries])

    let bw = 2.2
    let bh = 2.2
    let entries = (
      ("InFlight", col-inflt.lighten(30%), "3/32 used"),
      ("InFlight", col-inflt.lighten(50%), "4/32 used"),
      ("Filling",  col-fill,               "2/32 used"),
      ("Free",     col-idle,               "32 reserved"),
      ("Free",     col-idle,               "32 reserved"),
    )

    for (i, (state, fill_c, info)) in entries.enumerate() {
      let x = i * (bw + 0.2)
      rect((x, 3.0), (x + bw, 3.0 + bh), fill: fill_c, stroke: 0.3pt, radius: 3pt)
      content((x + bw/2, 4.8), text(size: 6.5pt, weight: "bold", fill: if state == "Free" { luma(160) } else { white })[#state])

      // Draw descriptor slots inside each entry
      let sw = 0.22
      let rows = 2
      let cols = 4
      for r in range(rows) {
        for c in range(cols) {
          let sx = x + 0.2 + c * (sw + 0.05)
          let sy = 4.0 + r * (sw + 0.05)
          let slot_fill = if state == "Free" {
            col-idle.lighten(30%)
          } else if state == "Filling" and (r * cols + c) >= 2 {
            col-idle
          } else {
            if state == "Filling" { col-fill.lighten(20%) } else { fill_c.lighten(20%) }
          }
          rect((sx, sy), (sx + sw, sy + sw), fill: slot_fill, stroke: 0.2pt, radius: 1pt)
        }
      }

      content((x + bw/2, 3.6), text(size: 5.5pt, fill: if state == "Free" { luma(160) } else { white.darken(10%) })[#info])

      if state != "Free" {
        rect((x + 0.2, 3.1), (x + bw - 0.2, 3.35), fill: white.transparentize(70%), stroke: 0.2pt, radius: 2pt)
        content((x + bw/2, 3.22), text(size: 5pt)[batch\_comp])
      }
    }

    line((0, 2.8), (0, 3.0), stroke: (paint: col-done, thickness: 1pt))
    content((0, 2.6), text(size: 6pt, fill: col-done, weight: "bold")[batch\_head])

    line((2 * (bw + 0.2) + bw/2, 2.8), (2 * (bw + 0.2) + bw/2, 3.0), stroke: (paint: col-fill, thickness: 1pt))
    content((2 * (bw + 0.2) + bw/2, 2.6), text(size: 6pt, fill: col-fill, weight: "bold")[batch\_fill])

    let ly = 2.0
    rect((0, ly), (0.6, ly + 0.3), fill: col-inflt.lighten(30%), radius: 2pt)
    content((1.4, ly + 0.15), text(size: 6pt)[in-flight])
    rect((2.3, ly), (2.9, ly + 0.3), fill: col-fill, radius: 2pt)
    content((3.6, ly + 0.15), text(size: 6pt)[filling])
    rect((4.4, ly), (5.0, ly + 0.3), fill: col-idle, radius: 2pt)
    content((5.6, ly + 0.15), text(size: 6pt)[free])

    content((6, 1.5), text(size: 6.5pt, fill: col-alloc)[
      Each entry reserves 32 descriptor slots\ regardless of actual batch size.
    ])
  }),
  caption: [Fixed-ring architecture. Each batch entry owns a private 32-slot descriptor array. Unused slots (grey) are wasted when batches are small.]
) <fixed-ring-fig>

*How it works:*
- `submit()` copies the descriptor into the current Filling entry's private array. When `count >= max_batch_size_`, the entry is sealed and submitted as a hardware batch.
- `poll()` seals any partial Filling entry, then reclaims completed InFlight entries in order.
- No wrap-around logic needed --- each entry's descriptor array is independent.

*Trade-offs:*
- *No blocking* (same as ring-buffer) -- 16 batch slots provide sufficient depth
- *Simpler state management* -- only `batch_fill_` and `batch_head_`, no `desc_head_`/`desc_tail_`
- *Wastes descriptor space* -- each entry reserves $B$ slots ($32 times 64$ B = 2 KB) even for small batches; total $16 times 2$ KB = 32 KB
- *No contiguity requirement* across batches -- each entry is self-contained

This strategy serves as an ablation baseline: it isolates the benefit of having 16 in-flight batches (vs. double-buffered's 2) from the ring-buffer's tighter memory packing.

== Ring-Buffer (`DsaRingBatchBase`) <ring-buffer>

#keypoint[
  *Idea:* Two separate ring buffers --- a descriptor ring (256 slots) for tightly-packed storage and a batch metadata ring (16 entries) for tracking in-flight batches. Full batches auto-submit; partial batches drain on `poll()`. 16 in-flight batches eliminate blocking.
]

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    content((7, 7.5), text(size: 9pt, weight: "bold")[Two-Ring Architecture])

    // ══ Descriptor Ring ══
    content((-1.8, 6.5), text(size: 8pt, weight: "bold")[Descriptor\ Ring])
    content((-1.8, 5.9), text(size: 6pt, fill: luma(100))[256 slots])

    let nslots = 14
    let sw = 0.85
    for i in range(nslots) {
      let x = i * (sw + 0.05)

      let (fill_c, label) = if i < 3 {
        (col-inflt.lighten(30%), str(i))
      } else if i >= 3 and i < 6 {
        (col-inflt.lighten(50%), str(i))
      } else if i >= 6 and i < 9 {
        (col-fill, str(i))
      } else {
        (col-idle, "")
      }

      rect((x, 5.8), (x + sw, 6.4), fill: fill_c, stroke: 0.3pt, radius: 2pt)
      if label != "" {
        content((x + sw/2, 6.1), text(fill: if fill_c == col-idle { luma(180) } else { white }, size: 5.5pt)[#label])
      }
    }

    line((0, 5.6), (0, 5.8), stroke: (paint: col-done, thickness: 1pt))
    content((0, 5.4), text(size: 6pt, fill: col-done, weight: "bold")[head])

    line((9 * (sw + 0.05), 5.6), (9 * (sw + 0.05), 5.8), stroke: (paint: col-desc, thickness: 1pt))
    content((9 * (sw + 0.05), 5.4), text(size: 6pt, fill: col-desc, weight: "bold")[tail])

    let bracket-y = 5.1
    line((0, bracket-y), (0, bracket-y + 0.15), stroke: 0.5pt)
    line((0, bracket-y), (2.6, bracket-y), stroke: 0.5pt)
    line((2.6, bracket-y), (2.6, bracket-y + 0.15), stroke: 0.5pt)
    content((1.3, bracket-y - 0.2), text(size: 5.5pt, fill: col-inflt)[batch 0])

    line((2.7, bracket-y), (2.7, bracket-y + 0.15), stroke: 0.5pt)
    line((2.7, bracket-y), (5.3, bracket-y), stroke: 0.5pt)
    line((5.3, bracket-y), (5.3, bracket-y + 0.15), stroke: 0.5pt)
    content((4.0, bracket-y - 0.2), text(size: 5.5pt, fill: col-inflt)[batch 1])

    line((5.4, bracket-y), (5.4, bracket-y + 0.15), stroke: 0.5pt)
    line((5.4, bracket-y), (8.0, bracket-y), stroke: 0.5pt)
    line((8.0, bracket-y), (8.0, bracket-y + 0.15), stroke: 0.5pt)
    content((6.7, bracket-y - 0.2), text(size: 5.5pt, fill: col-fill)[batch 2 (filling)])

    // ══ Batch Metadata Ring ══
    content((-1.8, 3.5), text(size: 8pt, weight: "bold")[Batch\ Metadata])
    content((-1.8, 2.9), text(size: 6pt, fill: luma(100))[16 entries])

    let bw = 2.6
    let bh = 1.4
    let entries = (
      ("InFlight", col-inflt.lighten(30%), "start=0\ncount=3"),
      ("InFlight", col-inflt.lighten(50%), "start=3\ncount=3"),
      ("Filling",  col-fill,               "start=6\ncount=3"),
      ("Free",     col-idle,               ""),
      ("Free",     col-idle,               ""),
    )

    for (i, (state, fill_c, info)) in entries.enumerate() {
      let x = i * (bw + 0.2)
      rect((x, 2.3), (x + bw, 2.3 + bh), fill: fill_c, stroke: 0.3pt, radius: 3pt)
      content((x + bw/2, 3.35), text(size: 6.5pt, weight: "bold", fill: if state == "Free" { luma(160) } else { white })[#state])
      if info != "" {
        let lines = info.split("\n")
        content((x + bw/2, 2.9), text(size: 5.5pt, fill: if state == "Free" { luma(160) } else { white.darken(10%) })[
          #lines.at(0)\ #lines.at(1)
        ])
      }
      if state != "Free" {
        rect((x + 0.2, 2.4), (x + bw - 0.2, 2.65), fill: white.transparentize(70%), stroke: 0.2pt, radius: 2pt)
        content((x + bw/2, 2.52), text(size: 5pt)[batch\_comp])
      }
    }

    line((0, 2.1), (0, 2.3), stroke: (paint: col-done, thickness: 1pt))
    content((0, 1.9), text(size: 6pt, fill: col-done, weight: "bold")[batch\_head])

    line((2 * (bw + 0.2) + bw/2, 2.1), (2 * (bw + 0.2) + bw/2, 2.3), stroke: (paint: col-fill, thickness: 1pt))
    content((2 * (bw + 0.2) + bw/2, 1.9), text(size: 6pt, fill: col-fill, weight: "bold")[batch\_fill])

    for i in range(3) {
      let mx = i * (bw + 0.2) + bw/2
      let dx = if i == 0 { 1.3 } else if i == 1 { 4.0 } else { 6.7 }
      line((mx, 3.7), (dx, 5.05), stroke: (paint: luma(180), thickness: 0.4pt, dash: "dashed"))
    }

    let ly = 1.2
    rect((0, ly), (0.6, ly + 0.3), fill: col-inflt.lighten(30%), radius: 2pt)
    content((1.4, ly + 0.15), text(size: 6pt)[in-flight])
    rect((2.3, ly), (2.9, ly + 0.3), fill: col-fill, radius: 2pt)
    content((3.6, ly + 0.15), text(size: 6pt)[filling])
    rect((4.4, ly), (5.0, ly + 0.3), fill: col-idle, radius: 2pt)
    content((5.6, ly + 0.15), text(size: 6pt)[free])
    content((7.5, ly + 0.15), text(size: 6pt, fill: luma(120))[Batches use exactly as many desc slots as needed])
  }),
  caption: [Ring-buffer architecture. Descriptor ring stores descriptors contiguously; batch metadata ring tracks start, count, and state. A batch of 3 uses exactly 3 slots, not 32.]
) <ring-buf-fig>

*How it works:*
- `submit()` appends to the descriptor ring and the current Filling batch. When `count >= max_batch_size_`, the batch is auto-submitted.
- `poll()` seals and submits any partial Filling batch, then reclaims completed batches (advancing `desc_head_`).
- *Wrap-around:* if the next slot crosses the ring boundary, the current batch is sealed early and a new one starts at index 0 (ensures contiguity for DMA).

*Advantages over double-buffered:*
- *No blocking* -- 16 batch slots vs. 2
- *Better utilization* -- consistently full batches instead of partial ones from frequent `poll()`
- *No wasted space* -- batches use exactly the descriptor slots they need

=== Wrap-Around

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let sw = 0.9
    let sh = 0.55
    let nslots = 8

    content((3.5, 3.5), text(size: 9pt, weight: "bold")[Wrap-Around: Early Batch Seal])

    let labels = ("252", "253", "254", "255", "0", "1", "2", "3")
    for (i, lbl) in labels.enumerate() {
      let x = i * (sw + 0.08)
      let fill_c = if i == 2 or i == 3 {
        col-inflt.lighten(30%)
      } else if i >= 4 and i < 7 {
        col-fill
      } else {
        col-idle
      }
      rect((x, 1.5), (x + sw, 1.5 + sh), fill: fill_c, stroke: 0.3pt, radius: 2pt)
      content((x + sw/2, 1.5 + sh/2), text(fill: if fill_c == col-idle { luma(160) } else { white }, size: 5.5pt)[#lbl])
    }

    let wrap_x = 4 * (sw + 0.08) - 0.04
    line((wrap_x, 1.2), (wrap_x, 2.35), stroke: (paint: col-alloc, thickness: 1.2pt, dash: "dashed"))
    content((wrap_x, 1.0), text(size: 6pt, fill: col-alloc, weight: "bold")[ring wraps])

    line((2 * (sw + 0.08), 2.2), (3 * (sw + 0.08) + sw, 2.2), stroke: 0.5pt)
    content((2.5 * (sw + 0.08) + sw/2, 2.5), text(size: 6pt, fill: col-inflt)[batch $N$ sealed\ (count=2)])

    line((4 * (sw + 0.08), 2.2), (6 * (sw + 0.08) + sw, 2.2), stroke: 0.5pt)
    content((5 * (sw + 0.08) + sw/2, 2.5), text(size: 6pt, fill: col-fill)[batch $N+1$\ starts at 0])
  }),
  caption: [When the next descriptor would cross the ring boundary, the current batch is sealed early and a new batch starts at index 0.]
) <wrap-fig>

// ══════════════════════════════════════════════════════════════
= Doorbell Cost Comparison

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *

    let tw = 13.0
    content((tw/2, 5.5), text(size: 9pt, weight: "bold")[Doorbell Cost: $N = 8$ ops, max batch size $B = 4$])

    // ── Immediate ──
    content((-2.0, 4.3), text(size: 8pt, weight: "bold")[Immediate])
    for i in range(8) {
      let x = i * 1.55
      rect((x, 3.8), (x + 0.7, 4.3), fill: col-desc, radius: 2pt)
      content((x + 0.35, 4.05), text(fill: white, size: 5.5pt)[d#str(i)])
      rect((x + 0.75, 3.8), (x + 1.05, 4.3), fill: col-hw, radius: 2pt)
      content((x + 0.9, 4.05), text(fill: white, size: 5pt)[!])
    }
    content((tw + 0.3, 4.05), text(size: 7pt, fill: col-alloc, weight: "bold")[8])

    // ── Double-Buffered ──
    content((-2.0, 2.6), text(size: 8pt, weight: "bold")[Double-\ Buffered])
    for i in range(4) {
      let x = i * 0.75
      rect((x, 2.1), (x + 0.7, 2.6), fill: col-desc, radius: 2pt)
      content((x + 0.35, 2.35), text(fill: white, size: 5.5pt)[d#str(i)])
    }
    rect((3.05, 2.1), (3.35, 2.6), fill: col-hw, radius: 2pt)
    content((3.2, 2.35), text(fill: white, size: 5pt)[!])
    for i in range(2) {
      let x = 3.6 + i * 0.75
      rect((x, 2.1), (x + 0.7, 2.6), fill: col-desc, radius: 2pt)
      content((x + 0.35, 2.35), text(fill: white, size: 5.5pt)[d#str(i+4)])
    }
    rect((5.15, 2.1), (5.45, 2.6), fill: col-hw, radius: 2pt)
    content((5.3, 2.35), text(fill: white, size: 5pt)[!])
    for i in range(2) {
      let x = 5.7 + i * 0.75
      rect((x, 2.1), (x + 0.7, 2.6), fill: col-desc, radius: 2pt)
      content((x + 0.35, 2.35), text(fill: white, size: 5.5pt)[d#str(i+6)])
    }
    rect((7.25, 2.1), (7.55, 2.6), fill: col-hw, radius: 2pt)
    content((7.4, 2.35), text(fill: white, size: 5pt)[!])
    content((tw + 0.3, 2.35), text(size: 7pt, fill: col-hw, weight: "bold")[3])
    content((8.5, 2.35), text(size: 6pt, fill: luma(130))[(partial batches from poll)])

    // ── Fixed-Ring ──
    content((-2.0, 0.9), text(size: 8pt, weight: "bold")[Fixed-\ Ring])
    for i in range(4) {
      let x = i * 0.75
      rect((x, 0.4), (x + 0.7, 0.9), fill: col-desc, radius: 2pt)
      content((x + 0.35, 0.65), text(fill: white, size: 5.5pt)[d#str(i)])
    }
    rect((3.05, 0.4), (3.35, 0.9), fill: col-hw, radius: 2pt)
    content((3.2, 0.65), text(fill: white, size: 5pt)[!])
    for i in range(4) {
      let x = 3.6 + i * 0.75
      rect((x, 0.4), (x + 0.7, 0.9), fill: col-desc, radius: 2pt)
      content((x + 0.35, 0.65), text(fill: white, size: 5.5pt)[d#str(i+4)])
    }
    rect((6.65, 0.4), (6.95, 0.9), fill: col-hw, radius: 2pt)
    content((6.8, 0.65), text(fill: white, size: 5pt)[!])
    content((tw + 0.3, 0.65), text(size: 7pt, fill: col-done, weight: "bold")[2])
    content((8.0, 0.65), text(size: 6pt, fill: luma(130))[(same as ring-buffer)])

    // ── Ring-Buffer ──
    content((-2.0, -0.8), text(size: 8pt, weight: "bold")[Ring-\ Buffer])
    for i in range(4) {
      let x = i * 0.75
      rect((x, -1.3), (x + 0.7, -0.8), fill: col-desc, radius: 2pt)
      content((x + 0.35, -1.05), text(fill: white, size: 5.5pt)[d#str(i)])
    }
    rect((3.05, -1.3), (3.35, -0.8), fill: col-hw, radius: 2pt)
    content((3.2, -1.05), text(fill: white, size: 5pt)[!])
    for i in range(4) {
      let x = 3.6 + i * 0.75
      rect((x, -1.3), (x + 0.7, -0.8), fill: col-desc, radius: 2pt)
      content((x + 0.35, -1.05), text(fill: white, size: 5.5pt)[d#str(i+4)])
    }
    rect((6.65, -1.3), (6.95, -0.8), fill: col-hw, radius: 2pt)
    content((6.8, -1.05), text(fill: white, size: 5pt)[!])
    content((tw + 0.3, -1.05), text(size: 7pt, fill: col-done, weight: "bold")[2])
    content((8.0, -1.05), text(size: 6pt, fill: luma(130))[(always full batches)])
  }),
  caption: [Doorbell count comparison for 8 operations. Immediate: 8. Double-buffered: 3 (partial batches from `poll()`). Fixed-ring and ring-buffer: 2 (always full batches of $B$).]
) <doorbell-cost-fig>

// ══════════════════════════════════════════════════════════════
= Benchmark Results

All benchmarks: sliding window, inline polling, NoLock queue, concurrency 16, 8-byte messages, 75k ops/iter, 3 iterations. Platform: Intel 4th Gen Xeon Scalable, DSA configured via `accel-config`.

== Message Rate

#figure(
  table(
    columns: 4,
    align: (left, right, right, right),
    stroke: 0.5pt,
    inset: 6pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Operation*][*Double-Buffered (MPPS)*][*Ring-Buffer (MPPS)*][*Speedup*],
    [data\_move],     [2.60], [4.29], [*1.65x*],
    [mem\_fill],      [4.13], [8.41], [*2.04x*],
    [compare],        [4.61], [8.09], [*1.76x*],
    [compare\_value], [4.69], [8.08], [*1.72x*],
    [dualcast],       [4.62], [7.99], [*1.73x*],
    [crc\_gen],       [4.68], [8.13], [*1.74x*],
    [copy\_crc],      [2.36], [2.96], [*1.25x*],
    [cache\_flush],   [2.41], [2.94], [*1.22x*],
  ),
  caption: [Message rate comparison at concurrency 16, 8-byte messages.]
) <mpps-table>

Largest gains on lightweight ops (mem\_fill, compare) where software overhead dominates over hardware execution time.

== Latency

#figure(
  table(
    columns: 5,
    align: (left, right, right, right, right),
    stroke: 0.5pt,
    inset: 6pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Operation*][*Double-Buffered avg (ns)*][*Ring-Buffer avg (ns)*][*Double-Buffered p99 (ns)*][*Ring-Buffer p99 (ns)*],
    [data\_move],     [4860], [3247], [6547], [4955],
    [mem\_fill],      [3128], [1678], [3969], [2155],
    [compare],        [2806], [1763], [3615], [2536],
    [compare\_value], [2749], [1765], [3545], [2527],
    [dualcast],       [2806], [1790], [3612], [2573],
    [crc\_gen],       [2759], [1750], [3553], [2480],
    [copy\_crc],      [6129], [5047], [6964], [7804],
    [cache\_flush],   [5973], [5081], [6745], [7083],
  ),
  caption: [Latency comparison at concurrency 16, 8-byte messages.]
) <latency-table>

Average latency drops 30--46% for lightweight ops. The p99 for copy\_crc and cache\_flush shows slight regression, likely from occasional wrap-around batch splits.

// ══════════════════════════════════════════════════════════════
= Performance Analysis

#keypoint[
  Three factors explain the 1.2--2.0x speedup:
  + *No submission blocking* --- 16 batch slots vs. 2. Back-pressure is extremely rare.
  + *Better batch utilization* --- auto-submit at `max_batch_size_` yields consistently full batches, whereas double-buffered submits partials on every `poll()`.
  + *No wasted descriptor space* --- tightly packed ring vs. $2 times 32$ fixed slots.

  The fixed-ring ablation (@fixed-ring) shares factors 1 and 2 but not 3: it also uses 16 in-flight batches with auto-submit, yet wastes descriptor space like double-buffered. Its performance closely matches ring-buffer, confirming that in-flight depth --- not memory packing --- is the dominant factor.
]

// ══════════════════════════════════════════════════════════════
= Strategy Comparison

#figure(
  table(
    columns: 5,
    align: (left, center, center, center, center),
    stroke: 0.5pt,
    inset: 6pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Property*][*Immediate*][*Double-Buffered*][*Fixed-Ring*][*Ring-Buffer*],
    [Doorbells per $N$ ops], [$N$], [$ceil(N \/ B)$], [$ceil(N \/ B)$], [$ceil(N \/ B)$],
    [Staging buffers], [None], [2 fixed arrays], [16 fixed arrays], [1 shared ring],
    [Max in-flight batches], [N/A], [2], [16], [16],
    [Submit blocks?], [No], [Yes (on prev batch)], [No], [No],
    [Descriptor waste], [None], [Up to $2 times 32$ slots], [Up to $16 times 32$ slots], [Only at wrap],
    [Memory footprint], [0], [$approx$4 KB], [$approx$32 KB], [$approx$17 KB],
    [Wrap-around handling], [N/A], [N/A], [N/A], [Early batch seal],
  ),
  caption: [Comparison of submission strategies. $B$ = max batch size (typically 32).]
) <comparison-table>

// ══════════════════════════════════════════════════════════════
= Sizing

#figure(
  table(
    columns: 3,
    align: (left, right, left),
    stroke: 0.5pt,
    inset: 6pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Parameter*][*Value*][*Rationale*],
    [`kDescRingSize`], [256], [256 #sym.times 64 B = 16 KB, fits L1 cache],
    [`kMaxBatches`], [16], [16 concurrent in-flight batches],
    [`max_batch_size_`], [min(hw, 32)], [Per-batch limit from hardware],
    [Total memory], [$approx$17 KB], [Descriptor ring + batch metadata],
  ),
  caption: [Ring-buffer sizing parameters.]
)

#figure(
  table(
    columns: 3,
    align: (left, right, left),
    stroke: 0.5pt,
    inset: 6pt,
    fill: (_, y) => if y == 0 { rgb("#f8fafc") },
    table.header[*Parameter*][*Value*][*Rationale*],
    [`kMaxBatches`], [16], [Same depth as ring-buffer],
    [`kBatchCapacity`], [32], [Per-entry descriptor array size],
    [`max_batch_size_`], [min(hw, 32)], [Per-batch limit from hardware],
    [Total memory], [$approx$34 KB], [16 #sym.times (32 #sym.times 64 B + 32 B comp)],
  ),
  caption: [Fixed-ring sizing parameters. Higher memory footprint than ring-buffer due to per-entry fixed arrays.]
)

// ══════════════════════════════════════════════════════════════
= Implementation

== Type Erasure and Composition

All four strategies satisfy the `DsaFacade` interface via Microsoft's proxy library. `DsaProxy` wraps any concrete DSA type, enabling runtime strategy selection. `DsaBatchBase`, `DsaFixedRingBatchBase`, and `DsaRingBatchBase` all use composition (contain a `DsaBase` inner instance) rather than inheritance.

== Template Parameterization

All classes are templated on queue type (`SingleThreadTaskQueue`, `MutexTaskQueue`, `SpinlockTaskQueue`, etc.) with explicit instantiations in separate `.cpp` files.

// ══════════════════════════════════════════════════════════════
= Future Work

- *Higher concurrency benchmarks* at 32--64 to validate ring depth sufficiency
- *Shared WQ contention* measurements -- batching reduces `ENQCMD` retries on multi-tenant systems
- *Adaptive batch sizing* based on submission rate and hardware queue depth
- *Explicit batch sender* API: `dsa_batch(sender1, sender2, ...)` for application-level ordering
