#import "../support.typ": *

#let title() = [
  = Deciphering the DSA submission bottleneck

  #callout(fill: c-blue, stroke: c-accent, inset: (x: 16pt, y: 10pt))[
    #text(size: 19pt, weight: "bold", fill: c-title)[Diagnostic target]
    #v(0.2em)
    #text(size: 18pt)[missing cycles under full-WQ pressure]
  ]

  #v(0.55em)

  #compact-table(
    columns: (0.28fr, 0.25fr, 0.47fr),
    table.header([Signal], [Scale], [Meaning]),
    [Unloaded submit],
    [#chip([~20 cycles], color: c-accent)],
    [posted/admitted write; credits available],
    [Single request],
    [#chip([sub-µs visible completion], color: rgb("#16a34a"))],
    [`NOOP` ~379 ns; 64B ~575 ns],
    [Full-loop async],
    [#chip([~8--10 Mops/s], color: rgb("#ea580c"))],
    [closed-loop cost, not isolated submit],
  )

  #v(0.65em)

  #soft-box[
    #grid(
      columns: (1fr, auto, 1fr),
      gutter: 16pt,
      [#text(size: 17pt, weight: "bold", fill: c-accent)[cheap submit]],
      [#text(size: 20pt, weight: "bold")[≠]],
      [#text(size: 17pt, weight: "bold", fill: rgb("#dc2626"))[cheap closed loop]],
    )
  ]

  #v(0.35em)
  #source-line[Sources: submit-only study 2026-05-24; Understanding the Host Network; direct latency measurement.]
]

#let host_path_view() = [
  = Host path view: what to measure

  #grid(
    columns: (0.46fr, 0.54fr),
    gutter: 22pt,
    [
      #section-label[Where credits live]
      #v(0.35em)
      #compact-table(
        columns: (0.36fr, 0.64fr),
        [#chip([domain], color: c-title)],
        [processor / memory / peripheral path],
        [#chip([credits], color: c-accent)],
        [outstanding request budget],
        [#chip([unloaded latency], color: rgb("#16a34a"))],
        [cheap path when credits exist],
        [#chip([wait], color: rgb("#dc2626"))],
        [upstream work gets slowed],
      )
    ],
    [
      #section-label[DSA implication]
      #v(0.35em)
      #compact-table(
        columns: (0.42fr, 0.58fr),
        [`movdir64b` returns],
        [the WQ accepted the write],
        [not proved],
        [DSA execution],
        [not proved],
        [completion writeback],
        [not proved],
        [CPU can see completion],
        [full-loop cost],
        [first place that runs out of credits],
      )
    ],
  )

  #v(0.65em)

  #callout(fill: white, stroke: c-title, inset: (x: 14pt, y: 8pt))[
    #text(size: 16pt, weight: "bold")[Design rule:]
    #h(0.5em)
    vary one pressure source → find first bend
  ]
]

#let dsa_loop_timestamps() = [
  = The DSA loop: where timestamps attach

  #scale(x: 80%, y: 80%)[
    #chronos.diagram({
      import chronos: *
      _par("CPU", display-name: "CPU thread")
      _par("FAB", display-name: "Host fabric / WQ admission")
      _par("DSA", display-name: "DSA engine")
      _par("CMEM", display-name: "Completion memory")

      _seq("CPU", "FAB", comment: "MEASURE t0→t1: submit/admit", enable-dst: true, lifeline-style: (
        fill: rgb("#2563eb"),
      ))
      _seq("FAB", "CPU", comment: "t1: return / credit stall", dashed: true, disable-src: true)
      _seq("FAB", "DSA", comment: "descriptor visible", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
      _seq("DSA", "DSA", comment: "execute")
      _seq("DSA", "CMEM", comment: "completion write", enable-dst: true, disable-src: true, lifeline-style: (
        fill: rgb("#16a34a"),
      ))
      _seq("CPU", "CMEM", comment: "poll status")
      _seq("CMEM", "CPU", comment: "MEASURE t1→t2: visible", dashed: true, disable-src: true)
      _seq("CPU", "FAB", comment: "reset slot + resubmit", dashed: true)
    })
  ]

  #v(0.35em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 14pt,
    [
      #soft-box(fill: c-blue)[#phase([t0 → t1], [submit / waiting], c-accent)]
    ],
    [
      #soft-box(fill: c-green)[#phase([t1 → t2], [work done + visible], rgb("#16a34a"))]
    ],
  )

  #v(0.2em)
  #fill-key(
    [#fill-item(rgb("#2563eb"))[`t0→t1` submit interval]],
    [#fill-item(rgb("#16a34a"))[`t1→t2` done-and-visible interval]],
  )
]

#let dsa_traffic_classes() = [
  = DSA traffic classes

  #compact-table(
    columns: (0.25fr, 0.38fr, 0.37fr),
    table.header([DSA action], [Host path], [What it tests]),
    [`movdir64b` submit],
    [CPU → peripheral posted write],
    [`t0→t1` submit slowdown],
    [DSA source read],
    [Peripheral → memory read],
    [payload-read pressure],
    [DSA destination write],
    [Peripheral → memory write],
    [write-buffer / fabric pressure],
    [DSA completion write],
    [Peripheral → memory write, cacheline-scale],
    [completion becomes visible],
    [CPU poll/reset],
    [CPU memory read/write],
    [cacheline ownership; slot reuse],
  )

  #v(0.55em)

  #callout(fill: c-orange, stroke: rgb("#ea580c"), inset: (x: 14pt, y: 7pt))[
    #text(weight: "bold")[Exp. 3:]
    add classes
    #h(1.5em)
    #text(weight: "bold")[Exp. 5:]
    collide classes
  ]
]
