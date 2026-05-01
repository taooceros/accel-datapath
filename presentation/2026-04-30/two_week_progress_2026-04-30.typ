// Advisor-facing two-week progress presentation, 2026-04-30
// Reader: advisor / project collaborator reviewing the research direction.
// Small claim: the Rust IDXD path is now cleaner and has a small real-hardware proof; the next step is a Tonic experiment modeled after the earlier dsa-stdexec layer-removal method.
// Sources:
// - presentation/2026-03-31/progress_2026-03-31.typ
// - presentation/2026-04-08/tonic_research_story.typ
// - docs/report/benchmarking/006.stdexec_overhead_results.md
// - docs/report/benchmarking/012.tonic_characterization_refinement_results.md
// - docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md
// - docs/report/benchmarking/014.idxd_tonic_same_repo_claim_package.md
// - docs/report/hw_eval/011.m011_s03_representative_ops_2026-04-30.md
// - docs/report/benchmarking/015.m011_representative_idxd_numbers_2026-04-30.md
// - docs/report/architecture/017.generic_idxd_elegance_audit.md
// - .gsd/OVERRIDES.md
// - .gsd/DECISIONS.md
// - .gsd/milestones/M004/M004-CONTEXT.md
// - .gsd/milestones/M006/M006-CONTEXT.md
// - .gsd/milestones/M010/M010-CONTEXT.md
// - .gsd/milestones/M011/M011-CONTEXT.md
// - dsa-stdexec/benchmark/dsa/README.md
// - dsa-stdexec/benchmark/dsa/strategies/README.md

#import "../template.typ": callout, card, deck, note, palette, panel, stage-card

#show: deck.with(
  margin: (x: 42pt, y: 30pt),
  size: 13.4pt,
  leading: 0.84em,
  spacing: 0.66em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red
#let c-row = palette.row

#let metric-card(label, value, body, fill: c-row, accent: c-accent) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 12pt, y: 10pt),
  fill: fill,
  stroke: 0.6pt + palette.border,
)[
  #text(size: 10.5pt, fill: luma(90))[#label]
  #v(0.15em)
  #text(size: 21pt, weight: "bold", fill: accent)[#value]
  #v(0.15em)
  #text(size: 10.2pt, fill: luma(65))[#body]
]

= Two-week progress update

#align(center + horizon)[
  #text(size: 18pt)[Building the Tonic experiment from the DSA lesson]
  #v(0.7em)
  #text(size: 15pt)[Hongtao Zhang]
  #v(0.25em)
  #text(size: 13pt, fill: luma(120))[Progress window: Apr 16--30, 2026]
]

#v(0.7em)

#callout(fill: c-blue, stroke: c-accent)[
  The simple story: earlier `dsa-stdexec` experiments showed how to test accelerator overhead by removing one software layer at a time. Over the last two weeks, I made the Rust IDXD path clean enough to run that style of experiment inside Tonic.
]

== The big picture

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[What we learned before]
    #v(0.25em)
    + DSA can be fast when submission cost is amortized.
    + Then the software path becomes visible.
    + The useful method was not guessing; it was comparing controlled layers.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What I did now]
    #v(0.25em)
    + Cleaned up the Rust IDXD crates.
    + Built a clearer async memmove path.
    + Proved one DSA operation and one IAX operation on hardware.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[What this enables]
    #v(0.25em)
    + A Tonic experiment with the same discipline.
    + Ordinary path, software-control path, and IDXD path can be compared.
    + The claim can stay small until the data supports more.
  ]],
)

#v(0.35em)

#note[
  Small claim for today: the Rust IDXD path is cleaner and has a limited real-hardware proof. I am not claiming a Tonic speedup yet.
]

== Reminder: why the old DSA experiment mattered

#grid(
  columns: (1.18fr, 0.82fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Layer-removal result]
    #v(0.35em)
    #table(
      columns: (1.45fr, 0.78fr, 0.7fr, 0.66fr),
      inset: (x: 6pt, y: 5pt),
      stroke: 0.4pt + luma(200),
      [#text(weight: "bold")[Path]],
      [#text(weight: "bold", size: 10.5pt)[Throughput]],
      [#text(weight: "bold", size: 10.5pt)[Per-op]],
      [#text(weight: "bold", size: 10.5pt)[vs base]],
      [Full stdexec], [26.3 Mpps], [38.0 ns], [1.00x],
      [Direct path], [41.6 Mpps], [24.0 ns], [1.58x],
      [Reusable ops], [59.9 Mpps], [16.7 ns], [2.28x],
    )
    #v(0.4em)
    #card(
      [The key number],
      [Removing framework layers cut per-operation cost by about `56%`: from `38.0 ns` to `16.7 ns`. At low concurrency, the reusable path reached `84 Mpps`.],
      fill: white,
      body-size: 10.8pt,
    )
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Real hardware context]
    #v(0.35em)
    #card(
      [DSA lower bound],
      [A later hardware-floor run reached `48.4 Mops/s` for `64 B` memmove with pipelined batching.],
      fill: white,
      body-size: 10.8pt,
    )
    #v(0.4em)
    #card(
      [What that means],
      [The device is not the only problem. Once hardware work is cheap enough, software structure can decide whether offload helps.],
      fill: white,
      body-size: 10.8pt,
    )
  ]],
)

#v(0.3em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  This is the method I want to reuse: build a ladder of comparable paths, remove one cost at a time, and only then say where the bottleneck is.
]

== Reusing that method for Tonic

#table(
  columns: (0.92fr, 1.05fr, 1.03fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Experiment step]],
  [#text(weight: "bold")[What `dsa-stdexec` did]],
  [#text(weight: "bold")[Tonic version]],

  [1. Baseline],
  [Full stdexec path with the normal sender/receiver stack.],
  [Ordinary Tonic path with instrumentation off for throughput.],

  [2. Software controls],
  [Direct and reusable strategies removed framework costs one layer at a time.],
  [Pooled buffers, copy-minimized paths, and lower-overhead stage counters.],

  [3. Hardware path],
  [Real DSA run checked whether the lower software cost transfers to hardware.],
  [Prepared-host IDXD path through the same workload and artifact format.],

  [4. Matched comparison],
  [Same operation, size, concurrency, and strategy labels.],
  [Same payload shape, size, concurrency, runtime, and endpoint split.],
)

#v(0.45em)

#callout(fill: c-blue, stroke: c-accent)[
  The next Tonic experiment should be a matched ladder: ordinary Tonic, software control Tonic, then IDXD Tonic.
]

== What changed in the last two weeks

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Cleaner measurement boundary]
    #v(0.25em)
    + The current Tonic package now says plainly what it can and cannot prove.
    + The current rows do not show an IDXD win.
    + The workflow is still useful because it is rerunnable and reviewable.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Cleaner Rust path]
    #v(0.25em)
    + `idxd-rust` is now the safe Rust crate.
    + `idxd-sys` is the raw UAPI/MMIO crate.
    + Async memmove now uses explicit owned buffers.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Small hardware proof]
    #v(0.25em)
    + `IdxdSession<Dsa>` ran DSA memmove.
    + `IdxdSession<Iax>` ran IAX crc64.
    + A small release-mode benchmark produced positive rows.
  ]],
)

#v(0.35em)

#note[
  I see this as preparation work for the right experiment, not as the final application result.
]

== Tonic today: useful, but not a speedup claim

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  The current Tonic result is negative or inconclusive: the package proves the workflow, but the current rows do not show IDXD beating the ordinary path.
]

#v(0.25em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[What is already useful]
    #v(0.3em)
    + Software path validation exists.
    + IDXD verifier gate exists.
    + JSON / CSV / markdown comparison package exists.
    + Current rows show IDXD at roughly `0.003x`--`0.653x` of software throughput.
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[What is missing]
    #v(0.3em)
    + A fresh prepared-host IDXD rerun.
    + Matched ordinary vs IDXD artifacts from the same run context.
    + Lower-overhead stage attribution.
    + A clear crossover rule for when offload helps.
  ]],
)

#v(0.35em)

#card(
  [How I would say this to an advisor],
  [I do not want to sell a speedup. I want to say that the comparison machinery is now honest enough to run the next experiment cleanly.],
  fill: c-blue,
  body-size: 11pt,
)

== Why the Rust cleanup matters for the experiment

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Before]
    #v(0.35em)
    + Old names like `dsa-ffi` and `idxd-bindings` made ownership unclear.
    + Async request helpers hid too much buffer behavior.
    + Some proof scripts and APIs were hard to explain as one clean path.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Now]
    #v(0.35em)
    + `AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)` makes ownership explicit.
    + Direct Tokio completion owns accepted operations until completion.
    + Errors expose phase, retry, completion, and validation metadata without dumping payload bytes.
  ]],
)

#v(0.4em)

#callout(fill: c-blue, stroke: c-accent)[
  This matters because Tonic offload will fail if the memory ownership and completion path are unclear. The cleanup makes the next experiment easier to trust.
]

== Small hardware proof through the new path

#table(
  columns: (0.72fr, 0.68fr, 0.66fr, 0.62fr, 0.62fr, 0.7fr, 0.7fr),
  inset: (x: 5pt, y: 4pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold", size: 10.4pt)[Evidence]],
  [#text(weight: "bold", size: 10.4pt)[Target]],
  [#text(weight: "bold", size: 10.4pt)[Device]],
  [#text(weight: "bold", size: 10.4pt)[Bytes]],
  [#text(weight: "bold", size: 10.4pt)[Iters]],
  [#text(weight: "bold", size: 10.4pt)[Mean latency]],
  [#text(weight: "bold", size: 10.4pt)[Rate]],

  [Operation proof],
  [`dsa-memmove`],
  [`/dev/dsa/wq0.0`],
  [`64`],
  [n/a],
  [`completed`],
  [`pass`],

  [Operation proof],
  [`iax-crc64`],
  [`/dev/iax/wq1.0`],
  [`64`],
  [n/a],
  [`completed`],
  [`crc ok`],

  [Small bench],
  [`dsa-memmove`],
  [`/dev/dsa/wq0.0`],
  [`4096`],
  [`1000`],
  [`6,837 ns`],
  [`146,246 ops/s`],

  [Small bench],
  [`iax-crc64`],
  [`/dev/iax/wq1.0`],
  [`4096`],
  [`1000`],
  [`2,178 ns`],
  [`459,064 ops/s`],
)

#v(0.35em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 10pt,
  [#metric-card([Verifier], [pass], [operation proof and small benchmark both passed], fill: c-green, accent: rgb("#16a34a"))],
  [#metric-card([Build], [release], [measurements were collected in release mode], fill: c-blue)],
  [#metric-card([Failures], [`0 / 2000`], [benchmark operations completed without failed rows], fill: c-row, accent: rgb("#f97316"))],
)

#v(0.2em)

#note[
  This is a proof that the path works for two representative operations. It is not a full performance study.
]

== The next Tonic experiment I want to run

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Matched workload ladder]
    #v(0.35em)
    + Start with ordinary Tonic, instrumentation off.
    + Rerun the same points with software controls: pooled buffers and copy-minimized mode.
    + Rerun the same points through the IDXD path.
    + Compare only matched size, payload shape, concurrency, runtime, and endpoint setup.
  ]],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Evidence to collect]
    #v(0.35em)
    + Throughput and p99 from instrumentation-off runs.
    + Stage attribution from lower-overhead counters.
    + `perf` counters or flamegraphs for CPU explanation.
    + Artifact package with ordinary, software-control, and IDXD rows side by side.
  ]],
)

#v(0.4em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  This mirrors the old DSA experiment: baseline, remove software overhead, add real hardware, then compare matched rows.
]

== Which Tonic points should be first?

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#stage-card(
    [4 KiB structured],
    [The pooled-buffer control previously improved throughput by about `2.51x`.],
    [Good first test for buffer policy.],
    fill: c-row,
    accent: c-accent,
  )],
  [#stage-card(
    [1 MiB structured],
    [The earlier run was strongly memory-bound: about `63.7%` memory-bound in the refined pass.],
    [Good first test for copy movement.],
    fill: c-row,
    accent: rgb("#16a34a"),
  )],
  [#stage-card(
    [64 KiB compression],
    [Compression is payload-sensitive: structured was `0.585x`, random collapsed to `0.112x`.],
    [Use only as a gated follow-up.],
    fill: c-row,
    accent: rgb("#f97316"),
  )],
)

#v(0.45em)

#note[
  My preferred first pass is `4 KiB` and `1 MiB` uncompressed. Compression should come after the copy/buffer story is clean.
]

== My struggle with the agent — why it helped

#callout(fill: c-blue, stroke: c-accent)[
  This was not criticism. The useful loop was: the agent kept the details safe, and I kept steering the story toward what I could explain to my advisor.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Agent default]
    #v(0.3em)
    + many artifacts and exact paths
    + careful verification language
    + lots of small true statements
    + too much repo-internal wording
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[My revisions]
    #v(0.3em)
    + make one small claim
    + sound like a human talk
    + connect to the old DSA method
    + explain the next experiment
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Why it worked]
    #v(0.3em)
    + detail stayed available
    + claims stayed honest
    + the narrative became clearer
    + collaboration improved the slide
  ]],
)

#v(0.45em)

#callout(fill: c-blue, stroke: c-accent)[
  The final story came from combining agent memory with human judgment: keep the evidence, but present the research argument.
]

== What the milestone trail revealed

#callout(fill: c-blue, stroke: c-accent)[
  The repeated problem was not missing effort. It was that the agent often moved forward on a technically workable path before the deeper research/API standard was explicit.
]

#v(0.3em)

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Where I was unsatisfied]
    #v(0.25em)
    + duplicate FFI/package surfaces before integration
    + handwritten descriptor ABI instead of bindgen-backed truth
    + source-only or double-copy memmove APIs
    + host-free proof where real hardware proof mattered
    + cleanup that added abstraction weight
    + correct details without a clear research claim
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What I kept correcting toward]
    #v(0.25em)
    + one canonical `idxd-sys` / `idxd-rust` stack
    + explicit source and destination ownership
    + zero-copy / minimal-copy as API pressure
    + proof strength matched to claim strength
    + lean `bon` / `snafu` only where useful
    + small, elegant, replaceable abstractions
  ]],
)

#v(0.35em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 10pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[M003 → M004]
    #v(0.2em)
    I stopped integration because duplicated FFI surfaces and bad ABI/API foundations remained.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[M006 → M007]
    #v(0.2em)
    The async API moved toward explicit `Bytes` / `BytesMut` ownership, then completion-record-driven Tokio proof.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[M010 → M011]
    #v(0.2em)
    The standard became explicit: clean, lean, elegant, hardware-backed where claimed, and no avoidable duplicate paths.
  ]],
)

#v(0.35em)

#callout(fill: c-blue, stroke: c-accent)[
  Lesson: the agent is useful when it preserves evidence, but the milestone only becomes satisfying when the claim, API taste, and proof standard are set before implementation.
]

== What I want feedback on

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Decision question]
    #v(0.35em)
    Does this experiment ladder answer the advisor-level question: #text(weight: "bold")[when does the software path erase the benefit of accelerator offload?]
    #v(0.5em)
    If yes, I should spend the next step on the prepared-host Tonic rerun.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Small conclusion]
    #v(0.35em)
    + The old DSA result gave the method.
    + The last two weeks made the Rust IDXD path usable for that method.
    + The next experiment should decide whether the method carries into Tonic.
  ]],
)

#v(0.5em)

#callout(fill: c-blue, stroke: c-accent)[
  Bottom line: I am not presenting a Tonic speedup result yet. I am presenting a cleaner path to run the right experiment, with enough hardware proof to make that next experiment credible.
]
