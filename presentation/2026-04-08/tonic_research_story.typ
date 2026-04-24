// Research story deck: async abstractions, DSA, and Tonic profiling
// Sources:
// - presentation/2026-04-05/google_interview_research.typ
// - presentation/2026-04-08/tonic_flamegraph_analysis.typ
// - docs/report/benchmarking/006.stdexec_overhead_results.md
// - docs/report/hw_eval/010.dsa_hw_eval_smoke_numbers_2026-04-06.md
// - docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md
// - docs/report/architecture/002.tonic_component_analysis.md
// - docs/report/architecture/003.tonic_interception_points.md

#import "../template.typ": callout, card, deck, fit-badge, note, palette, panel, stage-card

#show: deck.with(
  margin: (x: 52pt, y: 42pt),
  leading: 0.88em,
  spacing: 0.95em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red
#let c-row = palette.row

= When Async Abstractions Meet Hardware Accelerators: A Profiling Story

#align(center + horizon)[
  #text(size: 18pt)[Thesis, measurement, and implication from DSA to Tonic]
  #v(0.8em)
  #text(size: 16pt)[Hongtao Zhang]
  #v(0.3em)
  #text(size: 14pt, fill: luma(120))[April 8, 2026]
]

#v(0.8em)

#callout(fill: c-blue, stroke: c-accent)[
  This deck follows one research arc: *a device-level thesis* → *a hardware lower bound* → *end-to-end RPC profiling* → *a concrete accelerator-facing crate plan*.
]

== 1. The Thesis

#callout(fill: c-blue, stroke: c-accent)[
  *Thesis*: accelerator programming wants an async abstraction, but once submission cost is amortized,
  the async control path itself can become more expensive than the hardware work.
]

#panel(fill: white)[
  #set text(size: 17pt)
  + Modern accelerators like DSA, IAX, and RDMA are *naturally asynchronous devices*.
  + So async frameworks are the right programming model.
  + But if batching makes device submission cheap, the *software control path* becomes visible.
  + The research question is simple: can today's async APIs carry very high-rate offload efficiently?
]

== 2. RDMA Lesson: Batching Makes Submission Cheap

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Doorbell cost],
    [At high message rates, one RDMA doorbell can cost about #fit-badge([~100--200 ns MMIO], fill: rgb("#f59e0b")) by itself.],
    fill: c-orange,
  )],
  [#card(
    [Batching lesson],
    [Batching amortizes that submission cost across many operations, so the hardware path stops being the obvious bottleneck.],
    fill: c-blue,
  )],
  [#card(
    [Research turn],
    [If DSA and RPC stacks show the same pattern, then software overhead becomes the next question — not the last detail.],
    fill: c-green,
  )],
)

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  *Intuition pump*: once batching makes submission cheap, we must ask whether the software stack can still keep up.
]

== 3. Hardware Lower Bound: DSA Proves the Thesis

#grid(
  columns: (1.18fr, 0.82fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[stdexec control-path removal]
    #v(0.4em)
    #table(
      columns: (1.7fr, 1fr, 1fr, 1fr),
      inset: (x: 7pt, y: 6pt),
      [#text(weight: "bold")[Strategy]],
      [#text(weight: "bold", size: 11pt)[c=2048 msg=8]],
      [#text(weight: "bold", size: 11pt)[Per-op]],
      [#text(weight: "bold", size: 11pt)[vs base]],
      [Full stdexec], [26.3 Mpps], [38.0 ns], [1.00x],
      [Direct (no scope.nest/then)], [41.6 Mpps], [24.0 ns], [1.58x],
      [Reusable ops], [59.9 Mpps], [16.7 ns], [2.28x],
    )
    #v(0.45em)
    #note[Hot-cache ceiling: *84 Mpps / 11.9 ns* at `c=32`.]
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Real DSA raw number]
    #v(0.45em)
    #card(
      [Peak pipelined memmove],
      [Intel DSA `memmove` reaches *48.4 Mops/s* at `64 B`, `batch=128`, `c=4`.],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Single-op latency],
      [A single `64 B` DSA `memmove` is *955 ns*. Batching is what exposes the fast regime.],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Takeaway],
      [The device can move data faster than the baseline software path can efficiently submit and compose it.],
      fill: white,
    )
  ]],
)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Key result*: removing framework layers cuts per-op cost from *38.0 ns* to *16.7 ns* — a *56% reduction* before touching the hardware path itself.
]

== 4. The Research Turn

#callout(fill: c-blue, stroke: c-accent)[
  The thesis is proven at the hardware interface level. Now: *does the same pattern show up in a full end-to-end RPC stack?*
]

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Why move to Tonic],
    [We need an end-to-end system, not just a microbenchmark at the device boundary.],
    fill: c-blue,
  )],
  [#card(
    [What Tonic includes],
    [A realistic stack: prost, codec buffers, optional compression, gRPC framing, HTTP/2, Tower, and tokio.],
    fill: c-orange,
  )],
  [#card(
    [What we need to learn],
    [If the same pattern appears, which software layers are actually worth replacing with accelerator-aware paths?],
    fill: c-green,
  )],
)

== 5. The Tonic Stack: Where the Work Can Hide

#grid(
  columns: (1.16fr, 0.84fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Send path with profile verdicts]
    #v(0.35em)
    #stage-card(
      [Prost encode / decode],
      [serialize and parse application messages],
      [negligible — 0.1%],
      fill: c-row,
      accent: rgb("#6b7280"),
    )
    #v(0.15em)
    #stage-card(
      [Codec / buffer layer],
      [BytesMut growth, frame assembly, payload movement],
      [dominant in medium/large — memmove up to 71%],
      fill: c-row,
      accent: rgb("#dc2626"),
    )
    #v(0.15em)
    #stage-card(
      [Optional compression],
      [DEFLATE plus extra transfer work],
      [CPU-active — 61.77% if enabled],
      fill: c-row,
      accent: rgb("#2563eb"),
    )
    #v(0.15em)
    #stage-card(
      [HPACK / HTTP/2],
      [headers, framing, stream state],
      [small — HPACK 1.7%],
      fill: c-row,
      accent: rgb("#6b7280"),
    )
    #v(0.15em)
    #stage-card(
      [Tokio / runtime],
      [scheduler, wakeups, connection task],
      [visible but secondary — H2 futex 2.8%],
      fill: c-row,
      accent: rgb("#f97316"),
    )
  ]],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Interception boundaries]
    #v(0.4em)
    #card(
      [Codec/body boundary],
      [The cleanest payload hook: encoded bytes, framing, compression, and buffer policy all meet here.],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Tower layers],
      [The right place for cross-cutting transforms like middleware-managed CRC or compression policy.],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Main implication],
      [The architecture already exposes the exact boundaries the profile says matter most.],
      fill: white,
    )
  ]],
)

== 6. Four Regimes, Four Different Bottlenecks

#table(
  columns: (1.04fr, auto, auto, auto, auto, 1.7fr, 1.1fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Regime]],
  [#text(weight: "bold", size: 11pt)[IPC]],
  [#text(weight: "bold", size: 11pt)[Frontend]],
  [#text(weight: "bold", size: 11pt)[Backend/Mem]],
  [#text(weight: "bold", size: 11pt)[Retiring]],
  [#text(weight: "bold", size: 11pt)[Top hotspots]],
  [#text(weight: "bold", size: 11pt)[Offload]],

  [#text(weight: "bold")[Small — 256 B] #linebreak() #text(size: 10pt, fill: luma(110))[single, no compress]],
  [#fit-badge("2.1", fill: rgb("#ca8a04"))],
  [#fit-badge("54%", fill: rgb("#dc2626"))],
  [#fit-badge("17%", fill: rgb("#2563eb"))],
  [#fit-badge("17%", fill: rgb("#2563eb"))],
  [#text(size: 10pt)[`memmove` 5.97% #linebreak() `h2::poll_complete` 2.19% #linebreak() `hpack::encode` 1.70% #linebreak() #text(fill: luma(130))[Prost decode 0.10%]]],
  [#fit-badge("LOW", fill: rgb("#6b7280"))],

  [#text(weight: "bold")[Medium — 4 KiB] #linebreak() #text(size: 10pt, fill: luma(110))[multi, no compress]],
  [#fit-badge("1.8", fill: rgb("#ca8a04"))],
  [#fit-badge("23%", fill: rgb("#f97316"))],
  [#fit-badge("41%", fill: rgb("#dc2626"))],
  [#fit-badge("30%", fill: rgb("#2563eb"))],
  [#text(size: 10pt)[`memmove` 10.51% #linebreak() `malloc` 8.67% #linebreak() `RawVec::finish_grow` 2.88% #linebreak() #text(fill: luma(130))[H2 futex 2.78%]]],
  [#fit-badge("MEDIUM", fill: rgb("#ca8a04"))],

  [#text(weight: "bold")[Large — 1 MiB] #linebreak() #text(size: 10pt, fill: luma(110))[multi, no compress]],
  [#fit-badge("0.3", fill: rgb("#dc2626"))],
  [#fit-badge("14%", fill: rgb("#2563eb"))],
  [#fit-badge("74%", fill: rgb("#7f1d1d"))],
  [#fit-badge("8%", fill: rgb("#2563eb"))],
  [#text(size: 10pt)[`memmove` 71.37% #linebreak() `RawVec::finish_grow` 5.49% #linebreak() `realloc` 5.45% #linebreak() #text(fill: luma(130))[IPC 0.3: movement-dominated]]],
  [#fit-badge("HIGH", fill: rgb("#16a34a"))],

  [#text(weight: "bold")[Large — 64 KiB] #linebreak() #text(size: 10pt, fill: luma(110))[multi, compress=on]],
  [#fit-badge("4.5", fill: rgb("#16a34a"))],
  [#fit-badge("8%", fill: rgb("#2563eb"))],
  [#fit-badge("20%", fill: rgb("#2563eb"))],
  [#fit-badge("69%", fill: rgb("#16a34a"))],
  [#text(size: 10pt)[`compress_inner` 61.77% #linebreak() `memmove` 8.84% #linebreak() `transfer` 7.43% #linebreak() #text(fill: luma(130))[CRC32 2.90%]]],
  [#fit-badge("HIGH*", fill: rgb("#2563eb"))],
)

#v(0.35em)
#note[IPC legend: `>3.5` healthy, `1.5–3` moderate, `<1` stalled. `HIGH*` means compression offload only matters if real compression hardware exists.]

== 7. Where Offloading Helps — And Where It Doesn't

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Offload targets]
    #v(0.35em)
    #card(
      [Large copy path],
      [At `1 MiB`, `memmove` is `71.37%` and IPC falls to `0.3`. *Why:* the CPU is stalled on movement, so DMA targets the real bottleneck.],
      fill: white,
    )
    #v(0.42em)
    #card(
      [Medium copy + allocation],
      [At `4 KiB`, `memmove + malloc + RawVec` is about `22%`, with `41%` backend pressure. *Why:* DMA plus steadier buffers can attack both copy work and growth churn.],
      fill: white,
    )
    #v(0.42em)
    #card(
      [Compression, if hardware exists],
      [In the compressed large regime, `compress_inner` is `61.77%` and IPC is `4.5`. *Why:* this is active codec work, so it needs a compression engine, not just DMA.],
      fill: white,
    )
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[Software-only wins]
    #v(0.35em)
    #card(
      [H2 stream locks],
      [`futex` contention is about synchronization and ownership. *Why not offload:* hardware cannot fix stream-lock policy or wakeup design.],
      fill: white,
    )
    #v(0.42em)
    #card(
      [HPACK],
      [`hpack::encode` is only `1.70%` in the small regime. *Why not offload:* it is too small and too control-heavy to matter.],
      fill: white,
    )
    #v(0.42em)
    #card(
      [Prost encode/decode],
      [Prost shows only `0.10%` decode in the small regime and never becomes a top hotspot. *Why not offload:* it is even less relevant than expected.],
      fill: white,
    )
  ]],
)

#v(0.45em)
#callout(fill: c-blue, stroke: c-accent)[
  *Critical bridge*: pooled buffers are not optional polish. They are a *prerequisite* for effective DMA because offload needs stable, pre-sized regions to move.
]

== 8. Architecture Validation: Confirmed vs. Revised

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Confirmed by the profile]
    #v(0.35em)
    #card(
      [Codec/body boundary is the right hook],
      [The hottest path is payload movement and buffer handling, exactly where codec/body hooks have leverage.],
      fill: white,
    )
    #v(0.4em)
    #card(
      [`dsa-ffi/` targets the right bottleneck],
      [Medium and large uncompressed regimes are copy-dominant, so DMA belongs in the plan.],
      fill: white,
    )
    #v(0.4em)
    #card(
      [`accel-middleware/` still fits],
      [Tower remains the right place for cross-cutting transforms and policy-managed compression or CRC layers.],
      fill: white,
    )
    #v(0.4em)
    #card(
      [`iax-ffi/` is conditional, not fantasy],
      [Compression is genuinely expensive when enabled, so accelerator compression remains valid if the payload is compressible.],
      fill: white,
    )
  ]],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Revised by the profile]
    #v(0.35em)
    #card(
      [Prost is less relevant than expected],
      [Not just "lower value" — it is essentially absent from the bottleneck picture at `0.10%`.],
      fill: white,
    )
    #v(0.4em)
    #card(
      [Buffer discipline comes first],
      [Pooled buffers and stable pre-sized regions are prerequisites, not optional cleanup around DMA.],
      fill: white,
    )
    #v(0.4em)
    #card(
      [Compression needs a gate],
      [Compression is disastrous on incompressible payloads — as low as `0.019x` throughput at `1 MiB` random — so it must be compressibility-aware.],
      fill: white,
    )
  ]],
)

#v(0.45em)
#panel(fill: c-blue)[
  #text(weight: "bold", fill: c-title)[Concrete next-step crate order]
  #v(0.3em)
  #text(size: 13pt)[
    1. `accel-codec/` first: pooled buffers + stable pre-sized regions → DMA preconditions #linebreak()
    2. `dsa-ffi/` second: DMA move for copy-dominant medium/large regimes #linebreak()
    3. `accel-middleware/` third: Tower layer with compressibility-aware gating #linebreak()
    4. `iax-ffi/` fourth: compression accelerator only when payloads are compressible
  ]
]

== 9. The Decision Rule

#align(center + horizon)[
  #callout(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 22pt, y: 18pt))[
    #text(size: 24pt, weight: "bold")[memmove dominates in medium/large regimes — DMA offload targets the right bottleneck.]
  ]
]

#v(0.8em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Do not touch Prost],
    [Encode/decode is negligible across the matrix.],
    fill: c-blue,
  )],
  [#card(
    [Fix buffers first],
    [Software buffer management is a prerequisite for effective offload.],
    fill: c-orange,
  )],
  [#card(
    [Gate compression],
    [Compression is a compressibility decision problem, not a default throughput win.],
    fill: c-red,
  )],
)

== 10. Summary and Open Questions

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [#card(
    [What we established],
    [Batching makes submission cheap enough that software overhead becomes visible. DSA already proved that at the device boundary.],
    fill: c-blue,
  )],
  [#card(
    [What the profile shows],
    [In Tonic, medium/large uncompressed runs are dominated by copy and buffer growth; compressed runs are conditional and codec-heavy.],
    fill: c-green,
  )],
  [#card(
    [What remains open],
    [Internal phase timers, pooled-buffer variant measurement, and streaming-mode profiling are still missing.],
    fill: c-orange,
  )],
)

#v(0.55em)
#callout(fill: c-blue, stroke: c-accent)[
  *Bottom line*: the thesis survived contact with both hardware and a full RPC stack. The next build step is not “accelerate everything” — it is *stabilize buffers, then offload the copy-dominant path that the profile actually exposed*.
]
