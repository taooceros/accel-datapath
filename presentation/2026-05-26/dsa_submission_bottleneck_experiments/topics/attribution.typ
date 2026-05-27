#import "../support.typ": *

#let rule() = [
  = Attribution rule

  #compact-table(
    columns: (0.31fr, 0.41fr, 0.28fr),
    table.header([First bend], [Credit domain], [Next probe]),
    [#chip([one-submit vs K], color: c-accent)],
    [submit / WQ accept],
    [knee depth vs WQ size],
    [#chip([early marker], color: rgb("#16a34a"))],
    [hardware overlaps submit],
    [drain / reuse loop],
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
