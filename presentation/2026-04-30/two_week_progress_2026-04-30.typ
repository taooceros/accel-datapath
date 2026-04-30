// Two-week progress presentation, 2026-04-30
// Reader: technical advisor / project collaborator reviewing the last two weeks.
// Post-read action: decide which claim is now supported, which claim remains blocked, and which next work item is worth funding.
// Sources:
// - git log --since 2026-04-16 on branch gsd/quick/5-generate-a-presentation-based-on-what-i
// - docs/report/benchmarking/014.idxd_tonic_same_repo_claim_package.md
// - docs/report/architecture/004.bytes_async_memmove_contract.md
// - docs/report/api/006_async_memmove_inline_contract.md
// - docs/report/architecture/007.direct_tokio_async_memmove_implementation.md
// - docs/report/architecture/009.direct_tokio_baseline_evidence.md
// - docs/report/integration-review/004.idxd_ffi_consolidation_inventory.md
// - docs/report/m008/006.cleanup_conventions_and_integrated_proof.md
// - docs/report/architecture/015.hardware_rust_integrated_readability_evidence.md
// - docs/report/architecture/016.lean_bon_snafu_refactor.md
// - docs/report/hw_eval/011.m011_s03_representative_ops_2026-04-30.md
// - docs/report/benchmarking/015.m011_representative_idxd_numbers_2026-04-30.md
// - docs/report/architecture/017.generic_idxd_elegance_audit.md

#import "../template.typ": callout, card, deck, fit-badge, note, palette, panel, stage-card

#show: deck.with(
  margin: (x: 42pt, y: 30pt),
  size: 13.2pt,
  leading: 0.82em,
  spacing: 0.62em,
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

= Two-week project progress

#align(center + horizon)[
  #text(size: 18pt)[From Tonic characterization to a generic IDXD proof seam]
  #v(0.7em)
  #text(size: 15pt)[Hongtao Zhang]
  #v(0.25em)
  #text(size: 13pt, fill: luma(120))[Progress window: Apr 16--30, 2026]
]

#v(0.7em)

#callout(fill: c-blue, stroke: c-accent)[
  The main shift was from "can we talk about accelerator benefit?" to a more disciplined answer: #text(weight: "bold")[keep Tonic claims narrow], make the Rust IDXD path observable and maintainable, then prove a small generic DSA/IAX seam on prepared hardware.
]

== Executive summary

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Claim discipline]
    #v(0.25em)
    + Rebuilt the Tonic comparison story around what the retained evidence actually proves.
    + Current ordinary-vs-IDXD package is a #text(weight: "bold")[workflow and falsification surface], not a fresh same-host acceleration win.
    + Stronger Tonic claims still require a prepared-host rerun.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Rust path made real]
    #v(0.25em)
    + Migrated async memmove to explicit `Bytes` / `BytesMut` ownership.
    + Implemented direct Tokio completion-record async submission.
    + Consolidated legacy `dsa-ffi` / `idxd-bindings` surfaces into `idxd-rust` + `idxd-sys`.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Generic IDXD seam]
    #v(0.25em)
    + Added `IdxdSession<Accel>` as the shared DSA/IAX session boundary.
    + Shared the blocking lifecycle through `run_blocking_operation`.
    + Collected hardware proof and small release-profile numbers for DSA memmove + IAX crc64.
  ]],
)

#v(0.35em)

#note[
  This deck deliberately separates three evidence classes: host-free contract proof, ordinary-host expected-failure classification, and prepared-host hardware proof.
]

== Timeline: what changed in the branch

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Recent work volume]
    #v(0.35em)
    #grid(
      columns: (1fr, 1fr),
      gutter: 8pt,
      [#metric-card([Commits], [136], [non-merge commits since Apr 16 on this branch], fill: white)],
      [#metric-card([Active days], [6], [Apr 24, 25, 27, 28, 29, 30], fill: white, accent: rgb("#16a34a"))],
      [#metric-card([Core crates], [3], [`idxd-rust`, `idxd-sys`, `hw-eval`], fill: white, accent: rgb("#f97316"))],
      [#metric-card([Proof style], [3 lanes], [host-free, expected-failure, prepared-host], fill: white, accent: rgb("#7c3aed"))],
    )
  ]],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Milestone arcs]
    #v(0.3em)
    #stage-card(
      [Apr 24--27: boundary cleanup],
      [Tonic claim package tightened; package surfaces consolidated from stale `dsa-ffi` / `idxd-bindings` names toward canonical `idxd-rust` + `idxd-sys`.],
      [Stop overclaiming; remove naming confusion.],
      fill: white,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Apr 27--28: async binding],
      [Owned-buffer async API, inline ENQCMD policy, direct Tokio monitor, and benchmark/verifier surfaces landed.],
      [Make async IDXD observable.],
      fill: white,
      accent: rgb("#16a34a"),
    )
    #v(0.18em)
    #stage-card(
      [Apr 29--30: maintainability + generic IDXD],
      [Readability split, lean `bon`/`snafu` convention, generic `IdxdSession<Accel>`, and representative DSA/IAX hardware proof.],
      [Prepare handoff-worthy architecture.],
      fill: white,
      accent: rgb("#f97316"),
    )
  ]],
)

== Tonic claim package: honest boundary first

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  The current Tonic comparison evidence rejects casual acceleration claims. It proves the retained software verifier, IDXD-path verifier, and comparison-package workflow; it does #text(weight: "bold")[not] prove a fresh prepared-host same-host IDXD win.
]

#v(0.25em)

#table(
  columns: (0.78fr, 0.72fr, 1.5fr),
  inset: (x: 7pt, y: 5pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Surface]],
  [#text(weight: "bold")[Status]],
  [#text(weight: "bold")[Meaning now]],

  [`S02 software`],
  [#text(fill: rgb("#16a34a"), weight: "bold")[retained]],
  [Validates curated ordinary Tonic workloads and stable artifact labels.],

  [`S03 IDXD`],
  [#text(fill: rgb("#16a34a"), weight: "bold")[visible gate]],
  [Prepared-host gate for accelerated artifacts; preflight blocks remain explicit instead of hidden.],

  [`S04 package`],
  [#text(fill: rgb("#2563eb"), weight: "bold")[stable report]],
  [Assembles software, IDXD, and control-floor evidence into CSV / JSON / markdown outputs.],

  [Current ratios],
  [#text(fill: rgb("#dc2626"), weight: "bold")[no win]],
  [Existing `latest/` rows show IDXD at about `0.003x`--`0.653x` software throughput across curated rows.],
)

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [#card(
    [What is safe to say],
    [The repository can validate the ordinary path, validate the IDXD path contract, and publish a stable comparison package. The current table is useful as a blocking or falsifying surface.],
    fill: c-row,
    body-size: 10.8pt,
  )],
  [#card(
    [What remains blocked],
    [A stronger Tonic acceleration claim needs a prepared-host S03 pass and an S04 package regenerated from fresh live accelerated artifacts, not fixture-backed mixed provenance.],
    fill: c-red,
    body-size: 10.8pt,
  )],
)

== Async IDXD path: owned buffers plus direct completion

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Public contract]
    #v(0.35em)
    #card(
      [`AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)`],
      [The caller owns both buffers. The source length is the transfer size; the destination is returned in `AsyncMemmoveResult` with the validation report.],
      fill: white,
      body-size: 10.6pt,
    )
    #v(0.4em)
    #card(
      [Rejected stale surfaces],
      [`copy_exact`, `copy_into`, source-only constructors, public borrowed copy-back helpers, and `result.bytes` are intentionally not the post-migration API.],
      fill: white,
      body-size: 10.4pt,
    )
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Runtime implementation]
    #v(0.35em)
    + Direct Tokio runtime owns descriptors, completion records, source, destination, retry metadata, and waiter state until terminal completion.
    + ENQCMD acceptance/rejection is bounded and typed; backpressure includes retry budget/count and completion snapshot metadata.
    + The monitor resolves futures from completion snapshots and preserves accepted operation ownership even if an awaiter is dropped.
    + Hardware claim evidence remains gated by verifier `verdict=pass claim_eligible=true`.
  ]],
)

#v(0.35em)

#note[
  Important design constraint: inline ENQCMD is allowed for v1; software aggregation, batching, and MOVDIR64 support are future optimization topics, not prerequisites.
]

== Canonical package shape: less legacy ambiguity

#grid(
  columns: (0.95fr, 1.05fr),
  column-gutter: 16pt,
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Before consolidation]
    #v(0.35em)
    + Active safe crate: `accel-rpc/dsa-ffi`.
    + Raw package name: `idxd-bindings` in `dsa-bindings/`.
    + Top-level `dsa-ffi/` wrapper scripts looked like another owner.
    + `tonic-profile` depended on the stale safe crate name.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[After the pass]
    #v(0.35em)
    + Canonical raw layer: `idxd-sys`.
    + Canonical safe/Tokio layer: `idxd-rust`.
    + Top-level compatibility wrappers dispatch to canonical scripts instead of defining ownership.
    + Package inventory verifier rejects active drift back to `dsa-ffi`, `idxd-bindings`, or `dsa-bindings`.
  ]],
)

#v(0.4em)

#callout(fill: c-blue, stroke: c-accent)[
  The cleanup matters because future Tonic or generic-IDXD work can now depend on one named safe layer and one named raw layer instead of rediscovering which legacy crate owns the proof surface.
]

== Maintainability work: cleanup without semantics drift

#table(
  columns: (0.9fr, 1.08fr, 1.02fr),
  inset: (x: 7pt, y: 5pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Track]],
  [#text(weight: "bold")[What improved]],
  [#text(weight: "bold")[Non-change boundary]],

  [`bon` / `snafu`],
  [Builders and SNAFU stayed where they clarify config or diagnostics: validation config, `hw-eval` config, sync/async/direct errors, and WQ-open context.],
  [No builder around request buffers, raw descriptors, proof-private CLI structs, report schemas, or hot-loop policy.],

  [Readability split],
  [`idxd-rust` direct async, `tokio_memmove_bench`, `hw-eval`, and `idxd-sys` now have clearer owner modules and guard scripts.],
  [No public API churn, schema version churn, raw unsafe hiding, or prepared-host claim from host-free proof.],

  [`idxd-sys` raw boundary],
  [Descriptor, portal, completion, timing, topology, and cache concerns are separated behind a lean facade.],
  [Raw ABI layout, `std::io::Result`, volatile status reads, and ENQCMD accepted/rejected signals stay visible.],
)

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [#card(
    [Why this was worth doing],
    [Most of the next risk is integration complexity, not one missing helper. Making owner boundaries visible reduces the chance that future work duplicates lifecycle code or overstates ordinary-host checks.],
    fill: c-row,
    body-size: 10.8pt,
  )],
  [#card(
    [Proof discipline preserved],
    [Reports and verifiers keep the no-payload rule: diagnostics may include paths, lengths, phases, statuses, retry counts, and timings, but not source/destination bytes or dumps.],
    fill: c-blue,
    body-size: 10.8pt,
  )],
)

== Generic IDXD architecture: one seam, representative operations

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Core architecture]
    #v(0.35em)
    + `IdxdSession<Accel>` owns one `WqPortal` and one typed config.
    + `Dsa`, `Iax`, and `Iaa` keep accelerator-family naming explicit.
    + `IdxdSession<Dsa>::memmove` and `IdxdSession<Iax>::crc64` are the representative public operations.
    + Static marker types avoid public `dyn` dispatch or a broad operation hierarchy.
  ]],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Shared lifecycle]
    #v(0.35em)
    + `run_blocking_operation` owns reset, fill, submit, observe, classify, retry, and return.
    + DSA and IAX adapters own descriptor/completion state and operation-specific classification.
    + `WqPortal::submit_desc64` owns the dedicated/shared 64-byte descriptor submission branch.
    + Proof and benchmark CLIs consume the API instead of bypassing it.
  ]],
)

#v(0.35em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  The elegance decision is intentionally scoped: no full DSA/IAX surface, no scheduler, no pooling, no batching framework, no RPC integration, and no benchmark matrix inside M011.
]

== Prepared-host proof and measured rows

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

  [S03],
  [`dsa-memmove`],
  [`/dev/dsa/wq0.0`],
  [`64`],
  [n/a],
  [`completed`],
  [`verdict=pass`],

  [S03],
  [`iax-crc64`],
  [`/dev/iax/wq1.0`],
  [`64`],
  [n/a],
  [`completed`],
  [`crc64_verified=true`],

  [S04],
  [`dsa-memmove`],
  [`/dev/dsa/wq0.0`],
  [`4096`],
  [`1000`],
  [`6,837 ns`],
  [`146,246 ops/s`],

  [S04],
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
  [#metric-card([Verifier], [`verdict=pass`], [S03 operation proof and S04 measured-number proof both passed], fill: c-green, accent: rgb("#16a34a"))],
  [#metric-card([Profile], [`release`], [S04 required release-profile measurements with `claim_eligible=true`], fill: c-blue)],
  [#metric-card([Failures], [`0 / 2000`], [S04 representative benchmark completed all required operations], fill: c-row, accent: rgb("#f97316"))],
)

#v(0.2em)

#note[
  These are representative proof rows, not a performance characterization: no size sweep, concurrency sweep, batching comparison, software baseline, or optional shared-DSA claim is included.
]

== What is now stronger than two weeks ago?

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Supported now]
    #v(0.35em)
    + The canonical Rust IDXD stack has a real safe layer and raw layer with host-free guards.
    + Direct Tokio async memmove has explicit buffer ownership, completion-driven futures, and typed failure metadata.
    + Generic DSA/IAX session architecture is not just sketched: representative DSA memmove and IAX crc64 completed on prepared hardware.
    + Release-profile representative rows prove the new seam can produce positive measured metrics.
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[Still not supported]
    #v(0.35em)
    + A Tonic end-to-end IDXD speedup claim.
    + Full DSA/IAX/IAA operation coverage.
    + A production scheduler, pooling layer, batching policy, or MOVDIR64 strategy.
    + Broad benchmark conclusions from the tiny representative S04 proof.
  ]],
)

#v(0.4em)

#callout(fill: c-blue, stroke: c-accent)[
  The progress is architectural credibility plus proof plumbing: the project can now make small hardware-backed claims honestly, while rejecting bigger claims until the matching evidence exists.
]

== Recommended next discussion

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#stage-card(
    [Option A — Tonic rerun],
    [Use the prepared-host path to refresh the IDXD subtree and rebuild the ordinary-vs-IDXD package.],
    [Best if the meeting needs an application-level claim.],
    fill: c-row,
    accent: c-accent,
  )],
  [#stage-card(
    [Option B — generic seam expansion],
    [Add the next DSA or IAX operation only when a concrete consumer and verifier exist.],
    [Best if architecture maturation is the goal.],
    fill: c-row,
    accent: rgb("#16a34a"),
  )],
  [#stage-card(
    [Option C — proof utility extraction],
    [Wait for one more peer verifier, then extract shared launcher/artifact/no-payload helpers.],
    [Best if verifier duplication starts slowing work.],
    fill: c-row,
    accent: rgb("#f97316"),
  )],
)

#v(0.5em)

#note[
  My recommendation: start with #text(weight: "bold")[Option A] if the advisor meeting is about the original Tonic motivation; choose #text(weight: "bold")[Option B] only if the next milestone is explicitly about the IDXD library surface.
]
