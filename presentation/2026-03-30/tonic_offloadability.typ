// Tonic offloadability presentation

#import "../template.typ": callout, card, deck, fit-badge, palette, stage-card

#show: deck.with(
  leading: 0.9em,
  spacing: 0.95em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-orange = palette.orange
#let c-green = palette.green
#let c-red = palette.red

= Offloadability of Tonic

#align(center + horizon)[
  #text(size: 19pt)[Which parts of the Rust gRPC datapath can move to DSA / IAX?]
  #v(0.8em)
  #text(size: 16pt)[Hongtao Zhang]
  #v(0.3em)
  #text(size: 14pt, fill: luma(120))[Mar 30, 2026]
]

#v(0.9em)

#callout(fill: c-blue, stroke: c-accent)[
  *Working assumption in this repo*: keep the gRPC API and most control logic unchanged;
  offload only the regular, byte-oriented kernels below the API boundary.
]

== What does “offloadability” mean here?

- *Not* “move Tonic wholesale onto hardware”
- *Not* “replace tokio, tower, or HTTP/2 with a device runtime”
- *Yes*: identify the hot byte-path kernels inside request/response handling that are:
  - regular enough to batch
  - large enough to amortize setup cost
  - decoupled enough from control flow to sit behind a stable interface

#v(0.7em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Offloadability = useful accelerator fit, not mere technical possibility.*
  A stage is attractive only if device setup + memory traffic + synchronization
  cost less than the CPU work it removes.
]

== Tonic datapath, decomposed

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr, auto, 1fr, auto, 1fr),
  gutter: 8pt,
  [#stage-card(
    [Codec],
    [protobuf encode/decode, buffer writes, copies],
    [strong byte-path fit],
    fill: c-blue,
    accent: rgb("#16a34a"),
  )],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Middleware], [CRC-32C, compression, decompression], [strong byte-path fit], fill: c-blue, accent: rgb("#16a34a"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Framing], [gRPC prefix + HTTP/2 frame assembly], [partial fit], fill: c-orange, accent: rgb("#f59e0b"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Runtime], [tokio scheduling, wakeups, futures], [CPU control plane], fill: white, accent: rgb("#dc2626"))],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Network I/O], [socket / NIC path], [outside current focus], fill: white, accent: rgb("#dc2626"))],
)

#v(0.2em)

#grid(
  columns: (3fr, 2fr),
  gutter: 14pt,
  [#callout(fill: c-green, stroke: rgb("#16a34a"))[
    *Initial candidates*: codec, middleware, and parts of framing appear to
    contain the most regular data movement.
  ]],
  [#callout(fill: c-red, stroke: rgb("#dc2626"))[
    *Likely CPU-resident pieces*: runtime and protocol state currently look
    less promising for offload.
  ]],
)

#v(0.8em)

#callout(fill: c-blue, stroke: c-accent)[
  The repo layout suggests a possible split: *codec + middleware + FFI bridges*
  may be useful places to investigate first, while the async runtime remains the
  baseline comparison point.
]

== What happens to one message?

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr, auto, 1fr, auto, 1fr),
  gutter: 8pt,
  [#stage-card(
    [1. App object],
    [service handler creates or receives a typed protobuf message],
    [CPU],
    fill: c-blue,
    accent: rgb("#2563eb"),
  )],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card(
    [2. Encode / decode],
    [prost writes bytes into buffers or reconstructs fields],
    [mixed],
    fill: c-blue,
    accent: rgb("#16a34a"),
  )],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card(
    [3. Byte transforms],
    [copy, CRC, compression, decompression on payload bytes],
    [strongest current candidate],
    fill: c-green,
    accent: rgb("#16a34a"),
  )],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card(
    [4. gRPC / HTTP2 wrap],
    [length prefix, metadata, frame assembly],
    [partial fit],
    fill: c-orange,
    accent: rgb("#f59e0b"),
  )],
  [#align(center + horizon)[#text(size: 18pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card(
    [5. Runtime + socket],
    [tokio polling, scheduling, and actual network send/receive],
    [CPU],
    fill: white,
    accent: rgb("#dc2626"),
  )],
)

#v(0.8em)

#grid(
  columns: (1.2fr, 1fr),
  gutter: 14pt,
  [#callout(fill: c-green, stroke: rgb("#16a34a"))[
    *Working intuition*: a message alternates between *structured-object work* and *raw-byte work*.
    The raw-byte segment looks like the most plausible place to test accelerator
    benefit, while protocol/runtime work remains the harder fit.
  ]],
  [#card([Repo mapping], [
    `accel-codec` → step 2 \
    `accel-middleware` → step 3 \
    vendored `tonic` transport → step 4 \
    `tonic-profile` → end-to-end timing
  ], fill: c-blue)],
)

== Practical criteria for exploring offloadability

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card([1. Regular data access], [Contiguous or pooled buffers beat pointer-heavy object graphs.], fill: c-blue)],
  [#card(
    [2. Streaming / arithmetic density],
    [Compression and CRC are better fits than branchy protocol logic.],
    fill: c-blue,
  )],
  [#card(
    [3. Batchability],
    [Many messages or larger payloads can amortize enqueue, DMA, and completion cost.],
    fill: c-blue,
  )],
  [#card([4. Control isolation], [Prefer kernels that do not require deep visibility into service logic.], fill: c-blue)],
)

#v(0.8em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  *Heuristic*: a Tonic stage becomes interesting to offload when the kernel is
  regular, the payload regime is large enough, and the interface can stay clean
  with CPU fallback.
]

== Current assessment of offload potential by stage

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 14pt,
  [
    #card([More promising], [
      #fit-badge([HIGH], fill: rgb("#16a34a")) #h(0.5em)Copy / buffer movement \
      #text(size: 12pt, fill: luma(75))[Streaming bytes; DMA-friendly.]
      \
      #v(0.45em)
      #fit-badge([HIGH], fill: rgb("#16a34a")) #h(0.5em)CRC-32C / integrity \
      #text(size: 12pt, fill: luma(75))[Small semantic surface.]
      \
      #v(0.45em)
      #fit-badge([HIGH], fill: rgb("#16a34a")) #h(0.5em)Compression / decompression \
      #text(size: 12pt, fill: luma(75))[Compute-heavy and message-local.]
    ], fill: c-green)
  ],
  [
    #card([Worth testing, case-dependent], [
      #fit-badge([MED], fill: rgb("#f59e0b")) #h(0.5em)Length-prefix framing \
      #text(size: 12pt, fill: luma(75))[Simple bytes, but often too cheap.]
      \
      #v(0.45em)
      #fit-badge([MED], fill: rgb("#f59e0b")) #h(0.5em)Protobuf serialization \
      #text(size: 12pt, fill: luma(75))[Byte writes may help; object traversal remains CPU-heavy.]
    ], fill: c-orange)
  ],
  [
    #card([Less promising under current assumptions], [
      #fit-badge([LOW], fill: rgb("#dc2626")) #h(0.5em)HTTP/2 state management \
      #text(size: 12pt, fill: luma(75))[Too much branching and protocol state.]
      \
      #v(0.45em)
      #fit-badge([LOW], fill: rgb("#dc2626")) #h(0.5em)Tokio / tower scheduling \
      #text(size: 12pt, fill: luma(75))[Pure control-plane work.]
    ], fill: c-red)
  ],
)

== Tonic request path: concrete layers and the best middle point

#card([Client path], [
  generated client (`new`, `with_interceptor`) → optional interceptor \
  → `tonic::client::Grpc<T>` → `Endpoint` / `Channel` / `Service` \
  → `EncodeBody::new_client(...)` → `hyper` / `h2`
], fill: c-blue)

#v(0.45em)

#card([Server path], [
  `Server::builder()` → optional `Server::layer(...)` → generated `FooServer<T>` \
  → optional interceptor → `tonic::server::Grpc<TCodec>` \
  → `Streaming::new_request(...)` / `EncodeBody::new_server(...)` → `hyper` / `h2`
], fill: c-blue)

#v(0.65em)

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr, auto, 1fr),
  gutter: 8pt,
  [#card([Generated API], [
    `tonic-build` output \
    constructors + method wrappers \
    method metadata insertion \
    ergonomic entry point
  ], fill: c-blue)],
  [#align(center + horizon)[#text(size: 17pt, weight: "bold", fill: luma(110))[→]]],
  [#card([gRPC interceptor], [
    metadata + extensions only \
    auth / admission / hints \
    early rejection \
    body intentionally hidden
  ], fill: c-orange)],
  [#align(center + horizon)[#text(size: 17pt, weight: "bold", fill: luma(110))[→]]],
  [#stage-card([Tower layer / service], [
    whole request / response boundary \
    tracing, metrics, policy \
    body wrapping + extension injection \
    transparent across many services
  ], [good middle point], fill: c-green, accent: rgb("#16a34a"))],
  [#align(center + horizon)[#text(size: 17pt, weight: "bold", fill: luma(110))[→]]],
  [#card([`tonic::{client,server}::Grpc`], [
    runtime gRPC entry point \
    request preparation + status handling \
    bridge from service layer to payload path \
    still above raw bytes
  ], fill: c-blue)],
)

#v(0.65em)

#grid(
  columns: (1.4fr, 1fr),
  gutter: 14pt,
  [#callout(fill: c-green, stroke: rgb("#16a34a"))[
    *Good middle point*: the *Tower layer / service* boundary is where tonic is
    still structured as whole requests/responses rather than raw frames. That
    makes it a practical place to attach tracing, policy, request classification,
    body wrappers, and offload hints without rewriting generated stubs.
  ]],
  [#callout(fill: c-orange, stroke: rgb("#f59e0b"))[
    *Why interceptors are not enough*: they are useful for metadata and early
    reject, but tonic deliberately hides the body there.
  ]],
)

== Tonic request path: deeper payload hooks

#grid(
  columns: (1fr, auto, 1fr, auto, 1fr),
  gutter: 8pt,
  [#stage-card([Codec / body boundary], [
    protobuf encode / decode \
    compression / decompression \
    payload buffers become concrete \
    copy + checksum opportunities
  ], [deepest payload hook], fill: c-orange, accent: rgb("#f59e0b"))],
  [#align(center + horizon)[#text(size: 17pt, weight: "bold", fill: luma(110))[→]]],
  [#card([`EncodeBody` / `Streaming`], [
    reserve 5-byte gRPC header \
    emit / parse message bytes \
    assemble frames \
    request / response stream bodies
  ], fill: c-blue)],
  [#align(center + horizon)[#text(size: 17pt, weight: "bold", fill: luma(110))[→]]],
  [#card([`hyper` / `h2` / transport], [
    HTTP/2 execution \
    flow control + stream scheduling \
    connector / channel machinery \
    not the first payload-offload target
  ], fill: white)],
)

#v(0.6em)

#grid(
  columns: (1.25fr, 1fr, 1fr),
  gutter: 12pt,
  [#callout(fill: c-blue, stroke: c-accent)[
    *Where bytes become concrete*: this is the first place where framing,
    compression, and payload buffers are explicit enough for copy/checksum/
    compression work to be inserted directly.
  ]],
  [#callout(fill: c-orange, stroke: rgb("#f59e0b"))[
    *Why this is deeper than the middle point*: the codec/body path is more exact
    for byte-heavy work, but it is also less ergonomic and closer to tonic internals.
  ]],
  [#callout(fill: white, stroke: luma(140))[
    *Reading of the stack*: Tower is the practical middle hook; codec/body is the
    stronger payload hook; transport is mostly for connection and protocol tuning.
  ]],
)

== One possible split to evaluate

#card([CPU / control plane], [
  request dispatch  ·  futures / tower  ·  protocol state  ·  service logic
], fill: c-blue)

#v(0.55em)

#align(center)[#text(size: 18pt, weight: "bold", fill: c-accent)[↓ good middle point: Tower layer / service wrapper ↓]]

#v(0.35em)

#grid(
  columns: 3,
  gutter: 12pt,
  [#card([`accel-codec`], [pooled-buffer codec boundary and copy-aware staging], fill: c-blue)],
  [#card([`accel-middleware`], [good middle point for policy, tagging, body wrapping, CRC, and compression hooks], fill: c-green)],
  [#card([`dsa-ffi` / `iax-ffi`], [device-facing kernels and completions], fill: c-blue)],
)

#v(0.55em)

#card([Byte-oriented work under consideration for offload], [
  copy  ·  CRC  ·  compression  ·  selected buffer transforms
], fill: c-green)

#v(0.7em)

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  A favorable outcome would be that some byte-path work moves off CPU while
  control-heavy logic remains unchanged. In practice, the middle-layer question
  is whether `accel-middleware` is sufficient, or whether measurements force us
  to drop deeper into codec/body internals; `tonic-profile` is the end-to-end check.
]

== What still needs validation?

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#card(
    [Payload regime],
    [How often are payloads large enough to amortize enqueue, DMA, and completion cost?],
    fill: c-blue,
  )],
  [#card(
    [Dominant costs],
    [Which costs dominate in practice: serialization, copies, compression, framing, or runtime scheduling?],
    fill: c-blue,
  )],
  [#card([Buffering effects], [Does pooled buffering change the comparison more than hardware offload does?], fill: c-blue)],
  [#card([RPC style], [Do unary, medium-payload, and streaming RPCs require different conclusions?], fill: c-blue)],
)

#v(0.8em)

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  At this stage, the goal is not to claim a winning architecture, but to narrow
  the set of plausible interception points and payload regimes worth measuring.
]

== Suggested evaluation sequence

#grid(
  columns: 5,
  gutter: 10pt,
  [#card([1. Baseline], [`tonic-profile` for serialization, copies, compression, framing, runtime], fill: c-blue)],
  [#card([2. Regularize memory behavior], [use pooled buffers before adding hardware assumptions], fill: c-green)],
  [#card([3. Test middleware-heavy kernels], [start with CRC and compression], fill: c-green)],
  [#card(
    [4. Revisit codec decomposition],
    [separate byte movement from object traversal if results justify it],
    fill: c-orange,
  )],
  [#card([5. Compare by payload regime], [tiny unary vs medium payload vs streaming], fill: white)],
)

#v(0.7em)

#grid(
  columns: 3,
  gutter: 12pt,
  [#callout(fill: c-green, stroke: rgb("#16a34a"))[*Immediate experiments*: pooled-buffer codec + CRC/compression middleware]],
  [#callout(fill: c-orange, stroke: rgb("#f59e0b"))[*If warranted by results*: serialization decomposition]],
  [#callout(fill: white, stroke: luma(140))[*Probably lower priority*: transport/runtime changes]],
)

== Takeaway

#callout(fill: c-blue, stroke: c-accent)[
  *Current read*: Tonic appears more amenable to selective offload than wholesale offload.
]

- The strongest candidates so far are *copy*, *CRC*, and *compression*
- *Serialization* appears mixed and may need decomposition before it can be evaluated cleanly
- *HTTP/2, tower, and tokio* currently look more control-heavy than offload-friendly
- The repo's current crate layout suggests reasonable places to investigate first

#v(0.9em)

#text(size: 18pt, weight: "bold", fill: c-title)[Working conclusion:]
Treat Tonic as the baseline API/runtime shell and test whether selected byte kernels underneath it benefit from acceleration.
