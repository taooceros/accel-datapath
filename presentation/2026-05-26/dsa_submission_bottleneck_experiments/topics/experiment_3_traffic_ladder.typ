#import "../support.typ": *

#let setup() = [
  = Experiment 3: add one traffic type at a time

  #grid(
    columns: (0.56fr, 0.44fr),
    gutter: 18pt,
    [
      #section-label[Traffic ladder]
      #v(0.15em)
      #seqbox[
        #chronos.diagram({
          import chronos: *
          _par("CPU", display-name: "CPU")
          _par("WQ", display-name: "WQ")
          _par("DSA", display-name: "DSA")
          _par("MEM", display-name: "Memory")
          _par("C", display-name: "Completion")

          _seq("CPU", "WQ", comment: "A MEASURE: submit-only", enable-dst: true, lifeline-style: (fill: rgb("#2563eb")))
          _seq("DSA", "C", comment: "B MEASURE: + completion", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
          _seq("DSA", "MEM", comment: "C MEASURE: + 64B payload", enable-dst: true, lifeline-style: (
            fill: rgb("#ea580c"),
          ))
          _seq("DSA", "MEM", comment: "D MEASURE: + 4KiB payload", enable-dst: true, lifeline-style: (
            fill: rgb("#dc2626"),
          ))
        })
      ]

      #v(0.25em)
      #compact-table(
        columns: (0.22fr, 0.78fr),
        [#chip([A], color: c-accent)],
        [submit-only / no completion],
        [#chip([B], color: rgb("#16a34a"))],
        [completion-bearing NOOP],
        [#chip([C], color: rgb("#ea580c"))],
        [64B memmove],
        [#chip([D], color: rgb("#dc2626"))],
        [4KiB memmove],
      )
    ],
    [
      #section-label[Readout]
      #v(0.25em)
      #metric-pill[same curves; one class added]

      #v(0.25em)
      #fill-key(
        [#fill-item(rgb("#2563eb"))[submit-only path]],
        [#fill-item(rgb("#16a34a"))[completion path]],
        [#fill-item(rgb("#ea580c"))[64B payload path]],
        [#fill-item(rgb("#dc2626"))[4KiB payload path]],
      )

      #v(0.45em)

      #compact-table(
        columns: (0.36fr, 0.64fr),
        [#chip([A bends], color: c-accent)],
        [submit alone],
        [#chip([B adds], color: rgb("#16a34a"))],
        [completion write / seen by CPU],
        [#chip([C/D add], color: rgb("#ea580c"))],
        [payload DMA domain],
        [#chip([D unique], color: rgb("#dc2626"))],
        [fabric / bandwidth / locality],
      )
    ],
  )
]

#let why_run_it() = [
  = Experiment 3: why run it

  #grid(
    columns: (0.38fr, 0.62fr),
    gutter: 18pt,
    [
      #section-label[Why run it]
      #v(0.25em)
      #compact-table(
        columns: (0.33fr, 0.67fr),
        [#chip([why], color: c-accent)],
        [change one thing per run],
        [#chip([learn], color: rgb("#16a34a"))],
        [which added work makes it slow],
        [#chip([compare], color: c-title)],
        [same WQ, core, window, NUMA node],
      )

      #v(0.55em)
      #soft-box(fill: c-orange)[
        This is the simple ladder: submit only, then completion writes, then payload reads/writes. The first new slowdown says what to check next.
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
        let classes = [
            Op::SubmitOnly,
            Op::Noop,
            Op::Memcpy { bytes: 64 },
            Op::Memcpy { bytes: 4096 },
        ];

        for op in classes {
            for window in [1, 8, 32, 64, 96, 128] {
                bench.reset_slots();

                let curve = bench.run_window(op, window, reps);
                record_curve("traffic", op, window, curve);
            }
        }
        ```,
      )
      #v(0.25em)
      #code-note[The runner changes only the operation class; all pacing and measurement code stays shared.]
    ],
  )
]

#let result() = [
  = Experiment 3 result: payload does not move the knee

  #callout(fill: c-blue, stroke: c-accent, inset: (x: 16pt, y: 9pt))[
    #grid(
      columns: (0.33fr, 0.34fr, 0.33fr),
      gutter: 12pt,
      [
        #text(size: 10pt, weight: "bold", fill: c-accent)[first jump]
        #v(0.1em)
        #text(size: 21pt, weight: "bold", fill: c-title)[window 120]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: c-accent)[large by]
        #v(0.1em)
        #text(size: 21pt, weight: "bold", fill: c-title)[window 128]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: c-accent)[64B vs NOOP]
        #v(0.1em)
        #text(size: 21pt, weight: "bold", fill: c-title)[same knee]
      ],
    )
  ]

  #v(0.35em)

  #grid(
    columns: (0.62fr, 0.38fr),
    gutter: 16pt,
    [
      #lq-diagram(
        title: [Submit phase by traffic class],
        xlabel: [window sweep],
        ylabel: [median submit TSC],
        yscale: "log",
        xlim: (0, 10),
        ylim: (20, 40000),
        xaxis: (
          ticks: (0, 2, 4, 6, 8, 10).zip(([1], [32], [96], [120], [128], [256])),
          subticks: none,
        ),
        yaxis: (ticks: (30, 100, 1000, 10000), subticks: none),
        lq-plot(
          range(11),
          (26, 36, 172, 342, 514, 614, 1532, 2486, 3418, 10974, 33644),
          stroke: 1.4pt + c-accent,
          mark: "o",
          label: [submit-only],
        ),
        lq-plot(
          range(11),
          (26, 38, 184, 374, 534, 662, 1532, 2472, 3418, 10958, 34138),
          stroke: 1.4pt + rgb("#16a34a"),
          mark: "s",
          label: [NOOP+done],
        ),
        lq-plot(
          range(11),
          (32, 46, 226, 440, 668, 782, 1568, 2508, 3436, 10978, 33654),
          stroke: 1.4pt + rgb("#ea580c"),
          mark: "^",
          label: [64B],
        ),
        lq-plot(
          range(11),
          (30, 42, 184, 358, 644, 670, 1528, 2456, 3400, 10914, 33416),
          stroke: 1.4pt + rgb("#dc2626"),
          mark: "x",
          label: [4KiB],
        ),
      )
    ],
    [
      #section-label[Throughput readout]
      #v(0.25em)
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        NOOP+completion holds around `9.1 Mops/s` at high windows.
      ]
      #v(0.3em)
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        64B memmove is close to NOOP at high window: `9.08 Mops/s` at 256.
      ]
      #v(0.3em)
      #soft-box(fill: c-orange, stroke: rgb("#ea580c"), inset: (x: 12pt, y: 7pt))[
        4KiB lowers throughput to `6.44 Mops/s`, but the submit knee stays put.
      ]
    ],
  )

  #v(0.25em)
  #source-line[Source: docs/report/benchmarking/020.submission_bottleneck_experiments_2026-05-27.md, Experiment 3.]
]
