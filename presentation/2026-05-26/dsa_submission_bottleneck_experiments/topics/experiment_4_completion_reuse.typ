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
