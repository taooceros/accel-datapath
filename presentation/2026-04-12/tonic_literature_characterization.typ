// Tonic literature characterization deck
// Sources:
// - current.md
// - docs/plan/2026-04-12/03.tonic_literature_characterization_deck_plan.in_progress.md
// - docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md
// - docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md
// - docs/report/literature/008.paper_module_rebuild_analysis.md
// - docs/report/literature/papers/cloud-scale-characterization-of-remote-procedure-calls/README.md
// - docs/report/literature/papers/cloud-scale-characterization-of-remote-procedure-calls/paper.md
// - docs/related_work/04_rpc_acceleration_transports.md
// - docs/related_work/06_zero_copy_serialization_compression.md
// - docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md
// - docs/report/architecture/003.tonic_interception_points.md
// Style references:
// - presentation/template.typ
// - presentation/2026-04-08/tonic_research_story.typ
// - presentation/2026-04-08/tonic_flamegraph_analysis.typ

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

= Tonic Characterization Literature: Self-Study Walkthrough

#align(center + horizon)[
  #text(size: 18pt)[Analysis-driven rebuild of the literature deck's paper modules]
  #v(0.8em)
  #text(size: 16pt)[Hongtao Zhang]
  #v(0.3em)
  #text(size: 14pt, fill: luma(120))[April 13, 2026]
]

#v(0.8em)

#callout(fill: c-blue, stroke: c-accent)[
  This revision is for #text(weight: "bold")[offline learning]: each major paper now gets a #text(weight: "bold")[paper-first teaching module] with the paper's question, method or mechanism, evaluation setup, concrete findings, and only then the Tonic lesson.
]

== How to read this deck

#grid(
  columns: (0.94fr, 1.06fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Use the same lens on every paper]
    #v(0.35em)
    + What question is the paper trying to answer?
    + What mechanism or measurement method does it actually use?
    + What is the technically interesting trick, structure, or design choice?
    + What benchmark or comparison structure gives the numbers meaning?
    + What should Tonic measure differently because of that result?
  ]],
  [#panel(fill: white)[
    #card(
      [Six buckets to keep stable],
      [runtime / queueing, serialization / deserialization, copy / buffer lifecycle, compression / decompression, framing / transport glue, and wire / tail behavior],
      fill: c-row,
      body-size: 11.5pt,
    )
    #v(0.45em)
    #card(
      [How this differs from the older deck],
      [The older version synthesized the papers into Tonic lessons quickly. This one slows down and teaches what each paper is doing before using it.],
      fill: c-row,
      body-size: 11.3pt,
    )
    #v(0.45em)
    #card(
      [What counts as a useful lesson],
      [A paper matters here both because it helps Tonic and because it may teach a broader systems idea: a measurement strategy, a benchmark-construction trick, a hardware/software co-design, or a layout policy worth learning on its own.],
      fill: c-row,
      body-size: 10.9pt,
    )
  ]],
)

== What question should the literature answer?

#grid(
  columns: (0.94fr, 1.06fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[The repo-specific question]
    #v(0.35em)
    + serialization / deserialization
    + copies and buffer lifecycle
    + RPC stack processing and framing
    + queueing / scheduling / runtime policy
    + compression and host-side transforms
    + wire contribution and tail latency
  ]],
  [#panel(fill: white)[
    #card(
      [Why bucketing matters],
      [It prevents “RPC overhead” from collapsing payload work, stack work, queueing, and tail behavior into one average number.],
      fill: c-row,
      body-size: 11.4pt,
    )
    #v(0.45em)
    #card(
      [What literature can answer],
      [It can justify the decomposition vocabulary, show credible experiment design, and point to mechanisms worth isolating.],
      fill: c-row,
      body-size: 11.4pt,
    )
    #v(0.45em)
    #card(
      [What literature cannot answer],
      [It still cannot tell us which bucket dominates this repo's Tonic stack under each workload regime; only local measurement can do that.],
      fill: c-row,
      body-size: 11.2pt,
    )
  ]],
)

== Paper map: which paper helps with which bucket?

#table(
  columns: (1.08fr, 0.92fr, 1.28fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Bucket or design question]],
  [#text(weight: "bold", size: 11pt)[Strongest paper]],
  [#text(weight: "bold", size: 11pt)[What the reader should learn]],

  [#text(weight: "bold")[queueing / stack tax / tails]],
  [#text(size: 10.5pt)[Cloud-Scale RPC Characterization]],
  [#text(size: 10.5pt)[How a production measurement paper separates application work from RPC tax and keeps queueing, stack work, and wire time visible.]],

  [#text(weight: "bold")[serialization / deserialization]],
  [#text(size: 10.5pt)[Hardware Accelerator for Protocol Buffers]],
  [#text(size: 10.5pt)[Why protobuf behavior is rich enough to deserve its own benchmark, its own bucket, and its own accelerator story.]],

  [#text(weight: "bold")[gRPC measurement method]],
  [#text(size: 10.5pt)[TF-gRPC-Bench]],
  [#text(size: 10.5pt)[How to derive workload-shaped microbenchmarks that separate serialized payload cost, iovec shape, and transport.]],

  [#text(weight: "bold")[deployable acceleration]],
  [#text(size: 10.5pt)[RPCAcc]],
  [#text(size: 10.5pt)[Why acceleration wins only when serializer design, placement, and PCIe traversal are treated together.]],

  [#text(weight: "bold")[copy / layout policy]],
  [#text(size: 10.5pt)[Cornflakes]],
  [#text(size: 10.5pt)[Why copy policy and serialization layout belong together, with a hybrid copy vs scatter-gather decision instead of universal zero-copy.]],

  [#text(weight: "bold")[API-preserving transport replacement]],
  [#text(size: 10.5pt)[RR-Compound]],
  [#text(size: 10.5pt)[Why “keep the gRPC interface, replace the internal path” is a real design point, but only qualitatively grounded in this repo pass.]],
)

#v(0.35em)

#note[
  No single paper in this set gives an open tonic-like, stage-by-stage decomposition across serialization, copies, framing, scheduling, compression, and tail behavior.
]

== Cloud-Scale RPC Characterization: the decomposition to remember

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Why this paper anchors the deck]
    #v(0.35em)
    #card(
      [Paper type],
      [A measurement paper: it does not introduce a new RPC mechanism, it introduces a production-scale decomposition for where RPC completion time is spent.],
      fill: white,
      body-size: 11.4pt,
    )
    #v(0.4em)
    #card(
      [Core contribution],
      [The paper forces a first-order split of #text(weight: "bold")[application processing] from #text(weight: "bold")[RPC latency tax], then breaks the tax into queueing, RPC-stack, and wire terms.],
      fill: white,
      body-size: 10.9pt,
    )
  ]],
  [#panel(fill: white)[
    #stage-card(
      [Step 1 — split completion time],
      [The paper separates application execution from #text(weight: "bold")[non-application RPC work]. The resulting tax includes queuing, stack, and network wire terms.],
      [This prevents “just latency” summaries from hiding mechanism-level contributors.],
      fill: c-row,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Step 2 — open the RPC tax],
      [Tax is decomposed into request/response queues, request/response RPC processing + network stack work, and request/response wire delay along both directions.],
      [The measured model names the client/server queue and stack terms explicitly, not just “network delay.”],
      fill: c-row,
      accent: rgb("#f97316"),
    )
    #v(0.18em)
    #stage-card(
      [Step 3 — keep tails explicit],
      [The same component model is then used for heavy tails, nested RPC trees, and service-specific studies, so average tax and tail tax can diverge sharply.],
      [Decomposition is a tail and workload-skew problem too.],
      fill: c-row,
      accent: rgb("#16a34a"),
    )
  ]],
)

== Cloud-Scale RPC Characterization: how the study earns authority

#grid(
  columns: (0.98fr, 1.02fr),
  column-gutter: 16pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Methodology visual]
    #v(0.35em)
    #stage-card(
      [Monarch over 700 days],
      [Production metrics are sampled every 30 minutes (`700` days from Dec 2020 through Nov 2022), covering `10,000+` methods and service-level fleet behavior.],
      [Long-window fleet trend baseline],
      fill: white,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Dapper trace decomposition],
      [Per-RPC traces expose client/server/network components and nested RPC trees; the deep-dive trace pass covers one day with `722B` sampled RPCs, and per-method tail analysis keeps only methods with at least `100` samples so P99 is well-defined.],
      [Request, response, and nesting in one model],
      fill: white,
      accent: rgb("#16a34a"),
    )
    #v(0.18em)
    #stage-card(
      [GWP CPU profiling],
      [Daily sampled CPU profiles add RPC-cycle decomposition to latency work so completion time and processing cost are viewed together.],
      [Latency plus CPU-cycle tax],
      fill: white,
      accent: rgb("#f97316"),
    )
  ]],
  [#panel(fill: c-blue)[
    #card(
      [Study scope from the paper text],
      [The paper covers `10,000+` methods, `1B+` traces, `100s` of clusters, and a `700`-day production window spanning internal Google services built mostly on Stubby with some gRPC.],
      fill: white,
      body-size: 11.1pt,
    )
    #v(0.4em)
    #card(
      [What the study can compare],
      [Method and service distributions, cluster variation, request/response size distributions, call-tree shape (ancestors/descendants), latency components, and CPU-cycle tax are compared in one framework.],
      fill: white,
      body-size: 11.0pt,
    )
    #v(0.4em)
    #card(
      [Why this matters],
      [This is why the decomposition is believable: metrics, traces, and cycle profiles cross-check each other before the authors make method-level claims.],
      fill: white,
      body-size: 11.0pt,
    )
  ]],
)

== Cloud-Scale RPC Characterization: findings, limits, and Tonic lesson

#table(
  columns: (0.92fr, 1.02fr, 1.06fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Study context]],
  [#text(weight: "bold")[Reported finding]],
  [#text(weight: "bold")[Why it matters]],

  [#text(size: 10.2pt)[fleet-scale growth trend]],
  [#text(size: 10.2pt)[RPC throughput per CPU cycle rises by about `64%` over `700` days (`~30%` annual growth), using metrics from nearly 23 months of observation.]],
  [#text(size: 10.2pt)[RPC load is rising faster than CPU efficiency, so characterization has to handle both demand growth and changing mix.]],

  [#text(size: 10.2pt)[method-level latency variation]],
  [#text(size: 10.2pt)[`90%` of services have median latency `>= 10.7 ms`; `50%` of methods have `P99 >= 225 ms`; `99.5%` have `P99 >= 1 ms`; the slowest `5%` have `P1=166 ms` and `P99 >= 5 s`.]],
  [#text(size: 10.2pt)[Hyperscale RPC latency is heavily non-microsecond, so Tonic characterization should always include tail behavior and regime separation.]],

  [#text(size: 10.2pt)[service/traffic concentration]],
  [#text(size: 10.2pt)[Top-`8` services already dominate volume patterns; storage-heavy services like Network Disk can drive many calls and bytes while being relatively cycle-light per RPC, and ML/F1 consume more cycles with far fewer calls.]],
  [#text(size: 10.2pt)[This motivates service-specific optimization and why “average by method” is too coarse for planning optimization effort.]],

  [#text(size: 10.2pt)[popularity versus total time]],
  [#text(size: 10.2pt)[The top `100` lowest-latency methods are `40%` of calls, but the slowest `1000` methods are only `1.1%` of calls and still consume `89%` of total RPC time.]],
  [#text(size: 10.2pt)[You may need to target infrequent-but-expensive methods because they dominate time, not just frequency.]],

  [#text(size: 10.2pt)[fleet-average tax versus high-overhead tails]],
  [#text(size: 10.2pt)[Overall average tax = `2.0%`; median method tax ratio = `8.6%`; top `10%` overhead methods have `38%` median tax and `P90 = 96%`; network wire and RPC-processing-plus-network-stack together are major tail contributors.]],
  [#text(size: 10.2pt)[A single low average does not imply low tail risk: tax dominance is concentrated and often tail-driven.]],

  [#text(size: 10.2pt)[fleet-wide RPC cycle tax]],
  [#text(size: 10.2pt)[RPC work consumes about `7.1%` of all fleet CPU cycles; compression is `3.1%`, networking `1.7%`, and serialization `1.2%` of total cycles.]],
  [#text(size: 10.2pt)[The paper does not just decompose latency. It also shows why compression, networking, and serialization should remain separately visible in CPU-cost attribution.]],
)

#v(0.45em)

#note[
  The paper-text lesson is a #text(weight: "bold")[pattern of concentration]: average tax is modest, but service/method skew plus tail-heavy methods make the tax operationally critical.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Limitation],
    [Internal Google RPC stacks (mostly Stubby/gRPC-like) and one hyperscaler context give strong directional evidence, but not direct implementation-level hook equivalence for Tonic.],
    fill: c-orange,
    body-size: 10.8pt,
  )],
  [#card(
    [Tonic lesson],
    [Keep runtime, queueing, stack work, wire contribution, and tails visible by design. Averages and p99 are useful, but neither should be used alone for optimization decisions.],
    fill: c-green,
    body-size: 10.8pt,
  )],
)

== A Hardware Accelerator for Protocol Buffers: why protobuf gets its own bucket

#grid(
  columns: (0.94fr, 1.06fr),
  column-gutter: 16pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What question this paper asks]
    #v(0.35em)
    #card(
      [Core claim],
      [Protocol buffer serialization and deserialization are rich enough and costly enough to deserve dedicated study instead of being buried inside generic RPC cost.],
      fill: white,
      body-size: 11.2pt,
    )
    #v(0.4em)
    #card(
      [Why this matters for the deck],
      [The interesting systems lesson is not just “hardware wins.” The paper first explains why protobuf is a serious workload class, then makes a non-obvious placement choice for the accelerator.],
      fill: white,
      body-size: 10.9pt,
    )
  ]],
  [#panel(fill: white)[
    #card(
      [What the module must teach],
      [protobuf behavior is diverse; `HyperProtoBench` exists because message structure and field mix matter; the accelerator is intentionally #text(weight: "bold")[near-core and instruction-dispatched], not a NIC offload, because fleet data says much protobuf work is not RPC-bound],
      fill: c-row,
      body-size: 10.6pt,
    )
    #v(0.45em)
    #card(
      [What not to over-claim],
      [A protobuf accelerator result does not automatically explain runtime, framing, queueing, or buffer-movement cost in Tonic.],
      fill: c-orange,
      body-size: 11.1pt,
    )
    #v(0.45em)
    #card(
      [Best use in this repo],
      [Use it to justify keeping prost encode/decode visible as a separate bucket and to justify workload-derived serialization benchmarks before optimization claims.],
      fill: c-blue,
      body-size: 11.0pt,
    )
  ]],
)

== A Hardware Accelerator for Protocol Buffers: workflow and accelerator path

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Methodology to mechanism]
    #v(0.35em)
    #stage-card(
      [Step 1 — characterize protobuf usage],
      [The paper profiles protobuf behavior at Google scale so the benchmark is derived from observed service-side usage rather than a toy message.],
      [Profiling comes first],
      fill: white,
      accent: rgb("#16a34a"),
    )
    #v(0.18em)
    #stage-card(
      [Step 2 — derive `HyperProtoBench`],
      [That profiling becomes six synthetic benchmarks intended to reflect real workload diversity and field mix.],
      [Benchmark design is part of the contribution],
      fill: white,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Step 3 — choose a near-core architecture],
      [The accelerator is integrated into a Linux-capable RISC-V SoC, receives custom RoCC instructions directly from the BOOM core, and shares memory through the coherent TileLink path instead of sitting behind PCIe.],
      [The architecture choice is itself a major lesson],
      fill: white,
      accent: rgb("#f97316"),
    )
    #v(0.18em)
    #stage-card(
      [Step 4 — reduce software bookkeeping overheads],
      [The protobuf compiler generates one per-message-type Accelerator Descriptor Table (ADT), the design uses sparse hasbits for field presence, and the serializer walks fields in reverse order so sub-message lengths can be filled cleanly while staying wire-compatible.],
      [Interesting microarchitecture, not just a benchmark score],
      fill: white,
      accent: rgb("#dc2626"),
    )
  ]],
  [#panel(fill: c-blue)[
    #card(
      [Evaluation setup],
      [Six synthetic `HyperProtoBench` workloads, an RTL accelerator inside a RISC-V SoC, FireSim-based Linux evaluation, and comparison against BOOM-based and Xeon-based software contexts.],
      fill: white,
      body-size: 10.8pt,
    )
    #v(0.4em)
    #card(
      [Why the chip design is interesting],
      [Their fleet study argues against a simple NIC-placement story: only a minority of protobuf cycles are RPC-related, so the design favors low-latency instruction-level dispatch near the CPU instead of a PCIe-attached offload path.],
      fill: white,
      body-size: 10.6pt,
    )
    #v(0.4em)
    #card(
      [Why the workflow matters to Tonic],
      [This paper shows the profiling -> benchmark -> prototype chain that a serious accelerator argument should follow.],
      fill: white,
      body-size: 11.0pt,
    )
  ]],
)

== A Hardware Accelerator for Protocol Buffers: results, limits, and Tonic lesson

#table(
  columns: (0.92fr, 1fr, 1.02fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Evaluation context]],
  [#text(weight: "bold")[Reported result]],
  [#text(weight: "bold")[Why it matters]],

  [#text(size: 10.4pt)[benchmark basis]],
  [#text(size: 10.4pt)[Six synthetic `HyperProtoBench` workloads are derived from Google service-side protobuf behavior.]],
  [#text(size: 10.4pt)[The benchmark itself is evidence that protobuf behavior is diverse enough to characterize, not just accelerate.]],

  [#text(size: 10.4pt)[BOOM-based baseline SoC]],
  [#text(size: 10.4pt)[Average `6.2x–11.2x` speedup over the baseline SoC.]],
  [#text(size: 10.4pt)[The accelerator is not a marginal optimization in the near-CPU SoC setting used by the paper.]],

  [#text(size: 10.4pt)[Xeon-based server comparison]],
  [#text(size: 10.4pt)[Average `3.8x` speedup over a Xeon-based server.]],
  [#text(size: 10.4pt)[The result stays meaningful even when the comparison is not limited to the BOOM baseline.]],

  [#text(size: 10.4pt)[prototype form]],
  [#text(size: 10.4pt)[The prototype is a wire-compatible RTL accelerator integrated into a RISC-V SoC with custom-instruction dispatch, not a PCIe SmartNIC design.]],
  [#text(size: 10.4pt)[That broader design choice is what makes the paper memorable: it links characterization, benchmark design, and a concrete near-core architecture while preserving the wire format.]],
)

#v(0.45em)

#note[
  Read these as #text(weight: "bold")[protobuf-path comparisons], not as end-to-end gRPC transport numbers. The broader lesson is architectural too: this is a #text(weight: "bold")[near-core, instruction-dispatched] accelerator, very different from a PCIe- or NIC-only offload model.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Limitation],
    [This is not a gRPC end-to-end paper and not a deployable NIC-attached design. It does not answer runtime, queueing, or HTTP/2 framing questions.],
    fill: c-orange,
    body-size: 11.0pt,
  )],
  [#card(
    [Tonic lesson],
    [Keep prost encode/decode visible as a first-class bucket. Workload characterization should come before any serialization optimization story.],
    fill: c-green,
    body-size: 11.1pt,
  )],
)

== TF-gRPC-Bench: from TensorFlow traffic to benchmark classes

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Why the paper exists]
    #v(0.35em)
    #card(
      [Problem],
      [Full TensorFlow training runs are a noisy way to study the gRPC communication path if the real question is where communication cost comes from.],
      fill: white,
      body-size: 11.3pt,
    )
    #v(0.4em)
    #card(
      [Paper type],
      [A methodology paper: the main contribution is how it turns real traffic into controlled gRPC experiments.],
      fill: white,
      body-size: 11.2pt,
    )
  ]],
  [#panel(fill: white)[
    #stage-card(
      [Step 1 — observe TensorFlow-over-gRPC traffic],
      [The starting point is traffic analysis from popular TensorFlow models, not arbitrary benchmark inputs.],
      [Real workload structure first],
      fill: c-row,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Step 2 — derive three benchmark classes],
      [The communication patterns become point-to-point latency, point-to-point bandwidth, and parameter-server throughput microbenchmarks.],
      [Three ways to ask the question],
      fill: c-row,
      accent: rgb("#f97316"),
    )
    #v(0.18em)
    #stage-card(
      [Step 3 — keep serialization visible],
      [Serialized and non-serialized modes are both used so transport-side behavior can be distinguished from serialization overhead.],
      [Transport isolation is deliberate],
      fill: c-row,
      accent: rgb("#16a34a"),
    )
    #v(0.18em)
    #stage-card(
      [Step 4 — expose payload shape as a knob],
      [The benchmark does not treat payload construction as fixed: it lets users vary iovec count, buffer-size classes, and uniform/random/skewed payload generation so communication structure stays visible.],
      [Workload generation is part of the mechanism],
      fill: c-row,
      accent: rgb("#dc2626"),
    )
  ]],
)

== TF-gRPC-Bench: experiment matrix and controls

#table(
  columns: (0.95fr, 1fr, 1.15fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Controlled dimension]],
  [#text(weight: "bold")[Values used]],
  [#text(weight: "bold")[What it isolates]],

  [#text(size: 10.7pt)[benchmark class]],
  [#text(size: 10.7pt)[point-to-point latency, point-to-point bandwidth, parameter-server throughput]],
  [#text(size: 10.7pt)[Which communication behavior matters for the workload: latency, bulk transfer, or update throughput.]],

  [#text(size: 10.7pt)[serialization mode]],
  [#text(size: 10.7pt)[serialized vs non-serialized]],
  [#text(size: 10.7pt)[How much of the observed effect belongs to payload formatting rather than the underlying transport path.]],

  [#text(size: 10.7pt)[payload / iovec shape]],
  [#text(size: 10.7pt)[uniform / random / skewed iovec distributions, configurable buffer count and size]],
  [#text(size: 10.7pt)[Whether message layout and buffer distribution change the measured transport behavior; this is one of the paper's best benchmark-design ideas.]],

  [#text(size: 10.7pt)[transport]],
  [#text(size: 10.7pt)[40G Ethernet, 10G Ethernet, IPoIB, RDMA]],
  [#text(size: 10.7pt)[How the same communication pattern behaves when the underlying path changes but the workload shape stays matched.]],
)

#v(0.35em)

#note[
  This is why TF-gRPC-Bench matters so much to this repo: matched controls are part of the result, not just setup trivia. The broader lesson is benchmark craftsmanship: they model the parameter-server pattern and even use gRPC core C APIs in non-serialized mode to strip away serializer noise.
]

== TF-gRPC-Bench: reported findings and what to copy into Tonic

#table(
  columns: (0.95fr, 1fr, 1.03fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Benchmark setting]],
  [#text(weight: "bold")[Reported result]],
  [#text(weight: "bold")[Why this comparison matters]],

  [#text(size: 10.2pt)[`64 KB` serialized payloads]],
  [#text(size: 10.2pt)[RDMA cuts point-to-point latency by about `40%` versus 40G Ethernet and IPoIB in the reported setup.]],
  [#text(size: 10.2pt)[With serialization still in the path, transport gains are visible but moderated by payload-formatting cost.]],

  [#text(size: 10.2pt)[non-serialized skewed payloads on one cluster]],
  [#text(size: 10.2pt)[RDMA cuts latency by about `59%` versus Ethernet and `56%` versus IPoIB.]],
  [#text(size: 10.2pt)[Removing serialization overhead exposes a larger transport delta under the same skewed payload shape.]],

  [#text(size: 10.2pt)[non-serialized skewed payloads on another cluster]],
  [#text(size: 10.2pt)[RDMA cuts latency by about `78%` versus 10G Ethernet and `69%` versus IPoIB.]],
  [#text(size: 10.2pt)[The magnitude changes with the cluster and link context, which is why the experiment matrix has to stay explicit.]],

  [#text(size: 10.2pt)[parameter-server throughput]],
  [#text(size: 10.2pt)[Throughput improves by roughly `4.1x` versus 40G Ethernet, `3.43x` versus IPoIB, and `5.9x` versus 10G Ethernet in the reported settings.]],
  [#text(size: 10.2pt)[The workload-shaped throughput case shows that the transport choice matters differently from the point-to-point latency cases.]],
)

#v(0.45em)

#note[
  The serialized vs non-serialized split is what makes the transport result interpretable: the bigger wins appear once payload-formatting cost is controlled separately. The suite also includes a bandwidth class; this deck emphasizes latency and parameter-server cases because the local notes preserve those comparisons most clearly, but the mechanism is the full three-class matrix.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Limitation],
    [The traffic model is TensorFlow-specific and the transport context is 2018-era; this is a methodology model, not a direct Tonic headline source.],
    fill: c-orange,
    body-size: 11.0pt,
  )],
  [#card(
    [Tonic lesson],
    [Build workload-shaped, matched microbenchmarks. Separate payload formatting from transport choice and keep payload shape explicit instead of treating it as background noise.],
    fill: c-green,
    body-size: 11.0pt,
  )],
)

== RPCAcc: deployment problem and bottleneck model

#grid(
  columns: (0.95fr, 1.05fr),
  column-gutter: 16pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[The problem the paper is solving]
    #v(0.35em)
    #card(
      [Naive offload is not enough],
      [A PCIe-attached accelerator can lose much of its theoretical benefit if serializer behavior and placement are still movement-heavy.],
      fill: white,
      body-size: 11.2pt,
    )
    #v(0.4em)
    #card(
      [Why the paper argues this matters],
      [The repo's source trail records Google fleet motivation that RPC processing occupies about `7.1%` of CPU cycles.],
      fill: white,
      body-size: 11.2pt,
    )
  ]],
  [#panel(fill: c-blue)[
    #card(
      [Bottleneck model],
      [serialization cost, PCIe traversal, receive-side placement, and repeated field updates all interact; none of them can be treated as a side detail],
      fill: white,
      body-size: 11.0pt,
    )
    #v(0.4em)
    #card(
      [Why this belongs in the deck],
      [It is the clearest modern paper in the repo set showing that deployable RPC acceleration is a constrained co-design problem, not a generic “put it on hardware” move.],
      fill: white,
      body-size: 11.0pt,
    )
    #v(0.4em)
    #card(
      [What the module must teach],
      [What PCIe breaks, what the three named techniques actually do, and why each reported number has to stay attached to its workload context.],
      fill: white,
      body-size: 10.9pt,
    )
  ]],
)

== RPCAcc: host-to-accelerator datapath and named techniques

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Mechanism visual]
    #v(0.35em)
    #stage-card(
      [Step 1 — memory-affinity collaborative serializer],
      [RPCAcc adds a host pre-serialization phase that copies only CPU-resident fields into a contiguous DMA-safe buffer, can offload large copies to CPU memcpy engines, and leaves varint encoding to hardware so it does not pointer-chase nested CPU objects over PCIe.],
      [Avoid slow pointer-chasing reads across PCIe],
      fill: white,
      accent: rgb("#f97316"),
    )
    #v(0.18em)
    #stage-card(
      [Step 2 — PCIe-attached on-NIC accelerator],
      [Programmable compute units and the RoCE-based path live in a deployable PCIe-attached design rather than an idealized on-die accelerator.],
      [Deployment realism is the point],
      fill: white,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Step 3 — target-aware deserializer],
      [The receive path uses a schema table plus a 4 KB temp buffer so fields can be batched in SRAM and flushed with one-shot DMA writes only when needed, instead of paying a PCIe transaction per field.],
      [Avoid per-field PCIe writes on receive],
      fill: white,
      accent: rgb("#16a34a"),
    )
    #v(0.18em)
    #stage-card(
      [Step 4 — automatic field updates],
      [When kernels move between CPU and accelerator, RPCAcc updates schema placement metadata automatically so later requests deserialize fields into the right memory without manual re-tagging.],
      [Avoid stale placement causing extra traversals],
      fill: white,
      accent: rgb("#dc2626"),
    )
  ]],
  [#panel(fill: c-row)[
    #card(
      [Evaluation contexts],
      [HyperProtoBench, end-to-end cloud workload experiments, and an image-compression latency case on the FPGA prototype path.],
      fill: white,
      body-size: 11.0pt,
    )
    #v(0.4em)
    #card(
      [Why the datapath view matters],
      [The paper is relevant to this repo because it ties host serialization, PCIe movement, accelerator work, and receive-side layout into one story.],
      fill: white,
      body-size: 11.0pt,
    )
    #v(0.4em)
    #card(
      [What to remember],
      [Acceleration helps only when traversal-aware design turns movement cost into something the accelerator can actually avoid; the memorable tricks are one-shot DMA write, CPU pre-serialization, and dynamic schema placement.],
      fill: white,
      body-size: 10.7pt,
    )
  ]],
)

== RPCAcc: results, limits, and Tonic lesson

#table(
  columns: (0.9fr, 0.98fr, 1.05fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Evaluation context]],
  [#text(weight: "bold")[Reported result]],
  [#text(weight: "bold")[Why it matters]],

  [#text(size: 10.2pt)[motivating fleet picture]],
  [#text(size: 10.2pt)[RPC processing occupies about `7.1%` of CPU cycles in the motivating fleet picture carried into the analysis artifact.]],
  [#text(size: 10.2pt)[The CPU slice is large enough to motivate acceleration, but it does not by itself say how to accelerate safely.]],

  [#text(size: 10.2pt)[HyperProtoBench]],
  [#text(size: 10.2pt)[About `2.3x` lower RPC-layer processing time than a comparable accelerator baseline.]],
  [#text(size: 10.2pt)[The controlled benchmark result says the mechanism beats a more naive accelerator design, not just pure software.]],

  [#text(size: 10.2pt)[end-to-end cloud workloads]],
  [#text(size: 10.2pt)[Throughput improves by about `2.6x` in the reported settings.]],
  [#text(size: 10.2pt)[The benefit survives outside the benchmark harness, so the paper is not only a synthetic microbench story.]],

  [#text(size: 10.2pt)[image-compression case]],
  [#text(size: 10.2pt)[Average latency improves by about `2.6x`, p99 by about `1.9x`, and serialization time by about `4.3x` geometric mean in the accessible analysis trail.]],
  [#text(size: 10.2pt)[Latency, tail, and serialization improve by different amounts, which is exactly why one top-line multiplier is not enough.]],
)

#v(0.45em)

#note[
  RPCAcc is most useful when the results stay attached to #text(weight: "bold")[which workload and which metric] they come from; the paper does not collapse them into one universal win. It is also a strong lesson in #text(weight: "bold")[PCIe-aware co-design]: treat transaction rate, pointer chasing, and placement as first-class constraints.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Limitation],
    [This is a 2024 arXiv FPGA prototype, not a deployed tonic-specific production stack. The paper does not answer Rust async scheduling or Tonic framing questions directly.],
    fill: c-orange,
    body-size: 10.9pt,
  )],
  [#card(
    [Tonic lesson],
    [Treat serialization, placement, and interconnect traversal as a coupled design space. A single throughput headline is not enough evidence for later offload claims.],
    fill: c-green,
    body-size: 11.0pt,
  )],
)

== Cornflakes: hybrid copy vs scatter-gather mechanism

#grid(
  columns: (0.94fr, 1.06fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Why this paper is adjacent but useful]
    #v(0.35em)
    #card(
      [Problem],
      [Serialization overhead is entangled with packet assembly and extra copies in microsecond-scale networking.],
      fill: white,
      body-size: 11.2pt,
    )
    #v(0.4em)
    #card(
      [Core point],
      [Pure zero-copy is not always optimal. The paper argues for a #text(weight: "bold")[hybrid] choice between ordinary copies and scatter-gather transmission.],
      fill: white,
      body-size: 11.0pt,
    )
  ]],
  [#panel(fill: white)[
    #stage-card(
      [Step 1 — schema-driven hybrid pointers],
      [Cornflakes-generated objects represent string/bytes fields as `CFPtr`, a hybrid pointer that is either copied data or a reference-counted DMA-safe buffer (`RcBuf`).],
      [Layout policy is part of serialization],
      fill: c-row,
      accent: c-accent,
    )
    #v(0.18em)
    #stage-card(
      [Step 2 — networking runtime integration],
      [The stack can either flatten and copy or hand discontiguous chunks to the NIC through scatter-gather, and its combined serialize-and-send API avoids first materializing an intermediate scatter-gather array.],
      [Copy path vs scatter-gather path],
      fill: c-row,
      accent: rgb("#f97316"),
    )
    #v(0.18em)
    #stage-card(
      [Step 3 — choose the cheaper path],
      [The hybrid decision uses a per-field size threshold: on their hardware, zero-copy starts paying off around `512 B`, while smaller fields are cheaper to copy once metadata/cache-miss overhead is included.],
      [Zero-copy is regime-dependent],
      fill: c-row,
      accent: rgb("#16a34a"),
    )
  ]],
)

== Cornflakes: evidence, limits, and implication for Tonic

#table(
  columns: (0.96fr, 0.98fr, 1.06fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Evaluation path]],
  [#text(weight: "bold")[Accessible evidence]],
  [#text(weight: "bold")[How to read it]],

  [#text(size: 10.2pt)[custom KV store on a Twitter cache trace]],
  [#text(size: 10.2pt)[`15.4%` higher throughput.]],
  [#text(size: 10.2pt)[The hybrid copy / scatter-gather policy can pay off in a realistic service-like path.]],

  [#text(size: 10.2pt)[inside Redis]],
  [#text(size: 10.2pt)[`8.8%` higher throughput relative to Redis serialization.]],
  [#text(size: 10.2pt)[The paper is not limited to a custom toy runtime; layout policy matters even in an established software stack.]],

  [#text(size: 10.2pt)[precursor-path intuition, not primary SOSP evidence]],
  [#text(size: 10.2pt)[About `9.15 Gbps` highest throughput while staying under `15 μs` tail latency.]],
  [#text(size: 10.2pt)[The repo analysis explicitly treats some of the strongest convenient numbers as coming from the precursor path, not a full direct SOSP evaluation read.]],

  [#text(size: 10.2pt)[mechanism intuition]],
  [#text(size: 10.2pt)[Their mechanism combines DMA-safe `RcBuf`s, `CFPtr` hybrid pointers, a `512 B` zero-copy threshold, and serialize-and-send to remove intermediate array construction.]],
  [#text(size: 10.2pt)[This is why the paper belongs in the copy / layout bucket rather than as a transport-replacement citation: the cleverness is in policy and API co-design.]],
)

#v(0.45em)

#note[
  This module stays concrete but explicitly #text(weight: "bold")[adjacent]: the custom KV and Redis rows are the main paper-grounded evidence used here, while the precursor-path row is only mechanism intuition. The broader learning value is the #text(weight: "bold")[hybrid API design], not just the throughput delta.
]

#v(0.35em)

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Limitation],
    [This is not a gRPC paper, and the strongest accessible numeric details are partly stronger in the precursor trail than in a directly recovered full SOSP body.],
    fill: c-orange,
    body-size: 10.9pt,
  )],
  [#card(
    [Tonic lesson],
    [Keep copy policy and serialization layout visible. “Zero-copy” is not a universal answer; it is a workload- and bookkeeping-dependent decision.],
    fill: c-green,
    body-size: 11.0pt,
  )],
)

== RR-Compound: explicit low-confidence qualitative design-space note

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[What is safely known]
    #v(0.35em)
    + compatibility-first gRPC-over-RDMA framing
    + RDMA-enabled internal fast path from the artifact-repo view
    + runtime-tunable transport knobs exist in the artifact
    #v(0.45em)
    #card(
      [Artifact-level mechanism hints, not paper results],
      [RDMA disabled by default, one polling thread, `500 μs` busy-poll timeout, and `4096 KB` ring buffer per connection suggest a compatibility-first fast path with explicit runtime tuning rather than a magical transparent replacement.],
      fill: white,
      body-size: 10.6pt,
    )
  ]],
  [#panel(fill: c-orange)[
    #card(
      [What is missing],
      [This repo pass does not recover enough paper text to teach the evaluation deeply or to trust a numeric headline the way we can for the other modules.],
      fill: white,
      body-size: 11.1pt,
    )
    #v(0.4em)
    #card(
      [How to use this citation],
      [Use RR-Compound only as design-space context: API-preserving transport replacement is plausible, but the present evidence is qualitative.],
      fill: white,
      body-size: 11.0pt,
    )
    #v(0.4em)
    #card(
      [Why this slide is explicit],
      [The rebuild analysis says this module should stay low-confidence until a stronger paper-ingestion pass is done. This slide teaches the design goal, not the evaluation.],
      fill: white,
      body-size: 11.0pt,
    )
  ]],
)

== How the literature buckets map onto the Tonic byte path

#grid(
  columns: (1fr, 1fr),
  column-gutter: 16pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Outbound path: where the bytes are created]
    #v(0.35em)
    #stage-card(
      [encode],
      [prost encode of the outbound message],
      [protobuf stays visible because of the MICRO 2021 paper],
      fill: c-row,
      accent: rgb("#16a34a"),
    )
    #v(0.15em)
    #stage-card(
      [buffer reserve / grow],
      [reserve space and absorb growth / reallocation behavior],
      [copy and layout policy stay visible because of Cornflakes and TF-gRPC-Bench],
      fill: c-row,
      accent: rgb("#dc2626"),
    )
    #v(0.15em)
    #stage-card(
      [compression],
      [optional software compression before the frame is emitted],
      [compression is a separate CPU-tax bucket, not just “wire optimization”],
      fill: c-row,
      accent: c-accent,
    )
    #v(0.15em)
    #stage-card(
      [frame-header write + body handoff],
      [write the gRPC frame header and hand the body toward transport],
      [stack and framing work stay visible because of the cloud-scale RPC decomposition],
      fill: c-row,
      accent: rgb("#f97316"),
    )
  ]],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Inbound path: where the bytes are interpreted]
    #v(0.35em)
    #stage-card(
      [body accumulation],
      [collect the inbound body bytes before decode],
      [body accumulation belongs to copy / buffer lifecycle, not to protobuf alone],
      fill: white,
      accent: rgb("#dc2626"),
    )
    #v(0.15em)
    #stage-card(
      [frame parse],
      [read compression flag and frame length],
      [framing and transport glue are their own bucket],
      fill: white,
      accent: rgb("#f97316"),
    )
    #v(0.15em)
    #stage-card(
      [decompression],
      [optional software decompression on the receive side],
      [compression trade-offs must be interpreted on both throughput and tail axes],
      fill: white,
      accent: c-accent,
    )
    #v(0.15em)
    #stage-card(
      [prost decode + application handoff],
      [decode the protobuf message and return to application code],
      [this is the inbound mirror of the serialization bucket],
      fill: white,
      accent: rgb("#16a34a"),
    )
  ]],
)

#v(0.35em)

#note[
  `docs/report/architecture/003.tonic_interception_points.md` is the grounding artifact here: interceptors are metadata-only, so payload-path analysis belongs at the codec/body boundary instead.
]

== Repo evidence already shows multiple regimes

#table(
  columns: (1.08fr, 1.45fr, 1.15fr, 1.25fr),
  inset: (x: 7pt, y: 6pt),
  stroke: 0.4pt + luma(200),
  [#text(weight: "bold")[Regime]],
  [#text(weight: "bold", size: 11pt)[Repo evidence from report `009`]],
  [#text(weight: "bold", size: 11pt)[Dominant bucket]],
  [#text(weight: "bold", size: 11pt)[Characterization implication]],

  [#text(weight: "bold")[`256 B`] #linebreak() #text(size: 10pt, fill: luma(110))[low concurrency, compress=off]],
  [#text(size: 10pt)[single-thread beats multi-thread at `c=1` (`27.51k` vs `20.97k rps`); `2.1` IPC and `54.1%` frontend-bound]],
  [#panel(fill: c-row, inset: (x: 10pt, y: 8pt))[#fit-badge("runtime / control", fill: rgb("#f97316"))]],
  [#text(size: 10pt)[Tiny unary RPCs should be read as runtime-sensitive before claiming payload offload value.]],

  [#text(weight: "bold")[`4 KiB`] #linebreak() #text(size: 10pt, fill: luma(110))[medium concurrency, compress=off]],
  [#text(size: 10pt)[multi-thread wins at `c=128` (`65.79k` vs `40.59k rps`); `41.3%` backend-bound; `memmove 10.51%`, `malloc 8.67%`]],
  [#panel(fill: c-row, inset: (x: 10pt, y: 8pt))[#fit-badge("copy + allocation", fill: rgb("#ca8a04"))]],
  [#text(size: 10pt)[Buffer lifecycle has to be measured directly; one throughput number hides where the time is going.]],

  [#text(weight: "bold")[`1 MiB`] #linebreak() #text(size: 10pt, fill: luma(110))[large, uncompressed]],
  [#text(size: 10pt)[very large payloads stop benefiting from extra runtime parallelism (`1063.55` vs `961.28 rps` at `c=512`); `0.3` IPC; `memmove 71.37%` self time]],
  [#panel(fill: c-row, inset: (x: 10pt, y: 8pt))[#fit-badge("movement-dominated", fill: rgb("#dc2626"))]],
  [#text(size: 10pt)[This is the clearest copy-path characterization lane, but it still needs software controls before any hardware conclusion.]],

  [#text(weight: "bold")[`64 KiB`] #linebreak() #text(size: 10pt, fill: luma(110))[structured, compress=on]],
  [#text(size: 10pt)[throughput drops (`6.65k` vs `12.45k rps`) while p99 improves (`30.00 ms` vs `100.30 ms`); `compress_inner 61.77%` self time]],
  [#panel(fill: c-row, inset: (x: 10pt, y: 8pt))[#fit-badge("compression", fill: c-accent)]],
  [#text(size: 10pt)[Compression has to be interpreted on both throughput and tail latency axes; it is not globally good or bad.]],
)

#v(0.35em)

#note[
  Runtime crossover summary from `009`: tiny structured/off RPCs favor single-thread at low concurrency, medium structured/off RPCs favor multi-thread, and very large payloads fall back to movement-dominated behavior.
]

== Characterization rules derived from literature plus profiling

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Characterize by bucket],
    [Every result should resolve into runtime, serialization, copy/buffer lifecycle, compression, or framing/transport work rather than one throughput headline.],
    fill: c-blue,
  )],
  [#card(
    [Characterize by regime],
    [Matched size, concurrency, runtime, and payload kind matter because the dominant bucket changes with regime.],
    fill: c-green,
  )],
  [#card(
    [Use controls before claiming offload],
    [If pooled buffers or copy minimization collapse an apparent hotspot, that hotspot is not yet strong evidence for later DSA or IAX work.],
    fill: c-orange,
  )],
  [#card(
    [Separate throughput from tail latency],
    [Compression already gives cases where throughput falls while p99 improves, so one metric cannot stand in for the whole characterization.],
    fill: c-row,
  )],
)

== Concrete next-step measurement pass for Tonic

#callout(fill: c-blue, stroke: c-accent)[
  The next pass is a codec/body-path refinement with stage timers and software controls — not premature accelerator integration.
]

#v(0.45em)

#grid(
  columns: (0.92fr, 1.08fr),
  column-gutter: 16pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Timers and hooks to add]
    #v(0.35em)
    + encode / decode
    + compress / decompress
    + buffer reserve / grow
    + body accumulation / handoff
    + frame write / parse where visible
    #v(0.45em)
    #note[
      These hooks follow the real byte path from Slide 20 rather than metadata-only interception points.
    ]
  ]],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[Controls to run]
    #v(0.35em)
    #card(
      [Instrumentation-off],
      [Measure whether the timers themselves distort the path before using them for attribution.],
      fill: white,
    )
    #v(0.35em)
    #card(
      [Pooled-buffer],
      [Reduce allocation and growth noise while keeping semantics unchanged.],
      fill: white,
    )
    #v(0.35em)
    #card(
      [Copy-minimized],
      [Remove avoidable copies to learn how much of the hot path is policy rather than irreducible work.],
      fill: white,
    )
    #v(0.35em)
    #card(
      [Runtime and placement controls],
      [Re-run matched points under single vs multi runtime and controlled client/server placement.],
      fill: white,
    )
  ]],
)

== Final takeaway

#align(center + horizon)[
  #callout(fill: c-green, stroke: rgb("#16a34a"), inset: (x: 22pt, y: 18pt))[
    #text(size: 22pt, weight: "bold")[The literature gives the decomposition vocabulary and the experimental caution, but the answer still has to come from a Tonic-specific measurement campaign.]
  ]
]

#v(0.8em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [#card(
    [What is established],
    [The six-bucket view is well motivated, and report `009` already shows that Tonic moves across multiple bottleneck regimes.],
    fill: c-blue,
  )],
  [#card(
    [What remains missing],
    [No external paper replaces a stage-by-stage, stack-preserving Tonic decomposition with controls and tail interpretation.],
    fill: c-orange,
  )],
  [#card(
    [What comes next],
    [Use codec/body-boundary timers, matched regimes, and software controls to learn which buckets still justify later accelerator follow-up.],
    fill: c-green,
  )],
)
