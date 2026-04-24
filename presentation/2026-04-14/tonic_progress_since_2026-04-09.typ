// Progress report slide since 2026-04-09
// Sources:
// - docs/plan/2026-04-14/01.progress_report_slide_since_2026-04-09.in_progress.md
// - docs/report/benchmarking/012.tonic_characterization_refinement_results.md
// - docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md
// - docs/plan/2026-04-13/05.pre_advisor_tonic_characterization_priorities.in_progress.md
// - docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md
// - docs/report/literature/010.expanded_paper_folder_index_2026-04-13.md
// - .agents/state/threads/thr-20260414-tonic-literature-deck.md

#import "../template.typ": callout, card, deck, palette, panel

#show: deck.with(
  margin: (x: 38pt, y: 24pt),
  size: 13.5pt,
  leading: 0.76em,
  spacing: 0.5em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-row = palette.row

= Progress since Thu 2026-04-09

#align(center + horizon)[
  #text(size: 13.5pt)[Tonic characterization status update · Hongtao Zhang · April 14, 2026]
]

#v(0.35em)

#callout(fill: c-blue, stroke: c-accent)[
  Since last Thursday, the thread added matched characterization controls, bucket-level instrumentation, and a more explicit next-pass measurement plan.
]

#v(0.28em)

#set align(left)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 11pt,
  [#panel(fill: c-row, inset: (x: 12pt, y: 10pt))[
    #text(weight: "bold", fill: c-title)[Measurement lane]
    #v(0.2em)
    + Matched unary refinement now includes instrumentation on/off plus pooled and copy-minimized controls.
    + Internal timers isolate encode/decode, reserve, body accumulation, and compression buckets for a paired CPU-plus-decomposition next pass.
  ]],
  [#panel(fill: c-green, inset: (x: 12pt, y: 10pt))[
    #text(weight: "bold", fill: c-title)[Observed regimes]
    #v(0.2em)
    + `4 KiB` with the `pooled` control ran at #text(weight: "bold")[`2.51x`] the default instrumentation-on baseline.
    + `1 MiB` uncompressed remains movement-heavy at #text(weight: "bold")[`0.5` IPC] and #text(weight: "bold")[`63.7%` memory-bound].
    + Compression stays conditional: `64 KiB` structured drops to `0.585x`; random payloads collapse to `0.112x`.
  ]],
  [#panel(fill: c-blue, inset: (x: 12pt, y: 10pt))[
    #text(weight: "bold", fill: c-title)[Framing and deck support]
    #v(0.2em)
    + FleetBench-inspired intake adds realistic protobuf shape, request/response asymmetry, closed-loop load, and delay-distribution axes.
    + The literature workflow expanded to #text(weight: "bold")[`21` paper folders] beyond the original six-paper seed set.
    + The literature deck was rebuilt as a paper-first self-study artifact aligned with the repo notes.
  ]],
)

== Immediate next step before stronger claims

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Next pass: reduce timer distortion with lower-overhead split client/server accounting, then rerun a small paired matrix of instrumentation-off CPU runs and matched decomposition runs.
]

#v(0.35em)

#grid(
  columns: (1.05fr, 0.95fr),
  gutter: 14pt,
  [#card(
    [What the follow-up pass must do],
    [
      1. Separate throughput evidence from attribution evidence. 2. Keep the matrix small but realistic: message shape, asymmetry, compression, and load pressure. 3. Emit a one-page pre-advisor note that states which bucket dominates which regime and why.
    ],
    fill: c-row,
    body-size: 11pt,
  )],
  [#card(
    [What remains provisional right now],
    [Endpoint-specific claims and stronger offload-readiness statements. The current timers are diagnostic outside the tiny single-thread point, so stronger claims wait for the lower-overhead split pass. Grounding: benchmarking `012`/`013`, literature `009`/`010`, and plan `05` from 2026-04-13.],
    fill: c-green,
    body-size: 10.8pt,
  )],
)
