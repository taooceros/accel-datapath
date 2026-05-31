#import "../support.typ": *

#let setup() = [
  = Experiment 4: completion handling loop

  #grid(
    columns: (0.56fr, 0.44fr),
    gutter: 18pt,
    [
      #section-label[Reuse loop]
      #v(0.15em)
      #seqbox[
        #chronos.diagram({
          import chronos: *
          _par("DSA", display-name: "DSA")
          _par("C", display-name: "Completion line")
          _par("CPU", display-name: "CPU")
          _par("WQ", display-name: "WQ")

          _seq("DSA", "C", comment: "completion write")
          _seq("CPU", "C", comment: "MEASURE: harvest policy", enable-dst: true, disable-dst: true, lifeline-style: (
            fill: rgb("#2563eb"),
          ))
          _seq("CPU", "C", comment: "MEASURE: reset timing", enable-dst: true, disable-dst: true, lifeline-style: (
            fill: rgb("#ea580c"),
          ))
          _seq("CPU", "WQ", comment: "MEASURE: resubmit timing", enable-dst: true, lifeline-style: (
            fill: rgb("#dc2626"),
          ))
          _seq("WQ", "DSA", comment: "next descriptor", dashed: true, disable-src: true)
        })
      ]

      #v(0.25em)
      #compact-table(
        columns: (0.40fr, 0.60fr),
        [#chip([layout], color: c-accent)],
        [packed / padded],
        [#chip([poll], color: rgb("#16a34a"))],
        [scan-all / round-robin],
        [#chip([reset], color: rgb("#ea580c"))],
        [none / delayed / immediate],
        [#chip([submit], color: rgb("#dc2626"))],
        [per-completion / batch harvest],
      )
    ],
    [
      #section-label[Readout]
      #v(0.25em)
      #metric-pill[throughput sensitivity to reuse policy]

      #v(0.25em)
      #fill-key(
        [#fill-item(rgb("#2563eb"))[completion-line harvest]],
        [#fill-item(rgb("#ea580c"))[completion-line reset]],
        [#fill-item(rgb("#dc2626"))[WQ resubmit]],
      )

      #v(0.45em)

      #compact-table(
        columns: (0.42fr, 0.58fr),
        [#chip([padding helps], color: rgb("#16a34a"))],
        [ownership / false sharing],
        [#chip([scan hurts], color: c-accent)],
        [poll load stream],
        [#chip([batch helps], color: rgb("#ea580c"))],
        [burstiness / cache churn],
        [#chip([no movement], color: rgb("#dc2626"))],
        [lower-level path],
      )
    ],
  )
]

#let why_run_it() = [
  = Experiment 4: why run it

  #grid(
    columns: (0.38fr, 0.62fr),
    gutter: 18pt,
    [
      #section-label[Why run it]
      #v(0.25em)
      #compact-table(
        columns: (0.33fr, 0.67fr),
        [#chip([why], color: c-accent)],
        [CPU completion handling can be the slow part],
        [#chip([learn], color: rgb("#16a34a"))],
        [which CPU policy changes throughput],
        [#chip([record], color: c-title)],
        [ops/sec, polls/done, reset-to-submit cycles],
      )

      #v(0.55em)
      #soft-box(fill: white)[
        If padding, scan order, or batch harvesting changes the curve, fix the CPU completion path before changing hardware submission.
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
        for policy in [
            PackedScan,
            PaddedRoundRobin,
            PollOnly,
            DelayedReset,
            BatchHarvest,
        ] {
            bench.configure(policy);
            bench.fill_window(128, Op::Noop);

            while bench.elapsed() < run_for {
                let done = bench.harvest(policy);
                bench.reset_done(done, policy);
                bench.resubmit(done, Op::Noop);
            }

            record("reuse", policy, bench.ops_per_sec(), bench.polls_per_done());
        }
        ```,
      )
    ],
  )
]

#let result() = [
  = Experiment 4 result: reuse policy is not the first bend

  #callout(fill: c-orange, stroke: rgb("#ea580c"), inset: (x: 16pt, y: 9pt))[
    #grid(
      columns: (0.34fr, 0.33fr, 0.33fr),
      gutter: 12pt,
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#c2410c"))[NOOP best]
        #v(0.1em)
        #text(size: 21pt, weight: "bold", fill: c-title)[8.51 Mops/s]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#c2410c"))[memmove64 best]
        #v(0.1em)
        #text(size: 21pt, weight: "bold", fill: c-title)[8.53 Mops/s]
      ],
      [
        #text(size: 10pt, weight: "bold", fill: rgb("#c2410c"))[correctness]
        #v(0.1em)
        #text(size: 21pt, weight: "bold", fill: c-title)[0 missing]
      ],
    )
  ]

  #v(0.35em)

  #grid(
    columns: (0.62fr, 0.38fr),
    gutter: 16pt,
    [
      #lq-diagram(
        title: [Sustained throughput by reuse policy],
        xlabel: [policy],
        ylabel: [Mops/s],
        xlim: (-0.7, 4.7),
        ylim: (0, 9.5),
        xaxis: (
          ticks: (0, 1, 2, 3, 4).zip(([pack], [pad], [poll], [delay], [batch])),
          subticks: none,
        ),
        yaxis: (ticks: (0, 3, 6, 9), subticks: none),
        lq-bar(
          range(5),
          (8.38, 8.51, 7.00, 7.47, 8.38),
          offset: -0.18,
          width: 0.34,
          label: [NOOP],
        ),
        lq-bar(
          range(5),
          (8.47, 5.19, 7.33, 7.59, 8.53),
          offset: 0.18,
          width: 0.34,
          label: [64B],
        ),
      )
    ],
    [
      #section-label[Attribution]
      #v(0.25em)
      #metric-pill(color: rgb("#ea580c"))[policy moves throughput, not the admission knee]
      #v(0.35em)
      #soft-box(fill: white, stroke: c-title, inset: (x: 12pt, y: 8pt))[
        Packed scan, padded round-robin, and batch harvest stay in the same rough band for NOOP.
      ]
      #v(0.3em)
      #soft-box(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 12pt, y: 7pt))[
        Every policy completed `1,000,000` operations with missing = 0 and errors = 0.
      ]
    ],
  )

  #v(0.25em)
  #source-line[Source: docs/report/benchmarking/020.submission_bottleneck_experiments_2026-05-27.md, Experiment 4.]
]
