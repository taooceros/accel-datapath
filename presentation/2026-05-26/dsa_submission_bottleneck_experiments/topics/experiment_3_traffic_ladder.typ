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
