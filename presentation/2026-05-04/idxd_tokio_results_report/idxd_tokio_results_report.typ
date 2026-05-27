// Characteristics report: Tokio integration for IDXD/DSA accelerators
// Reader: advisor / project collaborator.
// Claim boundary: matched NUMA0 characterization only; not a final overhead/speedup claim.
// Sources:
// - docs/plan/2026-05-04/09.idxd-tokio-results-report-slide.plan.md
// - docs/report/benchmarking/013.tokio_single_numa_shared_wq.md
// - docs/report/benchmarking/shared_thread_sweep_numa0/tokio_wq0_1_numa0_threads{1,2,4,8,16,32,64}.jsonl
// - docs/report/benchmarking/shared_thread_sweep_numa0/hw_eval_wq0_1_numa0_threads{1,2,4,8,16,32,64}.json

#import "../../template.typ": callout, deck, palette, panel
#import "@preview/lilaq:0.6.0" as lq

#show: deck.with(
  margin: (x: 38pt, y: 28pt),
  size: 12.3pt,
  leading: 0.82em,
  spacing: 0.50em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-row = palette.row
#let c-tokio = rgb("#2563eb")
#let c-raw = rgb("#dc2626")

#let threads = (1, 2, 4, 8, 16, 32, 64)
#let thread-ticks = ((1, [1]), (2, [2]), (4, [4]), (8, [8]), (16, [16]), (32, [32]), (64, [64]))
#let raw-threads = threads

#let scenarios = (
  (
    title: [128 B: NUMA0 per-thread scaling at c=128],
    note: [NUMA-constrained Tokio keeps scaling through 32 threads; the refreshed raw sweep now covers the same thread counts.],
    bytes: 128,
    tokio: (3.616, 5.983, 12.342, 20.968, 35.738, 46.384, 33.443),
    raw: (5.049, 8.367, 15.820, 29.229, 44.259, 39.415, 32.864),
    ylim: (0, 50),
    bw-tick: 1,
  ),
  (
    title: [256 B: NUMA0 per-thread scaling at c=128],
    note: [Raw is generally higher for 256 B; Tokio's best matched point is around 4 threads.],
    bytes: 256,
    tokio: (1.965, 4.516, 11.421, 18.280, 22.954, 25.148, 23.988),
    raw: (5.057, 8.499, 10.579, 31.001, 41.640, 39.370, 34.554),
    ylim: (0, 45),
    bw-tick: 2,
  ),
  (
    title: [1 KiB: NUMA0 per-thread scaling at c=128],
    note: [Tokio is close at 16--32 threads; raw is steadier at high thread counts.],
    bytes: 1024,
    tokio: (2.036, 5.740, 9.954, 19.326, 24.795, 21.776, 15.637),
    raw: (4.999, 6.109, 13.026, 22.726, 24.252, 21.618, 21.960),
    ylim: (0, 30),
    bw-tick: 5,
  ),
  (
    title: [4 KiB: NUMA0 per-thread scaling at c=128],
    note: [Both paths reach high byte throughput; Tokio is closest at 32--64 threads.],
    bytes: 4096,
    tokio: (2.464, 4.627, 6.380, 5.247, 5.203, 6.608, 6.994),
    raw: (4.576, 5.273, 6.883, 7.030, 6.951, 6.770, 6.665),
    ylim: (0, 8),
    bw-tick: 10,
  ),
)

#let source-line(body) = text(size: 7.2pt, fill: luma(115))[#body]

#let badge(title, value, body, fill: c-row, accent: c-accent) = block(
  width: 100%,
  radius: 8pt,
  inset: (x: 12pt, y: 10pt),
  fill: fill,
  stroke: 0.6pt + palette.border,
)[
  #text(size: 9.4pt, fill: luma(85))[#title]
  #v(0.18em)
  #text(size: 19pt, weight: "bold", fill: accent)[#value]
  #v(0.15em)
  #text(size: 9.5pt, fill: luma(65))[#body]
]

#let contract-card(title, body) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 10pt, y: 8pt),
  fill: c-row,
  stroke: 0.5pt + palette.border,
)[
  #text(weight: "bold", fill: c-title)[#title]
  #v(0.18em)
  #text(size: 9.5pt, fill: luma(65))[#body]
]

#let thread-plot(bytes, tokio, raw, ylim, bw-tick, width: 610pt, height: 250pt) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 8pt, y: 7pt),
  fill: white,
  stroke: 0.5pt + palette.border,
)[
  #align(center)[
    #lq.diagram(
      width: width,
      height: height,
      xscale: "log",
      xlim: (0.8, 80),
      ylim: ylim,
      xlabel: [submitter threads],
      ylabel: [Mops/s],
      xaxis: (ticks: thread-ticks, subticks: none),
      yaxis: (exponent: 0),
      grid: auto,
      legend: (position: top + left, fill: white, stroke: 0.35pt + luma(190), pad: 3pt),
      lq.plot(threads, tokio, label: [Tokio NUMA0], mark: "o", mark-size: 4pt, color: c-tokio),
      lq.plot(raw-threads, raw, label: [raw NUMA0], mark: "s", mark-size: 4pt, color: c-raw),
      lq.yaxis(
        position: right,
        label: [GB/s],
        functions: (x => x * bytes / 1000, y => y * 1000 / bytes),
        tick-distance: bw-tick,
      ),
    )
  ]
]

= Tokio + IDXD: NUMA0 per-message, per-thread characteristics

#callout(fill: c-blue, stroke: c-accent, inset: (x: 12pt, y: 8pt))[
  Use NUMA-constrained data. For each message size, sweep submitter thread count and compare Tokio to raw hardware at matched NUMA0 points.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr, 1fr, 1fr),
  gutter: 10pt,
  [#contract-card([Device/WQ], [`/dev/dsa/wq0.1` shared WQ])],
  [#contract-card([Operation], [direct/no-batch DSA memmove; `batch_n=1`])],
  [#contract-card([Placement], [NUMA node 0 CPU + memory])],
  [#contract-card([Threads], [`1, 2, 4, 8, 16, 32, 64`])],
)

#v(0.55em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#badge([128 B, 32T], [1.18x], [46.4 / 39.4 Mops/s], fill: c-green, accent: rgb("#16a34a"))],
  [#badge([256 B, 4T], [1.08x], [11.4 / 10.6 Mops/s], fill: c-green, accent: rgb("#16a34a"))],
  [#badge([128 B, 16T], [0.81x], [35.7 / 44.3 Mops/s], fill: c-orange, accent: rgb("#c2410c"))],
)

#v(0.55em)

#panel(fill: c-row, inset: (x: 13pt, y: 10pt))[
  #text(weight: "bold", fill: c-title)[Plot rule]
  #v(0.25em)
  Generated from the `scenarios` data array. Each slide fixes one message size and `c=128`, then sweeps all matched NUMA0 Tokio and raw thread counts.
]

#for (i, scenario) in scenarios.enumerate() [
  #pagebreak()

  = #scenario.title

  #callout(fill: c-blue, stroke: c-accent, inset: (x: 12pt, y: 8pt))[#scenario.note]

  #v(0.3em)

  #thread-plot(
    scenario.bytes,
    scenario.tokio,
    scenario.raw,
    scenario.ylim,
    scenario.bw-tick,
  )

  #if i == scenarios.len() - 1 [
    #v(0.25em)
    #source-line[Sources: NUMA0 Tokio JSONL files and refreshed NUMA0 raw hw-eval JSON files under `shared_thread_sweep_numa0/`. Left axis: Mops/s. Right axis: GB/s.]
  ]
]
