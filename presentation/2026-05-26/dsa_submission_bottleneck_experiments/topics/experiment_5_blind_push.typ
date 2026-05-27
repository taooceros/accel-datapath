#import "../support.typ": *

#let setup() = [
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
          _seq("DSA", "C", comment: "write completions", enable-dst: true, disable-src: true, lifeline-style: (
            fill: rgb("#16a34a"),
          ))
          _seq("CPU", "C", comment: "count done records", dashed: true)
        })
      ]

      #v(0.25em)
      #compact-table(
        columns: (0.34fr, 0.66fr),
        [#chip([N], color: c-accent)],
        [`depth/2`, `depth`, `2×depth`, `4×depth`],
        [#chip([op], color: rgb("#16a34a"))],
        [`NOOP` first; optional `64B memmove`],
        [#chip([rule], color: rgb("#dc2626"))],
        [do not poll while pushing],
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
        [#chip([same count], color: rgb("#16a34a"))],
        [no loss seen in this run],
        [#chip([missing ids], color: rgb("#dc2626"))],
        [some pushes were dropped],
        [#chip([loss after depth], color: rgb("#ea580c"))],
        [WQ full behavior],
        [#chip([SWERROR set], color: c-title)],
        [hardware noticed bad submit],
      )
    ],
  )

  #v(0.35em)
  #callout(fill: c-red, stroke: rgb("#dc2626"), inset: (x: 14pt, y: 7pt))[
    This intentionally stops counting in-flight work to test whether requests can disappear.
  ]
]

#let why_run_it() = [
  = Experiment 5: why run it

  #grid(
    columns: (0.38fr, 0.62fr),
    gutter: 18pt,
    [
      #section-label[Why run it]
      #v(0.25em)
      #compact-table(
        columns: (0.33fr, 0.67fr),
        [#chip([why], color: c-accent)],
        [`movdir64b` does not tell us "WQ accepted it"],
        [#chip([learn], color: rgb("#16a34a"))],
        [whether pushing without counting loses requests],
        [#chip([record], color: c-title)],
        [pushed, completed, missing ids, SWERROR],
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
]
