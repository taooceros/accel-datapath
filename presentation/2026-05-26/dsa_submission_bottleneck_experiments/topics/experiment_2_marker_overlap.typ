#import "../support.typ": *

#let setup() = [
  = Experiment 2: marker completion during submit

  #grid(
    columns: (0.56fr, 0.44fr),
    gutter: 18pt,
    [
      #section-label[Setup]
      #v(0.15em)
      #seqbox[
        #chronos.diagram({
          import chronos: *
          _par("CPU", display-name: "CPU submitter")
          _par("WQ", display-name: "WQ")
          _par("DSA", display-name: "DSA")
          _par("C", display-name: "Completion line")

          _seq("CPU", "WQ", comment: "fillers before p")
          _seq("CPU", "WQ", comment: "MEASURE: tp marker", enable-dst: true, lifeline-style: (fill: rgb("#2563eb")))
          _seq("CPU", "WQ", comment: "fillers after p")
          _seq("WQ", "DSA", comment: "marker reaches DSA?", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
          _seq("DSA", "C", comment: "completion write", enable-dst: true, disable-src: true, lifeline-style: (
            fill: rgb("#16a34a"),
          ))
          _seq("C", "CPU", comment: "MEASURE: poll every M", enable-dst: true, lifeline-style: (fill: rgb("#ea580c")))
          _seq("C", "CPU", comment: "tc seen", dashed: true, disable-dst: true)
          _seq("WQ", "CPU", comment: "t1 burst done", dashed: true, disable-src: true)
        })
      ]

      #v(0.25em)
      #compact-table(
        columns: (0.32fr, 0.68fr),
        [#chip([p], color: c-title)],
        [`1`, `N/2`, `N`],
        [#chip([M], color: c-accent)],
        [`1`, `4`, `16`, `64`, `never`],
        [#chip([fillers], color: rgb("#16a34a"))],
        [no-completion NOOP first],
      )
    ],
    [
      #section-label[Readout]
      #v(0.25em)
      #metric-pill[overlap = tc < t1]

      #v(0.25em)
      #fill-key(
        [#fill-item(rgb("#2563eb"))[remaining submit window]],
        [#fill-item(rgb("#16a34a"))[marker device/completion path]],
        [#fill-item(rgb("#ea580c"))[poll every M]],
      )

      #v(0.45em)

      #compact-table(
        columns: (0.40fr, 0.60fr),
        [#chip([tc < t1], color: rgb("#16a34a"))],
        [device overlaps submit],
        [#chip([only tight M], color: rgb("#ea580c"))],
        [poll-sensitive],
        [#chip([no tc < t1], color: rgb("#dc2626"))],
        [late completion or long latency],
      )

      #v(0.35em)
      #soft-box(fill: white)[same core; one TSC stream]
    ],
  )
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
        [#chip([why], color: c-accent)],
        [a burst can hide whether DSA work overlaps CPU submission],
        [#chip([learn], color: rgb("#16a34a"))],
        [whether marker completion appears before the submit loop ends],
        [#chip([control], color: c-title)],
        [poll every `M` submits to test polling cost],
      )

      #v(0.55em)
      #soft-box(fill: c-green)[
        A marker that completes before `t1` means DSA work overlaps the submit tail. If the answer changes only with tight polling, polling is changing the result.
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
        for p in [1, n / 2, n] {
            for m in [1, 4, 16, 64, usize::MAX] {
                bench.reset_slots();
                let mut tc = None;

                for i in 0..n {
                    let op = if i == p { Marker } else { NoComp };
                    bench.submit_one(op);

                    if i % m == 0 {
                        tc = tc.or_else(|| bench.poll_s());
                    }
                }

                let t1 = ticks();
                tc = tc.or_else(|| bench.wait_s());
                record("marker", p, m, tc.unwrap() < t1);
            }
        }
        ```,
      )
    ],
  )
]

#let polling_control() = [
  = Polling can change the result

  #grid(
    columns: (0.48fr, 0.52fr),
    gutter: 18pt,
    [
      #section-label[What can go wrong]
      #v(0.3em)

      #soft-box(fill: c-blue)[
        #text(weight: "bold", fill: c-accent)[1. CPU keeps reading `status = 0`]
        #v(0.15em)
        This is not free: it is a tight load loop.
      ]

      #v(0.35em)

      #soft-box(fill: c-green)[
        #text(weight: "bold", fill: rgb("#16a34a"))[2. DSA writes `done`]
        #v(0.15em)
        The completion line changes owner / state.
      ]

      #v(0.35em)

      #soft-box(fill: c-red)[
        #text(weight: "bold", fill: rgb("#dc2626"))[3. CPU resets the same line]
        #v(0.15em)
        Reusing the slot may add cacheline traffic.
      ]
    ],
    [
      #section-label[Why this control exists]
      #v(0.25em)

      #soft-box(fill: white, stroke: c-title, inset: (x: 14pt, y: 8pt))[
        Are we measuring DSA, or are we measuring our polling loop?
      ]

      #v(0.45em)

      #compact-table(
        columns: (0.38fr, 0.62fr),
        [#chip([poll less], color: c-accent)],
        [does the result move?],
        [#chip([poll tight], color: rgb("#16a34a"))],
        [best possible notice time],
        [#chip([delay reset], color: rgb("#ea580c"))],
        [separate reset cost],
        [#chip([pad slots], color: rgb("#dc2626"))],
        [test cacheline sharing],
      )

      #v(0.45em)

      #metric-pill[if polling changes throughput, fix polling first]
    ],
  )
]
