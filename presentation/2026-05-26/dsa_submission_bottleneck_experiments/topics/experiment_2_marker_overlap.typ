#import "../support.typ": *

#let req-box(body, fill: c-row, stroke: luma(190), width: 34pt) = block(
  width: width,
  height: 23pt,
  radius: 4pt,
  inset: 0pt,
  fill: fill,
  stroke: 0.55pt + stroke,
)[#align(center + horizon)[#text(size: 8.2pt, weight: "bold", fill: c-title)[#body]]]

#let gap-box() = block(
  width: 22pt,
  height: 23pt,
  inset: 0pt,
)[#align(center + horizon)[#text(size: 13pt, fill: luma(100))[⋯]]]

#let wall-box() = block(
  width: 40pt,
  height: 23pt,
  radius: 4pt,
  inset: 0pt,
  fill: c-orange,
  stroke: 0.65pt + rgb("#ea580c"),
)[#align(center + horizon)[#text(size: 8.3pt, weight: "bold", fill: rgb("#9a3412"))[wall]]]

#let setup() = [
  = Experiment 2: trace the wall region

  #grid(
    columns: (0.56fr, 0.44fr),
    gutter: 18pt,
    [
      #section-label[Setup]
      #v(0.15em)
      #soft-box(fill: white, stroke: c-title, inset: (x: 12pt, y: 8pt))[
        #text(weight: "bold", fill: c-title)[Question]
        #v(0.18em)
        Experiment 1 found the submit wall near requests 115--117. Are early requests already complete before that wall?
      ]

      #v(0.4em)
      #align(center)[#text(size: 9.2pt, fill: luma(85))[CPU submit order; tracing starts at `poll_offset`]]
      #v(0.2em)

      #grid(
        columns: (34pt, 34pt, 34pt, 34pt, 22pt, 34pt, 40pt, 34pt),
        gutter: 3pt,
        req-box([1], fill: c-green, stroke: rgb("#16a34a")),
        req-box([2], fill: c-green, stroke: rgb("#16a34a")),
        req-box([3], fill: c-green, stroke: rgb("#16a34a")),
        req-box([4], fill: c-green, stroke: rgb("#16a34a")),
        gap-box(),
        req-box([112], fill: c-blue, stroke: c-accent),
        wall-box(),
        req-box([160]),
      )

      #v(0.5em)
      #fill-key(
        [#fill-item(c-green)[completion records tracked: comp[1..4]]],
        [#fill-item(c-blue)[first traced submit offset]],
        [#fill-item(c-orange)[admission-wall region from Experiment 1]],
      )
    ],
    [
      #section-label[Measured at every traced index]
      #v(0.25em)

      #compact-table(
        columns: (0.36fr, 0.64fr),
        [#chip([submit_tsc[i]], color: c-accent)],
        [latency of the `i`th MMIO submit],
        [#chip([poll_tsc[i,j]], color: rgb("#16a34a"))],
        [cost to read completion `j` after submit `i`],
        [#chip([visible prefix], color: c-title)],
        [how many of comp[1..4] are visible contiguously],
        [#chip([visible count], color: rgb("#ea580c"))],
        [how many of comp[1..4] are visible in any order],
      )

      #v(0.45em)
      #metric-pill[poll step = 1; offsets = 96, 112, 115]
      #v(0.35em)
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        The first poll at an offset is the boundary check; later polls intentionally trace the active-observation path.
      ]
    ],
  )
]

#let marker_position() = [
  = Poll offset: where do we start looking?

  #callout(fill: c-blue, stroke: c-accent, inset: (x: 16pt, y: 8pt))[
    We keep the marker at request 1 and vary only the first submit index where polling begins.
  ]

  #v(0.35em)

  #grid(
    columns: (0.27fr, 0.73fr),
    gutter: 14pt,
    row-gutter: 10pt,
    [
      #chip([offset 96], color: c-accent)
    ],
    [
      #soft-box(fill: white, inset: (x: 12pt, y: 7pt))[
        Starts well before the wall. Useful for seeing how active polling changes the subsequent submit stream.
      ]
    ],
    [
      #chip([offset 112], color: rgb("#16a34a"))
    ],
    [
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        Boundary check before the wall is expected to bite. If comp[1] is visible here, DSA completed early work before the wall.
      ]
    ],
    [
      #chip([offset 115], color: rgb("#ea580c"))
    ],
    [
      #soft-box(fill: c-orange, stroke: rgb("#ea580c"), inset: (x: 12pt, y: 7pt))[
        Checks the wall itself. The next submit can expose the wall while the first poll shows what had already completed.
      ]
    ],
  )

  #v(0.5em)
  #metric-pill(color: c-title)[decision axis: first visible completion index vs. submit-cost wall index]
]

#let why_run_it() = [
  = Experiment 2: why run it

  #grid(
    columns: (0.38fr, 0.62fr),
    gutter: 18pt,
    [
      #section-label[Why run it]
      #v(0.25em)
      #compact-table(
        columns: (0.33fr, 0.67fr),
        [#chip([concern], color: c-accent)],
        [cheap submits may be only upstream buffering],
        [#chip([learn], color: rgb("#16a34a"))],
        [whether comp[1..4] are already visible by index 112/115],
        [#chip([cost], color: rgb("#ea580c"))],
        [how expensive the observation poll is],
      )

      #v(0.55em)
      #soft-box(fill: c-green)[
        If comp[1] is visible at the first poll at index 112, then request 1 was not waiting for the wall to become device-visible.
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
        for i in 1..=n {
            let t0 = rdtscp();
            submit(desc[i]);
            let t1 = rdtscp();

            if i >= poll_offset {
                trace.submit[i].push(t1 - t0);

                for j in 1..=4 {
                    let p0 = rdtscp();
                    let status = comp[j].status();
                    let p1 = rdtscp();
                    trace.poll[i][j].push(status, p1 - p0);
                }
            }
        }
        ```,
      )
    ],
  )
]

#let polling_control() = [
  = Poll cost is part of the measurement

  #grid(
    columns: (0.48fr, 0.52fr),
    gutter: 18pt,
    [
      #section-label[Poll states]
      #v(0.3em)

      #soft-box(fill: c-blue)[
        #text(weight: "bold", fill: c-accent)[NONE poll]
        #v(0.15em)
        CPU reads a completion line before hardware has written success.
      ]

      #v(0.35em)

      #soft-box(fill: c-green)[
        #text(weight: "bold", fill: rgb("#16a34a"))[first SUCCESS poll]
        #v(0.15em)
        First observed device-written success on that line.
      ]

      #v(0.35em)

      #soft-box(fill: c-row)[
        #text(weight: "bold", fill: c-title)[hot SUCCESS poll]
        #v(0.15em)
        Later reads after the completion line is already hot to the CPU.
      ]
    ],
    [
      #section-label[Why keep submit latency too?]
      #v(0.25em)

      #soft-box(fill: white, stroke: c-title, inset: (x: 14pt, y: 8pt))[
        Polling can stretch the loop, but submit latency still tells us when the wall appears in the traced run.
      ]

      #v(0.45em)

      #compact-table(
        columns: (0.42fr, 0.58fr),
        [#chip([first poll], color: rgb("#16a34a"))],
        [boundary observation at the configured offset],
        [#chip([later polls], color: c-accent)],
        [active-observation trace],
        [#chip([submit trace], color: rgb("#ea580c"))],
        [does the wall still appear?],
      )

      #v(0.45em)
      #metric-pill[record a list, not only one aggregate]
    ],
  )
]

#let result() = [
  = Experiment 2 result: completions are visible by the wall

  #callout(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 16pt, y: 9pt))[
    #grid(
      columns: (0.34fr, 0.33fr, 0.33fr),
      gutter: 12pt,
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#15803d"))[offset 112]
        #v(0.1em)
        #text(size: 20pt, weight: "bold", fill: c-title)[comp[1..4]]
        #v(0.05em)
        visible at first poll
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#15803d"))[offset 115]
        #v(0.1em)
        #text(size: 20pt, weight: "bold", fill: c-title)[comp[1..4]]
        #v(0.05em)
        visible at wall
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#15803d"))[correctness]
        #v(0.1em)
        #text(size: 20pt, weight: "bold", fill: c-title)[160 / 160]
        #v(0.05em)
        completed; 0 missing/errors
      ],
    )
  ]

  #v(0.35em)

  #grid(
    columns: (0.58fr, 0.42fr),
    gutter: 16pt,
    [
      #lq-diagram(
        title: [Submit latency trace, offset 115],
        xlabel: [submit index],
        ylabel: [median TSC ticks],
        xlim: (114, 161),
        ylim: (0, 520),
        xaxis: (ticks: (115, 120, 128, 144, 160), subticks: none),
        yaxis: (ticks: (0, 100, 300, 500), subticks: none),
        lq-plot(
          (115, 116, 117, 120, 128, 144, 160),
          (28, 470, 28, 28, 28, 28, 28),
          stroke: 1.6pt + rgb("#ea580c"),
          mark: "o",
          label: [submit],
        ),
      )
    ],
    [
      #section-label[First-poll summary]
      #v(0.22em)
      #compact-table(
        columns: (0.24fr, 0.25fr, 0.25fr, 0.26fr),
        table.header([offset], [prefix], [comp1], [poll]),
        [96], [0], [0.016], [78 ticks],
        [112], [4], [1.000], [72 ticks],
        [115], [4], [1.000], [70 ticks],
      )

      #v(0.35em)
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        At offsets 112 and 115, the first four completions are already visible at the first observation.
      ]

      #v(0.25em)
      #soft-box(fill: c-blue, stroke: c-accent, inset: (x: 12pt, y: 7pt))[
        Hot success polls settle near 22 TSC ticks after the first success observation.
      ]
    ],
  )

  #v(0.25em)
  #source-line[Source: docs/report/benchmarking/021.submit_marker_trace_2026-05-27.md.]
]