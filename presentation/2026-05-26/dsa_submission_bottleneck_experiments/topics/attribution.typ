#import "../support.typ": *

#let rule() = [
  = Attribution rule

  #compact-table(
    columns: (0.31fr, 0.41fr, 0.28fr),
    table.header([First bend], [Credit domain], [Next probe]),
    [#chip([one-submit vs K], color: c-accent)],
    [submit / WQ accept],
    [knee depth vs WQ size],
    [#chip([trace marker], color: rgb("#16a34a"))],
    [early completions visible by wall],
    [separate progress from admission],
    [#chip([late marker], color: rgb("#ea580c"))],
    [posting / completion delay],
    [IIO / WQ credits],
    [#chip([reuse variants], color: rgb("#ea580c"))],
    [poll / reset / cacheline],
    [reuse redesign],
    [#chip([no-count push loses], color: rgb("#dc2626"))],
    [missing completions],
    [restore in-flight tracking],
  )

  #v(0.85em)

  #soft-box(fill: white, stroke: c-title, inset: (x: 16pt, y: 10pt))[
    #text(size: 17pt, weight: "bold", fill: c-title)[Find first bend → name the credit domain.]
  ]
]

#let measured_answer() = [
  = Measured answer: WQ admission is the first bottleneck

  #callout(fill: c-red, stroke: rgb("#dc2626"), inset: (x: 16pt, y: 9pt))[
    #grid(
      columns: (0.36fr, 0.32fr, 0.32fr),
      gutter: 12pt,
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#991b1b"))[practical bound]
        #v(0.1em)
        #text(size: 22pt, weight: "bold", fill: c-title)[< 116 outstanding]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#991b1b"))[nominal WQ]
        #v(0.1em)
        #text(size: 22pt, weight: "bold", fill: c-title)[128 entries]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#991b1b"))[not explained by]
        #v(0.1em)
        #text(size: 18pt, weight: "bold", fill: c-title)[64B / reuse / loss]
      ],
    )
  ]

  #v(0.45em)

  #compact-table(
    columns: (0.26fr, 0.43fr, 0.31fr),
    table.header([Probe], [Observed result], [Implication]),
    [#chip([one-submit vs K], color: c-accent)],
    [`K≈116` plateau at `224--226` ticks],
    [admission / credits bend first],
    [#chip([marker trace], color: rgb("#16a34a"))],
    [comp[1..4] visible at first poll for offsets `112` and `115`],
    [DSA progressed before/at the wall],
    [#chip([traffic ladder], color: rgb("#ea580c"))],
    [same submit knee for submit-only, NOOP, 64B, 4KiB],
    [payload does not move first bend],
    [#chip([reuse policy], color: rgb("#ea580c"))],
    [best policies stay around `8.5 Mops/s`],
    [completion loop not the first cause],
    [#chip([blind push], color: rgb("#16a34a"))],
    [`1024/1024` complete; missing/errors `0/0`],
    [not descriptor loss],
  )

  #v(0.45em)

  #soft-box(fill: white, stroke: c-title, inset: (x: 16pt, y: 9pt))[
    #text(size: 15pt, weight: "bold", fill: c-title)[Interpretation: keep batch-size-1/no-batch logical concurrency below the observed admission threshold, not the nominal WQ size.]
  ]

  #v(0.25em)
  #source-line[Sources: docs/report/benchmarking/019.submit_occupancy_one_extra_2026-05-26.md; docs/report/benchmarking/020.submission_bottleneck_experiments_2026-05-27.md; docs/report/benchmarking/021.submit_marker_trace_2026-05-27.md.]
]
