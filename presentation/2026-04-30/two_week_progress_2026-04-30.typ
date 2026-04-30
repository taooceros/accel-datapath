// Simplified two-week progress presentation, 2026-04-30
// Reader: technical advisor / project collaborator reviewing the last two weeks.
// Small claim: the Rust IDXD path is now cleaner and has a small real-hardware proof; this is not yet a Tonic speedup claim.
// Sources:
// - git log --since 2026-04-16 on branch gsd/quick/5-generate-a-presentation-based-on-what-i
// - docs/report/benchmarking/014.idxd_tonic_same_repo_claim_package.md
// - docs/report/architecture/004.bytes_async_memmove_contract.md
// - docs/report/api/006_async_memmove_inline_contract.md
// - docs/report/architecture/007.direct_tokio_async_memmove_implementation.md
// - docs/report/integration-review/004.idxd_ffi_consolidation_inventory.md
// - docs/report/m008/006.cleanup_conventions_and_integrated_proof.md
// - docs/report/architecture/015.hardware_rust_integrated_readability_evidence.md
// - docs/report/architecture/016.lean_bon_snafu_refactor.md
// - docs/report/hw_eval/011.m011_s03_representative_ops_2026-04-30.md
// - docs/report/benchmarking/015.m011_representative_idxd_numbers_2026-04-30.md
// - docs/report/architecture/017.generic_idxd_elegance_audit.md

#import "../template.typ": callout, card, deck, note, palette, panel, stage-card

#show: deck.with(
  margin: (x: 42pt, y: 30pt),
  size: 13.5pt,
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
  #text(size: 22pt, weight: "bold", fill: accent)[#value]
  #v(0.15em)
  #text(size: 10.2pt, fill: luma(65))[#body]
]

= Two-week progress update

#align(center + horizon)[
  #text(size: 18pt)[A cleaner Rust IDXD path, with a small hardware proof]
  #v(0.7em)
  #text(size: 15pt)[Hongtao Zhang]
  #v(0.25em)
  #text(size: 13pt, fill: luma(120))[Progress window: Apr 16--30, 2026]
]

#v(0.7em)

#callout(fill: c-blue, stroke: c-accent)[
  Small claim for this update: #text(weight: "bold")[the Rust IDXD path is now cleaner, easier to test, and has a small real-hardware proof]. This is #text(weight: "bold")[not yet] a claim that IDXD speeds up Tonic end to end.
]

== The short version

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Tonic result]
    #v(0.25em)
    + I cleaned up the Tonic evidence package.
    + The current data does not show an IDXD speedup.
    + A stronger Tonic result needs a fresh prepared-host rerun.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Rust IDXD work]
    #v(0.25em)
    + Moved async memmove to explicit owned buffers.
    + Added a direct Tokio completion path.
    + Replaced legacy crate names with `idxd-rust` and `idxd-sys`.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Hardware proof]
    #v(0.25em)
    + Added one shared session shape for DSA and IAX.
    + Ran DSA memmove and IAX crc64 on prepared hardware.
    + Collected a small release-mode measurement.
  ]],
)

#v(0.35em)

#note[
  The update is useful because it narrows the story. We now know what is proved, what is not proved, and what should be tested next.
]

== Work done during the two weeks

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Work volume]
    #v(0.35em)
    #grid(
      columns: (1fr, 1fr),
      gutter: 8pt,
      [#metric-card([Commits], [136], [non-merge commits since Apr 16], fill: white)],
      [#metric-card([Active days], [6], [Apr 24, 25, 27, 28, 29, 30], fill: white, accent: rgb("#16a34a"))],
      [#metric-card([Main crates], [3], [`idxd-rust`, `idxd-sys`, `hw-eval`], fill: white, accent: rgb("#f97316"))],
      [#metric-card([Deck claim], [small], [library progress plus limited hardware proof], fill: white, accent: rgb("#7c3aed"))],
    )
  ]],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Main phases]
    #v(0.3em)
    #stage-card(
      [Apr 24--27: clean up the evidence boundary],
      [Tonic evidence was rewritten around what the current package can actually show. Legacy package names were also consolidated.],
      [Make the story honest and easier to follow.],
      fill: white,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Apr 27--28: build the async path],
      [The async API now uses caller-owned buffers, and direct Tokio completion is the main path.],
      [Make async IDXD testable and observable.],
      fill: white,
      accent: rgb("#16a34a"),
    )
    #v(0.18em)
    #stage-card(
      [Apr 29--30: simplify and generalize],
      [The hardware Rust code was split by responsibility, then DSA and IAX were connected through one shared session pattern.],
      [Make the code easier to extend.],
      fill: white,
      accent: rgb("#f97316"),
    )
  ]],
)

== Tonic: the current result is negative or inconclusive

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Do not claim a Tonic speedup yet. The current package proves that the workflow exists, but the current rows do not show IDXD beating the ordinary software path.
]

#v(0.25em)

#table(
  columns: (0.8fr, 0.72fr, 1.48fr),
  inset: (x: 7pt, y: 5pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Part]],
  [#text(weight: "bold")[State]],
  [#text(weight: "bold")[Plain meaning]],

  [Software path],
  [#text(fill: rgb("#16a34a"), weight: "bold")[works]],
  [The ordinary Tonic workloads can be validated and packaged.],

  [IDXD path],
  [#text(fill: rgb("#2563eb"), weight: "bold")[gated]],
  [The IDXD verifier is in place, but a clean live rerun needs the prepared host to pass preflight.],

  [Claim package],
  [#text(fill: rgb("#16a34a"), weight: "bold")[stable]],
  [The workflow emits JSON, CSV, and markdown summaries that can be reviewed.],

  [Current numbers],
  [#text(fill: rgb("#dc2626"), weight: "bold")[no win]],
  [The current rows show IDXD at about `0.003x`--`0.653x` of software throughput.],
)

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [#card(
    [Small claim],
    [The Tonic measurement workflow is now clearer and easier to rerun. It does not currently support a positive acceleration claim.],
    fill: c-row,
    body-size: 10.8pt,
  )],
  [#card(
    [Next proof needed],
    [Run the IDXD side again on a prepared host, then rebuild the comparison package from fresh live artifacts.],
    fill: c-red,
    body-size: 10.8pt,
  )],
)

== Rust IDXD API: make ownership explicit

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Public API change]
    #v(0.35em)
    #card(
      [`AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)`],
      [The caller provides both buffers. The library does the memmove and returns the destination plus a validation report.],
      fill: white,
      body-size: 10.8pt,
    )
    #v(0.4em)
    #card(
      [Why this is simpler],
      [The API no longer hides destination allocation or copy-back behavior. The caller can see exactly what memory is submitted.],
      fill: white,
      body-size: 10.8pt,
    )
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Direct Tokio path]
    #v(0.35em)
    + Each accepted operation owns its descriptor, completion record, source, and destination until it finishes.
    + Completion records, not submit acceptance alone, decide when the future resolves.
    + Backpressure, retry count, completion status, and validation phase stay visible in errors.
    + Payload bytes are not printed in diagnostics.
  ]],
)

#v(0.35em)

#note[
  Software batching and alternate submit paths are still future work. They are not needed to explain the current API.
]

== Package cleanup: one safe crate, one raw crate

#grid(
  columns: (0.95fr, 1.05fr),
  column-gutter: 16pt,
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Before]
    #v(0.35em)
    + Safe code lived under `dsa-ffi`.
    + Raw bindings were named `idxd-bindings`.
    + Wrapper scripts made it hard to tell which path was current.
    + Downstream code still pointed at old names.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Now]
    #v(0.35em)
    + `idxd-rust` is the safe Rust and Tokio-facing crate.
    + `idxd-sys` is the raw UAPI and MMIO crate.
    + Compatibility wrappers point to the new scripts.
    + A package inventory check catches old active references.
  ]],
)

#v(0.4em)

#callout(fill: c-blue, stroke: c-accent)[
  This is not a performance result. It is a cleanup result: future work has fewer names to reason about and fewer stale entrypoints to trip over.
]

== Code quality: easier to navigate, less magic

#table(
  columns: (0.9fr, 1.08fr, 1.02fr),
  inset: (x: 7pt, y: 5pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Area]],
  [#text(weight: "bold")[What changed]],
  [#text(weight: "bold")[What did not change]],

  [Builders and errors],
  [Used builders and SNAFU errors only where they made config or diagnostics clearer.],
  [No builder was added around raw descriptors, request buffers, benchmark hot loops, or report records.],

  [Module split],
  [`idxd-rust`, `idxd-sys`, and `hw-eval` now have clearer owner modules and guard scripts.],
  [Public APIs, JSON fields, verifier output, and raw hardware behavior were kept stable.],

  [Raw boundary],
  [`idxd-sys` now separates descriptor, portal, completion, timing, topology, and cache helpers.],
  [Low-level facts such as OS errors, volatile status reads, and ENQCMD accepted/rejected results stay visible.],
)

#v(0.35em)

#callout(fill: c-blue, stroke: c-accent)[
  The practical benefit is maintenance: a future change should now have a clearer owner, a clearer test, and a smaller chance of duplicating lifecycle code.
]

== Generic IDXD session: shared shape for DSA and IAX

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What was added]
    #v(0.35em)
    + `IdxdSession<Dsa>` for DSA memmove.
    + `IdxdSession<Iax>` for IAX crc64.
    + One portal owner and one config shape.
    + Separate operation code for DSA and IAX details.
  ]],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[What is shared]
    #v(0.35em)
    + Reset and fill a descriptor.
    + Submit it to the work queue.
    + Watch the completion record.
    + Classify success, retry, or failure.
    + Return typed operation results.
  ]],
)

#v(0.35em)

#note[
  This is intentionally small. It does not try to cover every DSA or IAX operation yet.
]

== Small hardware proof

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
  This is a proof that the new path works on two representative operations. It is not a full benchmark study.
]

== What this means

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Safe to say now]
    #v(0.35em)
    + The Rust IDXD code is in a cleaner shape.
    + Async memmove has a clearer ownership model.
    + DSA memmove and IAX crc64 both ran through the new generic session path.
    + The small release-mode benchmark produced positive metrics for both rows.
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[Do not say yet]
    #v(0.35em)
    + IDXD speeds up Tonic end to end.
    + The benchmark results generalize across sizes or workloads.
    + The generic session covers the full DSA/IAX surface.
    + The code is ready for production scheduling or batching.
  ]],
)

#v(0.4em)

#callout(fill: c-blue, stroke: c-accent)[
  The small conclusion is enough: the library path is cleaner, the proof path is real, and the next Tonic claim needs a focused rerun.
]

== Next step

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#stage-card(
    [1. Rerun Tonic evidence],
    [Use a prepared host to refresh the IDXD side, then rebuild the ordinary-vs-IDXD comparison package.],
    [Best next step for an advisor update.],
    fill: c-row,
    accent: c-accent,
  )],
  [#stage-card(
    [2. Add one more operation],
    [Extend the generic session only when there is a real consumer and a verifier for the new operation.],
    [Best next step for library growth.],
    fill: c-row,
    accent: rgb("#16a34a"),
  )],
  [#stage-card(
    [3. Clean verifier helpers],
    [Extract shared launcher and artifact helpers only after one more verifier repeats the same pattern.],
    [Best next step if scripts become painful.],
    fill: c-row,
    accent: rgb("#f97316"),
  )],
)

#v(0.5em)

#note[
  Recommended next step: rerun the Tonic comparison on a prepared host, because that is the result closest to the original project question.
]
