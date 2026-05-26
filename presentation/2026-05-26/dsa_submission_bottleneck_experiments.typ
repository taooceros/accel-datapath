// Diagnostic experiment deck: DSA submission bottleneck localization.
// Reader: advisor / project collaborator.
// Claim boundary: experiment design, not a final root-cause claim.
// Sources:
// - docs/plan/2026-05-26/01.dsa-submission-bottleneck-experiment-slide.plan.md
// - docs/report/benchmarking/018.dsa_submit_workload_study_2026-05-24.md
// - docs/report/literature/papers/understanding-the-host-network/paper.md
// - docs/report/literature/005.accelerator_hostpath_2026-03-28.md
// - Direct latency measurement in the 2026-05-26 working session.

#import "../template.typ": callout, deck, palette, panel
#import "@preview/chronos:0.3.0" as chronos
#import "@preview/zebraw:0.6.3": *

#show: deck.with(
  margin: (x: 44pt, y: 32pt),
  size: 13.2pt,
  leading: 0.84em,
  spacing: 0.54em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-row = palette.row
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red

#let source-line(body) = text(size: 7.3pt, fill: luma(115))[#body]

#let soft-box(body, fill: c-row, stroke: palette.border, inset: (x: 14pt, y: 9pt)) = block(
  width: 100%,
  radius: 7pt,
  inset: inset,
  fill: fill,
  stroke: 0.55pt + stroke,
  body,
)

#let section-label(body, color: c-title) = text(size: 15.5pt, weight: "bold", fill: color)[#body]

#let phase(label, body, color) = [
  #text(weight: "bold", fill: color)[#label]
  #h(0.35em)
  #text(fill: luma(65))[#body]
]

#let compact-table(..args) = table(
  inset: (x: 8pt, y: 5.5pt),
  stroke: 0.42pt + palette.border,
  ..args,
)

#let metric-pill(body, color: c-title) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 10pt, y: 6pt),
  stroke: 0.55pt + palette.border,
  fill: white,
  text(font: "Latin Modern Mono", size: 10.2pt, fill: color)[#body],
)

#let chip(body, color: c-title) = text(weight: "bold", fill: color)[#body]

#let swatch(color) = box(width: 8pt, height: 8pt, fill: color, stroke: 0.35pt + luma(60))

#let fill-key(..items) = compact-table(
  columns: (1fr,),
  inset: (x: 7pt, y: 4pt),
  table.header([#text(size: 9.8pt, weight: "bold", fill: c-title)[lifeline fill = measured region]]),
  ..items,
)

#let fill-item(color, body) = [#swatch(color)#h(0.45em)#body]

#let seqbox(body) = scale(x: 74%, y: 74%)[#body]

#let code-note(body) = text(size: 8.6pt, fill: luma(90))[#body]

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

#pagebreak()

= Host path view: what to measure

#grid(
  columns: (0.46fr, 0.54fr),
  gutter: 22pt,
  [
    #section-label[Where credits live]
    #v(0.35em)
    #compact-table(
      columns: (0.36fr, 0.64fr),
      [#chip([domain], color: c-title)], [processor / memory / peripheral path],
      [#chip([credits], color: c-accent)], [outstanding request budget],
      [#chip([unloaded latency], color: rgb("#16a34a"))], [cheap path when credits exist],
      [#chip([wait], color: rgb("#dc2626"))], [upstream work gets slowed],
    )
  ],
  [
    #section-label[DSA implication]
    #v(0.35em)
    #compact-table(
      columns: (0.42fr, 0.58fr),
      [`movdir64b` returns], [the WQ accepted the write],
      [not proved], [DSA execution],
      [not proved], [completion writeback],
      [not proved], [CPU can see completion],
      [full-loop cost], [first place that runs out of credits],
    )
  ],
)

#v(0.65em)

#callout(fill: white, stroke: c-title, inset: (x: 14pt, y: 8pt))[
  #text(size: 16pt, weight: "bold")[Design rule:]
  #h(0.5em)
  vary one pressure source → find first bend
]

#pagebreak()
= The DSA loop: where timestamps attach

#scale(x: 80%, y: 80%)[
  #chronos.diagram({
    import chronos: *
    _par("CPU", display-name: "CPU thread")
    _par("FAB", display-name: "Host fabric / WQ admission")
    _par("DSA", display-name: "DSA engine")
    _par("CMEM", display-name: "Completion memory")

    _seq("CPU", "FAB", comment: "MEASURE t0→t1: submit/admit", enable-dst: true, lifeline-style: (fill: rgb("#2563eb")))
    _seq("FAB", "CPU", comment: "t1: return / credit stall", dashed: true, disable-src: true)
    _seq("FAB", "DSA", comment: "descriptor visible", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
    _seq("DSA", "DSA", comment: "execute")
    _seq("DSA", "CMEM", comment: "completion write", enable-dst: true, disable-src: true, lifeline-style: (fill: rgb("#16a34a")))
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

#pagebreak()

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

#pagebreak()

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
        _seq("CPU", "WQ", comment: "MEASURE: t0 extra submit", enable-dst: true, lifeline-style: (fill: rgb("#2563eb")))
        _seq("WQ", "CPU", comment: "t1 return / stall", dashed: true, disable-src: true)
        _seq("WQ", "DSA", comment: "old slot drains", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
        _seq("DSA", "C", comment: "tc first old completion", enable-dst: true, disable-src: true, lifeline-style: (fill: rgb("#16a34a")))
        _seq("CPU", "C", comment: "observe tc", dashed: true, disable-dst: true)
        _seq("CPU", "WQ", comment: "drain / reset", dashed: true)
      })
    ]

    #v(0.25em)
    #compact-table(
      columns: (0.35fr, 0.65fr),
      [#chip([x], color: c-title)], [`K = 0,32,64,96,112,120,124,127,128`],
      [#chip([y], color: c-accent)], [extra-submit cycles],
      [#chip([hold fixed], color: rgb("#16a34a"))], [op, NUMA, WQ, core],
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
      [#chip([flat], color: rgb("#16a34a"))], [submit credits remain],
      [#chip([knee near 128], color: rgb("#ea580c"))], [WQ/device credits],
      [#chip([early knee], color: rgb("#dc2626"))], [store / posting / IIO / portal],
      [#chip([full-WQ tail], color: c-title)], [wait for device progress],
    )
  ],
)

#v(0.35em)
#callout(fill: c-blue, stroke: c-accent, inset: (x: 14pt, y: 7pt))[
  Fill WQ more → see when submit gets slow.
]

#pagebreak()

= Experiment 1: why run it

#grid(
  columns: (0.38fr, 0.62fr),
  gutter: 18pt,
  [
    #section-label[Why run it]
    #v(0.25em)
    #compact-table(
      columns: (0.33fr, 0.67fr),
      [#chip([why], color: c-accent)], [fill the WQ, then time one extra submit],
      [#chip([learn], color: rgb("#16a34a"))], [when the extra submit gets slow],
      [#chip([record], color: c-title)], [`K`, submit cycles, first old done time],
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

#pagebreak()

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
        _seq("DSA", "C", comment: "completion write", enable-dst: true, disable-src: true, lifeline-style: (fill: rgb("#16a34a")))
        _seq("C", "CPU", comment: "MEASURE: poll every M", enable-dst: true, lifeline-style: (fill: rgb("#ea580c")))
        _seq("C", "CPU", comment: "tc seen", dashed: true, disable-dst: true)
        _seq("WQ", "CPU", comment: "t1 burst done", dashed: true, disable-src: true)
      })
    ]

    #v(0.25em)
    #compact-table(
      columns: (0.32fr, 0.68fr),
      [#chip([p], color: c-title)], [`1`, `N/2`, `N`],
      [#chip([M], color: c-accent)], [`1`, `4`, `16`, `64`, `never`],
      [#chip([fillers], color: rgb("#16a34a"))], [no-completion NOOP first],
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
      [#chip([tc < t1], color: rgb("#16a34a"))], [device overlaps submit],
      [#chip([only tight M], color: rgb("#ea580c"))], [poll-sensitive],
      [#chip([no tc < t1], color: rgb("#dc2626"))], [late completion or long latency],
    )

    #v(0.35em)
    #soft-box(fill: white)[same core; one TSC stream]
  ],
)

#pagebreak()

= Experiment 2: why run it

#grid(
  columns: (0.38fr, 0.62fr),
  gutter: 18pt,
  [
    #section-label[Why run it]
    #v(0.25em)
    #compact-table(
      columns: (0.33fr, 0.67fr),
      [#chip([why], color: c-accent)], [a burst can hide whether DSA work overlaps CPU submission],
      [#chip([learn], color: rgb("#16a34a"))], [whether marker completion appears before the submit loop ends],
      [#chip([control], color: c-title)], [poll every `M` submits to test polling cost],
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

#pagebreak()

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
      [#chip([poll less], color: c-accent)], [does the result move?],
      [#chip([poll tight], color: rgb("#16a34a"))], [best possible notice time],
      [#chip([delay reset], color: rgb("#ea580c"))], [separate reset cost],
      [#chip([pad slots], color: rgb("#dc2626"))], [test cacheline sharing],
    )

    #v(0.45em)

    #metric-pill[if polling changes throughput, fix polling first]
  ],
)

#pagebreak()

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
        _seq("DSA", "MEM", comment: "C MEASURE: + 64B payload", enable-dst: true, lifeline-style: (fill: rgb("#ea580c")))
        _seq("DSA", "MEM", comment: "D MEASURE: + 4KiB payload", enable-dst: true, lifeline-style: (fill: rgb("#dc2626")))
      })
    ]

    #v(0.25em)
    #compact-table(
      columns: (0.22fr, 0.78fr),
      [#chip([A], color: c-accent)], [submit-only / no completion],
      [#chip([B], color: rgb("#16a34a"))], [completion-bearing NOOP],
      [#chip([C], color: rgb("#ea580c"))], [64B memmove],
      [#chip([D], color: rgb("#dc2626"))], [4KiB memmove],
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
      [#chip([A bends], color: c-accent)], [submit alone],
      [#chip([B adds], color: rgb("#16a34a"))], [completion write / seen by CPU],
      [#chip([C/D add], color: rgb("#ea580c"))], [payload DMA domain],
      [#chip([D unique], color: rgb("#dc2626"))], [fabric / bandwidth / locality],
    )
  ],
)

#pagebreak()

= Experiment 3: why run it

#grid(
  columns: (0.38fr, 0.62fr),
  gutter: 18pt,
  [
    #section-label[Why run it]
    #v(0.25em)
    #compact-table(
      columns: (0.33fr, 0.67fr),
      [#chip([why], color: c-accent)], [change one thing per run],
      [#chip([learn], color: rgb("#16a34a"))], [which added work makes it slow],
      [#chip([compare], color: c-title)], [same WQ, core, window, NUMA node],
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

#pagebreak()

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
        _seq("CPU", "C", comment: "MEASURE: harvest policy", enable-dst: true, disable-dst: true, lifeline-style: (fill: rgb("#2563eb")))
        _seq("CPU", "C", comment: "MEASURE: reset timing", enable-dst: true, disable-dst: true, lifeline-style: (fill: rgb("#ea580c")))
        _seq("CPU", "WQ", comment: "MEASURE: resubmit timing", enable-dst: true, lifeline-style: (fill: rgb("#dc2626")))
        _seq("WQ", "DSA", comment: "next descriptor", dashed: true, disable-src: true)
      })
    ]

    #v(0.25em)
    #compact-table(
      columns: (0.40fr, 0.60fr),
      [#chip([layout], color: c-accent)], [packed / padded],
      [#chip([poll], color: rgb("#16a34a"))], [scan-all / round-robin],
      [#chip([reset], color: rgb("#ea580c"))], [none / delayed / immediate],
      [#chip([submit], color: rgb("#dc2626"))], [per-completion / batch harvest],
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
      [#chip([padding helps], color: rgb("#16a34a"))], [ownership / false sharing],
      [#chip([scan hurts], color: c-accent)], [poll load stream],
      [#chip([batch helps], color: rgb("#ea580c"))], [burstiness / cache churn],
      [#chip([no movement], color: rgb("#dc2626"))], [lower-level path],
    )
  ],
)

#pagebreak()

= Experiment 4: why run it

#grid(
  columns: (0.38fr, 0.62fr),
  gutter: 18pt,
  [
    #section-label[Why run it]
    #v(0.25em)
    #compact-table(
      columns: (0.33fr, 0.67fr),
      [#chip([why], color: c-accent)], [CPU completion handling can be the slow part],
      [#chip([learn], color: rgb("#16a34a"))], [which CPU policy changes throughput],
      [#chip([record], color: c-title)], [ops/sec, polls/done, reset-to-submit cycles],
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

#pagebreak()

= Experiment 5: push without counting

#grid(
  columns: (0.56fr, 0.44fr),
  gutter: 18pt,
  [
    #section-label[Setup]
    #v(0.15em)
    #seqbox[
      #chronos.diagram({
        import chronos: *
        _par("CPU", display-name: "CPU")
        _par("WQ", display-name: "Dedicated WQ")
        _par("DSA", display-name: "DSA")
        _par("C", display-name: "Completion records")

        _seq("CPU", "WQ", comment: "push N requests")
        _seq("CPU", "WQ", comment: "no in-flight count", enable-dst: true, lifeline-style: (fill: rgb("#dc2626")))
        _seq("WQ", "DSA", comment: "accepted requests", enable-dst: true, lifeline-style: (fill: rgb("#16a34a")))
        _seq("WQ", "CPU", comment: "possible drop after full", dashed: true, disable-src: true)
        _seq("DSA", "C", comment: "write completions", enable-dst: true, disable-src: true, lifeline-style: (fill: rgb("#16a34a")))
        _seq("CPU", "C", comment: "count done records", dashed: true)
      })
    ]

    #v(0.25em)
    #compact-table(
      columns: (0.34fr, 0.66fr),
      [#chip([N], color: c-accent)], [`depth/2`, `depth`, `2×depth`, `4×depth`],
      [#chip([op], color: rgb("#16a34a"))], [`NOOP` first; optional `64B memmove`],
      [#chip([rule], color: rgb("#dc2626"))], [do not poll while pushing],
    )
  ],
  [
    #section-label[Readout]
    #v(0.25em)
    #metric-pill[completed count vs. pushed count]

#v(0.25em)
#fill-key(
  [#fill-item(rgb("#dc2626"))[push past WQ depth]],
  [#fill-item(rgb("#16a34a"))[accepted requests complete]],
)

    #v(0.45em)

    #compact-table(
      columns: (0.42fr, 0.58fr),
      [#chip([same count], color: rgb("#16a34a"))], [no loss seen in this run],
      [#chip([missing ids], color: rgb("#dc2626"))], [some pushes were dropped],
      [#chip([loss after depth], color: rgb("#ea580c"))], [WQ full behavior],
      [#chip([SWERROR set], color: c-title)], [hardware noticed bad submit],
    )
  ],
)

#v(0.35em)
#callout(fill: c-red, stroke: rgb("#dc2626"), inset: (x: 14pt, y: 7pt))[
  This intentionally stops counting in-flight work to test whether requests can disappear.
]

#pagebreak()

= Experiment 5: why run it

#grid(
  columns: (0.38fr, 0.62fr),
  gutter: 18pt,
  [
    #section-label[Why run it]
    #v(0.25em)
    #compact-table(
      columns: (0.33fr, 0.67fr),
      [#chip([why], color: c-accent)], [`movdir64b` does not tell us "WQ accepted it"],
      [#chip([learn], color: rgb("#16a34a"))], [whether pushing without counting loses requests],
      [#chip([record], color: c-title)], [pushed, completed, missing ids, SWERROR],
    )

    #v(0.55em)
    #soft-box(fill: c-red)[
      If pushed count is larger than completed count, then we need in-flight tracking before trusting any throughput number from a dedicated WQ.
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
      for n in [depth / 2, depth, 2 * depth, 4 * depth] {
          bench.reset_unique_slots(n);

          for id in 0..n {
              bench.submit_no_wait(id, Op::Noop);
          }

          let done = bench.wait_done_until(timeout);
          let missing = bench.missing_ids(0..n);
          let swerr = bench.read_software_error();

          record("no-count-push", n, done.len(), missing, swerr);
      }
      ```,
    )
    #v(0.25em)
    #code-note[Use one completion slot per pushed request; otherwise slot reuse can hide loss.]
  ],
)
#pagebreak()

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