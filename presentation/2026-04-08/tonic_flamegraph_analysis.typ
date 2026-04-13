// Tonic gRPC — hotspot analysis framed for hardware offloadability
// Source: docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md
// Architecture: docs/report/architecture/002.tonic_component_analysis.md
//             docs/report/architecture/003.tonic_interception_points.md
// Perf data: results/tonic/2026-04-01-loop2/*_perf_report.txt, *_perf_stat.txt
//            results/tonic/2026-04-08-frameptr/*_perf_report.txt, *_perf_stat.txt
//            results/tonic/2026-04-08-frameptr-debuginfo/*_perf_report.txt, *_perf_stat.txt

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

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 1 — Title
// ══════════════════════════════════════════════════════════════════════════════

= Tonic gRPC: Where Does the Time Actually Go?

#align(center + horizon)[
  #text(size: 15pt)[
    Flamegraph-level hotspot analysis across 4 payload × runtime regimes \
    #v(0.5em)
    #text(size: 13pt, fill: luma(120))[
      Perf: cpu/cycles/Pu · representative rerun with frame pointers + client/server profiles · Xeon Gold 6438M \
      #text(weight: "bold")[
        Reports: ]#text(fill: c-accent)[`009.bounded_matrix_results`, `010.frame_pointer_rerun_client_server_results`, `011.debug_symbol_rerun_client_server_results`]
      ]
  ]
]

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 1b — Repo story context
// ══════════════════════════════════════════════════════════════════════════════

== The Repo Question: Where Can Hardware Offloading Help?

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[The Tonic stack]
    #v(0.35em)
    #stage-card(
      [Prost encode / decode],
      [serialize app data],
      [frameptr rerun: large `bytes` payloads surface as copy callers],
      fill: c-row,
      accent: rgb("#f97316"),
    )
    #v(0.15em)
    #stage-card(
      [Codec / buffer layer],
      [bytes → BytesMut → frame],
      [profile: body buffering + growth + memmove — dominant],
      fill: c-row,
      accent: rgb("#dc2626"),
    )
    #v(0.15em)
    #stage-card(
      [Compression / CRC],
      [DEFLATE + checksum],
      [profile: 61% CPU-active — good offload target],
      fill: c-row,
      accent: rgb("#ca8a04"),
    )
    #v(0.15em)
    #stage-card(
      [HPACK / HTTP/2 framing],
      [header encode + stream state],
      [profile: 1.7% — not the bottleneck],
      fill: c-row,
      accent: rgb("#6b7280"),
    )
    #v(0.15em)
    #stage-card(
      [Tokio / runtime],
      [scheduler + I/O],
      [profile: H2 futex 2.8% — sync overhead],
      fill: c-row,
      accent: rgb("#f97316"),
    )
  ]],
  [#panel(fill: c-orange, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[The intended crate split]
    #v(0.35em)
    #card(
      [`accel-codec/`],
      [Pooled buffers + zero-copy slices. Intercepts at Codec\/Encoder\/Decoder boundaries — the cleanest payload-path hook.],
      fill: white,
    )
    #v(0.35em)
    #card(
      [`dsa-ffi/`],
      [DSA bridge for copy + CRC. DMA move targets the copy-dominant medium/large regimes. Intel DSA handles movement directly.],
      fill: white,
    )
    #v(0.35em)
    #card(
      [`iax-ffi/`],
      [IAX bridge for compression. Targets DEFLATE-dominated regimes. IAX handles the codec work that CPU does actively.],
      fill: white,
    )
    #v(0.35em)
    #card(
      [`accel-middleware/`],
      [Tower CRC/compression layers. Composable with the codec boundary; exposes compression to Tower's layer model.],
      fill: white,
    )
  ]],
)

#v(0.4em)
#callout(fill: c-green, stroke: rgb("#16a34a"))[
  *Story so far:* RDMA lesson showed batching makes submission cheap — software overhead becomes visible. \
  Tonic profiling now tells us *which software overhead is worth replacing*: copy in medium/large, codec in compressed paths.
]

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 2 — 4-regime analysis table
// ══════════════════════════════════════════════════════════════════════════════

== 4 Regimes, 4 Different Bottlenecks

#table(
  columns: (1.05fr, auto, auto, auto, auto, 1.8fr, 1.2fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),

  [#text(weight: "bold")[Regime]],
  [#text(weight: "bold", size: 11pt)[IPC]],
  [#text(weight: "bold", size: 11pt)[Frontend]],
  [#text(weight: "bold", size: 11pt)[Backend\/Mem]],
  [#text(weight: "bold", size: 11pt)[Retiring]],
  [#text(weight: "bold", size: 11pt)[Top hotspots]],
  [#text(weight: "bold", size: 11pt)[Offload potential]],

  [
    #text(weight: "bold")[Small — 256 B] \
    #text(size: 10pt, fill: luma(110))[single-thread, compress=off]
  ],
  [#fit-badge("2.1", fill: rgb("#16a34a"))],
  [#fit-badge("54%", fill: rgb("#dc2626"))],
  [#fit-badge("17%", fill: rgb("#2563eb"))],
  [#fit-badge("17%", fill: rgb("#2563eb"))],
  [#text(size: 10.1pt)[
    `memmove` 5.97% \
    `h2::poll_complete` 2.19% \
    `hpack::encode` 1.70% \
    #text(size: 9.3pt, fill: luma(140))[Prost decode 0.10%]
  ]],
  [#panel(fill: c-row, inset: (x: 10pt, y: 8pt))[
    #fit-badge("LOW", fill: rgb("#6b7280"))
    #v(0.25em)
    #text(size: 9.6pt, fill: luma(85))[Fetch-limited CPU; copy is too small to justify hardware.]
  ]],

  [
    #text(weight: "bold")[Medium — 4 KiB] \
    #text(size: 10pt, fill: luma(110))[multi-thread, compress=off]
  ],
  [#fit-badge("1.8", fill: rgb("#ca8a04"))],
  [#fit-badge("23%", fill: rgb("#f97316"))],
  [#fit-badge("41%", fill: rgb("#dc2626"))],
  [#fit-badge("30%", fill: rgb("#2563eb"))],
  [#text(size: 10.1pt)[
    `memmove` 10.51% \
    `malloc` 8.67% \
    `RawVec::finish_grow` 2.88% \
    #text(size: 9.3pt, fill: luma(140))[H2 futex 2.78%]
  ]],
  [#panel(fill: c-orange, inset: (x: 10pt, y: 8pt))[
    #fit-badge("MEDIUM", fill: rgb("#ca8a04"))
    #v(0.25em)
    #text(size: 9.6pt, fill: luma(85))[Memory-bound. DMA copy + pooled buffers could reclaim stalled cycles.]
  ]],

  [
    #text(weight: "bold")[Large — 1 MiB] \
    #text(size: 10pt, fill: luma(110))[multi-thread, compress=off]
  ],
  [#fit-badge("0.3", fill: rgb("#dc2626"))],
  [#fit-badge("14%", fill: rgb("#2563eb"))],
  [#fit-badge("74%", fill: rgb("#7f1d1d"))],
  [#fit-badge("8%", fill: rgb("#2563eb"))],
  [#text(size: 10.1pt)[
    `memmove` self #text(weight: "bold", fill: rgb("#dc2626"))[71.37%] \
    frameptr rerun resolves callers: Prost encode 22.62%, decode 16.22%, body poll_frame 10.06% \
    #text(size: 9.3pt, fill: luma(140))[plus client harness copy 18.01%; still movement-dominated]
  ]],
  [#panel(fill: c-green, inset: (x: 10pt, y: 8pt))[
    #fit-badge("HIGH", fill: rgb("#16a34a"))
    #v(0.25em)
    #text(size: 9.6pt, fill: luma(85))[Best DMA target in the deck: offload the whole copy pipeline, not a single prost-only hotspot.]
  ]],

  [
    #text(weight: "bold")[Large — 64 KiB] \
    #text(size: 10pt, fill: luma(110))[multi-thread, compress=on]
  ],
  [#fit-badge("4.5", fill: rgb("#16a34a"))],
  [#fit-badge("8%", fill: rgb("#2563eb"))],
  [#fit-badge("20%", fill: rgb("#2563eb"))],
  [#fit-badge("69%", fill: rgb("#16a34a"))],
  [#text(size: 10.1pt)[
    `compress_inner` #text(weight: "bold", fill: rgb("#dc2626"))[61.77%] \
    `memmove` 8.84% \
    `transfer` 7.43% \
    #text(size: 9.3pt, fill: luma(140))[CRC32 2.90%]
  ]],
  [#panel(fill: c-blue, inset: (x: 10pt, y: 8pt))[
    #fit-badge("HIGH*", fill: rgb("#2563eb"))
    #v(0.25em)
    #text(size: 9.6pt, fill: luma(85))[
      CPU is active doing real DEFLATE work; offload only if compression hardware exists.
    ]
  ]],
)

#v(0.35em)
#grid(
  columns: (auto, auto, auto, auto),
  column-gutter: 20pt,
  [#fit-badge(">3.5", fill: rgb("#16a34a")) #text(size: 11pt)[healthy IPC]],
  [#fit-badge("1.5–3", fill: rgb("#ca8a04")) #text(size: 11pt)[moderate]],
  [#fit-badge("<1", fill: rgb("#dc2626")) #text(size: 11pt)[stalled]],
  [#fit-badge(">50%", fill: rgb("#7f1d1d")) #text(size: 11pt)[critical]],
)

#v(0.25em)
#note[
  `HIGH*` means the work is offloadable only with a real compression engine; unlike DMA copy, this is not idle CPU stall to reclaim.
]

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 3 — Caller-resolved attribution for large uncompressed lane
// ══════════════════════════════════════════════════════════════════════════════

== Large 1 MiB Uncompressed: Where the Copy Time Lands

#note[
  Debuginfo keeps the same bottleneck regime but makes the large-copy lane readable enough to map back to concrete repo files.
]

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-green, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[Client-side caller split]
    #v(0.35em)
    #card(
      [Prost encode],
      [
        `<tonic_prost::codec::ProstEncoder<T> as Encoder>::encode` \
        `→ __memmove` `22.62%` \
        #text(size: 9.1pt, fill: luma(135))[source: `accel-rpc/tonic/tonic-prost/src/codec.rs:97-101`] \
        #text(weight: "bold")[Meaning:] request serialization is a major bulk-copy caller.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Prost decode],
      [
        `<tonic_prost::codec::ProstDecoder<U> as Decoder>::decode` \
        `→ prost::encoding::bytes::merge → __memmove` `16.22%` \
        #text(size: 9.1pt, fill: luma(135))[source: `accel-rpc/tonic/tonic-prost/src/codec.rs:130-135`] \
        #text(weight: "bold")[Meaning:] large reply decode is also a major copy lane.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Tonic body buffering + harness],
      [
        `StreamingInner::poll_frame → __memmove` `10.06%` \
        `run_phase` worker closure direct `__memmove` `18.01%` \
        `realloc → __memmove` about `1.33%` \
        #text(size: 9.1pt, fill: luma(135))[
          body source: `accel-rpc/tonic/tonic/src/codec/decode.rs:129-133,280-282` \
          harness source: `accel-rpc/tonic-profile/src/main.rs:331-333`
        ] \
        #text(weight: "bold")[Meaning:] the benchmark harness itself contributes large request-copy cost on the client.
      ],
      fill: white,
    )
  ]],
  [#panel(fill: c-red, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[Server-side caller split]
    #v(0.35em)
    #card(
      [Prost decode],
      [
        `ProstDecoder::decode → bytes::merge → __memmove` `22.75%` \
        #text(size: 9.1pt, fill: luma(135))[source: `accel-rpc/tonic/tonic-prost/src/codec.rs:130-135`] \
        #text(weight: "bold")[Meaning:] request ingest on the server is also a large bulk-copy lane.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Tonic body buffering],
      [
        `StreamingInner::poll_frame → __memmove` `17.08%` \
        `OpaqueStreamRef::poll_data` still carries futex contention as secondary overhead. \
        #text(size: 9.1pt, fill: luma(135))[source: `accel-rpc/tonic/tonic/src/codec/decode.rs:129-133,280-282`] \
        #text(weight: "bold")[Meaning:] transport/body copies remain material even before service logic.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Prost encode],
      [
        `ProstEncoder::encode → __memmove` `13.37%` \
        #text(size: 9.1pt, fill: luma(135))[source: `accel-rpc/tonic/tonic-prost/src/codec.rs:97-101`] \
        #text(weight: "bold")[Meaning:] server replies pay the same large bytes-field copy cost on the way out.
      ],
      fill: white,
    )
  ]],
)

#v(0.55em)
#callout(fill: c-blue, stroke: c-accent)[
  *Correction from the frame-pointer rerun:* the large uncompressed regime is still the best DMA target, but the target is the #text(weight: "bold")[whole copy pipeline] — client harness copy + prost encode/decode + tonic body buffering + realloc — not a single prost-free `memmove` blob.
]

#note[
  `__memmove_avx512_unaligned_erms` is still the libc leaf. The useful source mapping comes from its Rust callers, which debuginfo makes much easier to present.
]

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 4 — Offload framing
// ══════════════════════════════════════════════════════════════════════════════

== Where Hardware Offloading Helps — And Where It Doesn't

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-green, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[Offload targets]
    #v(0.35em)
    #card(
      [Large payload copy pipeline],
      [
        `1 MiB` uncompressed remains the clearest DMA case: client IPC is `0.3`, server IPC is `0.5`, and the rerun resolves large copy lanes across prost encode, prost decode, body buffering, and harness copy. \
        #text(weight: "bold")[Why hardware helps:] the CPU is still stalled on movement, just now with caller-resolved evidence.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Medium payload copy + alloc],
      [
        At `4 KiB`, allocator and copy pressure remain the main client/server issue. \
        #text(weight: "bold")[Why hardware helps:] DMA copy plus steadier buffers can cut both movement and allocator churn.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Compression path, if HW exists],
      [
        `compress_inner` still dominates (`~65%` client, `~66%` server) in the compressed large regime. \
        #text(weight: "bold")[Why hardware helps:] this is genuine transform work for a compression engine; pure DMA is not enough.
      ],
      fill: white,
    )
  ]],
  [#panel(fill: c-red, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[Software-only wins]
    #v(0.35em)
    #card(
      [H2 stream locks],
      [
        `futex` / stream-state contention is still visible on the server in the large uncompressed lane. \
        #text(weight: "bold")[Hardware won't fix:] mutex design, queue ownership, or wakeup policy.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [HPACK and headers],
      [
        `hpack::encode` is still small in the tiny-payload regime. \
        #text(weight: "bold")[Hardware won't fix:] protocol bookkeeping that is not the dominant bottleneck.
      ],
      fill: white,
    )
    #v(0.45em)
    #card(
      [Harness artifacts and buffer discipline],
      [
        The client rerun shows an `18.01%` direct `run_phase` copy bucket. \
        Source-mapped to `accel-rpc/tonic-profile/src/main.rs:331-333`. \
        #text(weight: "bold")[Software must fix first:] remove per-request harness clone and pre-size buffers before making stronger offload claims.
      ],
      fill: white,
    )
  ]],
)

#v(0.55em)
#callout(fill: c-blue, stroke: c-accent)[
  *Refined rule:* do not say “prost is irrelevant.” Say “protobuf semantics are simple, but large `bytes` encode/decode still participate in the movement path that hardware and pooled buffers target.”
]

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 5 — Takeaway
// ══════════════════════════════════════════════════════════════════════════════

== The Decision Rule from This Profile

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  #text(size: 18pt, weight: "bold")[The large-message bottleneck is still movement — but now we know whose movement.] \
  #v(0.3em)
  Frame pointers resolve the hot copy lanes into client harness copy, prost encode/decode, tonic body buffering, and realloc. \
  Software buffer management is a prerequisite for effective offload.
]

#v(0.7em)

#grid(
  columns: (1fr, 1fr, 1fr),
  column-gutter: 14pt,
  [#card(
    [Do offload],
    [Whole copy pipelines in medium/large payloads, and real compression engines where compression is enabled.],
    fill: c-green,
  )],
  [#card(
    [Do in software],
    [Allocator discipline, buffer lifetime control, stream synchronization, and removing benchmark-harness copies.],
    fill: c-orange,
  )],
  [#card(
    [Do not overclaim],
    [Do not equate all `memmove` with prost encode, and do not call prost universally negligible without size/scope qualifiers.],
    fill: c-red,
  )],
)

// ══════════════════════════════════════════════════════════════════════════════
// SLIDE 5 — Implication: connects profiling to interception architecture
// ══════════════════════════════════════════════════════════════════════════════

== What This Profile Validates About the Architecture

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-green, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[Architecture claim — confirmed]
    #v(0.35em)
    #card(
      [`accel-codec/` is the right hook],
      [
        The profile shows copy and buffer growth dominate medium/large payloads. \
        The frame-pointer rerun makes this stronger: the codec\/Encoder\/Decoder boundary is exactly where caller-resolved prost encode/decode and body-buffer copies show up. \
        Intercepting here gives DMA a stable, pre-sized source and destination to work from.
      ],
      fill: white,
    )
    #v(0.4em)
    #card(
      [`dsa-ffi/` targets the right bottleneck],
      [
        Large payload IPC of `0.3` client-side and `0.5` server-side still means the CPU is waiting on memory traffic. \
        Intel DSA DMA move is the textbook fix for this pattern. \
        The rerun sharpens, rather than weakens, the case: now the movement path is caller-resolved.
      ],
      fill: white,
    )
    #v(0.4em)
    #card(
      [`iax-ffi/` is real but conditional],
      [
        Compression dominates (61%) only when compression is enabled. \
        IAX is the right architecture — but the payload must be compressible and the regime must have enough throughput headroom to care.
      ],
      fill: white,
    )
  ]],
  [#panel(fill: c-red, inset: (x: 16pt, y: 14pt))[
    #text(weight: "bold", fill: c-title)[Architecture claim — revised]
    #v(0.35em)
    #card(
      [Protobuf is even less relevant than expected],
      [
        The old slide overstated this point. Small-message protobuf logic is negligible, but large `bytes` payloads make prost encode/decode visible as copy callers. \
        `accel-codec/` still should not accelerate protobuf semantics; it should intercept the buffers that prost and tonic move through.
      ],
      fill: white,
    )
    #v(0.4em)
    #card(
      [Buffer discipline is prerequisite, not optional],
      [
        Report 003 focused on interception mechanics. \
        The profile adds urgency: pooled buffers and pre-sized allocations are not just a cleanliness optimization — \
        they expose the stable memory regions that DMA move actually needs.
      ],
      fill: white,
    )
    #v(0.4em)
    #card(
      [Compression needs a compressibility gate],
      [
        Report 002 recommended compression as a top candidate. \
        The profile shows it is actively harmful on incompressible payloads (0.02x throughput). \
        A compressibility check at the Tower layer (before calling IAX) would prevent the worst outcomes.
      ],
      fill: white,
    )
  ]],
)

#v(0.4em)
#callout(fill: c-blue, stroke: c-accent)[
  *Next concrete step:* start with `accel-codec/` — pooled buffers with stable pre-sized regions give DMA move its preconditions. Then layer `dsa-ffi/` for the copy path. Add `accel-middleware/` with compressibility-aware gating before `iax-ffi/`.
]
