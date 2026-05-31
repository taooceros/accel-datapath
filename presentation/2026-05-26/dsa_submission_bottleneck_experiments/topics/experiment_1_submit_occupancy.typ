#import "../support.typ": *

#let setup() = [
  = Experiment 1: submit gets slow when WQ is full?

  #grid(
    columns: (0.55fr, 0.45fr),
    gutter: 18pt,
    [
      #section-label[Setup]
      #v(0.2em)
      #seqbox[
        #chronos.diagram({
          import chronos: *
          _par("CPU", display-name: "CPU")
          _par("WQ", display-name: "WQ / admission")
          _par("DSA", display-name: "DSA")
          _par("C", display-name: "Completion")

          _seq("CPU", "WQ", comment: "prefill K")
          _seq("CPU", "WQ", comment: "MEASURE: t0 extra submit", enable-dst: true, lifeline-style: (
            fill: rgb("#2563eb"),
          ))
          _seq("WQ", "CPU", comment: "t1 return / stall", dashed: true, disable-src: true)
          _seq("WQ", "DSA", comment: "old slot drains", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
          _seq("DSA", "C", comment: "tc first old completion", enable-dst: true, disable-src: true, lifeline-style: (
            fill: rgb("#16a34a"),
          ))
          _seq("CPU", "C", comment: "observe tc", dashed: true, disable-dst: true)
          _seq("CPU", "WQ", comment: "drain / reset", dashed: true)
        })
      ]

      #v(0.25em)
      #compact-table(
        columns: (0.35fr, 0.65fr),
        [#chip([x], color: c-title)],
        [`K = 0,32,64,96,112,120,124,127,128`],
        [#chip([y], color: c-accent)],
        [extra-submit cycles],
        [#chip([hold fixed], color: rgb("#16a34a"))],
        [op, NUMA, WQ, core],
      )
    ],
    [
      #section-label[Readout]
      #v(0.25em)
      #metric-pill[submit latency vs. occupancy]

      #v(0.25em)
      #fill-key(
        [#fill-item(rgb("#2563eb"))[extra-submit timer]],
        [#fill-item(rgb("#16a34a"))[first old-slot completion wait]],
      )

      #v(0.45em)

      #compact-table(
        columns: (0.42fr, 0.58fr),
        [#chip([flat], color: rgb("#16a34a"))],
        [submit credits remain],
        [#chip([knee near 128], color: rgb("#ea580c"))],
        [WQ/device credits],
        [#chip([early knee], color: rgb("#dc2626"))],
        [store / posting / IIO / portal],
        [#chip([full-WQ tail], color: c-title)],
        [wait for device progress],
      )
    ],
  )

  #v(0.35em)
  #callout(fill: c-blue, stroke: c-accent, inset: (x: 14pt, y: 7pt))[
    Fill WQ more → see when submit gets slow.
  ]
]

#let why_run_it() = [
  = Experiment 1: why run it

  #grid(
    columns: (0.38fr, 0.62fr),
    gutter: 18pt,
    [
      #section-label[Why run it]
      #v(0.25em)
      #compact-table(
        columns: (0.33fr, 0.67fr),
        [#chip([why], color: c-accent)],
        [fill the WQ, then time one extra submit],
        [#chip([learn], color: rgb("#16a34a"))],
        [when the extra submit gets slow],
        [#chip([record], color: c-title)],
        [`K`, submit cycles, first old done time],
      )

      #v(0.55em)
      #soft-box(fill: c-blue)[
        If submit stays fast until the queue is nearly full, the 20-cycle number is still real. If it gets slow early, the host/WQ path is already running out of credits.
      ]
    ],
    [
      #section-label[Simple Rust sketch]
      #v(0.2em)
      #zebraw(
        numbering: false,
        inset: (x: 4pt, y: 2pt),
        comment-font-args: (size: 6.5pt),
        ```rust
        for &k in &[0, 32, 64, 96, 112, 120, 124, 127, 128] {
            bench.reset_slots();
            bench.prefill(k, Op::Noop);

            let t0 = ticks();
            bench.submit_one(Op::Noop);
            let t1 = ticks();

            let first_done = bench.poll_first_old();
            bench.drain_all();

            record("credit-knee", k, t1 - t0, first_done);
        }
        ```,
      )
      #v(0.25em)
      #code-note[These are simple calls; the real runner still sets descriptors, fences, and checks completions.]
    ],
  )
]

#let result() = [
  = Experiment 1 result: admission bends before WQ size

  #callout(fill: c-red, stroke: rgb("#dc2626"), inset: (x: 16pt, y: 9pt))[
    #grid(
      columns: (0.34fr, 0.33fr, 0.33fr),
      gutter: 12pt,
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#991b1b"))[first plateau]
        #v(0.1em)
        #text(size: 23pt, weight: "bold", fill: c-title)[K≈116]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#991b1b"))[extra submit]
        #v(0.1em)
        #text(size: 18pt, weight: "bold", fill: c-title)[224--226 ticks]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#991b1b"))[nominal WQ]
        #v(0.1em)
        #text(size: 23pt, weight: "bold", fill: c-title)[128]
      ],
    )
  ]

  #v(0.35em)

  #grid(
    columns: (0.62fr, 0.38fr),
    gutter: 16pt,
    [
      #lq-diagram(
        title: [Extra-submit latency vs occupancy],
        xlabel: [`K` prefill],
        ylabel: [median TSC],
        xlim: (112, 124),
        ylim: (0, 250),
        xaxis: (ticks: (112, 114, 116, 118, 120, 122, 124), subticks: none),
        yaxis: (ticks: (0, 50, 100, 150, 200, 250), subticks: none),
        lq-plot(
          (112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124),
          (24, 24, 32, 86, 226, 224, 224, 224, 224, 224, 224, 224, 224),
          stroke: 1.5pt + c-accent,
          mark: "o",
          label: [NOOP],
        ),
        lq-plot(
          (112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124),
          (32, 28, 24, 194, 224, 224, 224, 224, 224, 222, 222, 226, 228),
          stroke: 1.5pt + rgb("#16a34a"),
          mark: "s",
          label: [64B],
        ),
      )
    ],
    [
      #section-label[Attribution]
      #v(0.25em)
      #metric-pill(color: rgb("#dc2626"))[one-submit vs K bends early]
      #v(0.35em)
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        Cheap through `K≤114`: `24--32` TSC ticks.
      ]
      #v(0.3em)
      #soft-box(fill: c-orange, stroke: rgb("#ea580c"), inset: (x: 12pt, y: 7pt))[
        Transition at `K=115`: NOOP `86`, 64B `194` ticks.
      ]
      #v(0.3em)
      #soft-box(fill: c-red, stroke: rgb("#dc2626"), inset: (x: 12pt, y: 7pt))[
        Plateau begins at `K=116`; 64B follows NOOP, so payload is not the first bend.
      ]
    ],
  )

  #v(0.25em)
  #source-line[Source: docs/report/benchmarking/019.submit_occupancy_one_extra_2026-05-26.md]
]
