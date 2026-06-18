# Slide plan: tonic message stack path

## Context

- **Goal:** Create concise explanatory slides for how `tonic` sends and receives one unary protobuf message. The call stack should be tonic/prost code; gRPC should appear only where we mean protocol algorithms or wire rules.
- **Audience:** Systems/Rust audience that knows networking basics but may not know the tonic internals.
- **Target duration:** 6–7 minutes.
- **Grounding:** tonic's standard message path: generated client/server API, tonic client/server runtime, tonic codec framing, `prost`, `tower::Service`, `hyper`, `h2`, `tokio`, TCP/TLS; gRPC protocol framing/compression/status rules where relevant.

## Story spine

1. Start at the generated tonic API, then move down the tonic/prost call stacks; keep `Grpc` terminology tied to tonic runtime types.
2. Introduce the gRPC wire-shape anchor underneath that stack: `Rust T → protobuf bytes → [flag:1][len:4][payload] → HTTP/2 DATA`, then use it to explain the send and receive overviews before diving into detailed mechanics.
3. Show component boundaries as Chronos lifelines and call/future active spans as activation bars on the overview/detail slides.
4. Emphasize the implication: accelerators can only replace selected bulk-byte spans, not the whole tonic stack.

## Visual style

- Use a soft blue-white page background, a consistent accent-bar slide header, and a two-tone divider to give each page a conference-deck frame.
- Use open captioned diagram areas with hairline rules on the overview slides; dense detail diagrams may keep light containment when readability needs it.
- Use native Typst boxes/arrows, not Chronos, for the message-shape anchor once the tonic/prost stack has been established so non-code readers see the core transformation before the sequence overviews.
- Use compact legends and activation-bar glyphs so Chronos semantics stay visually explicit: lifelines are software components, bars are active spans, and colored spans are candidates.
- On detail slides, use the compact repeated offload legend (`orange = IAX compression`, `blue = DSA copy/CRC`, `gray = CPU control`) so color semantics stay visible without consuming explanation height; keep the longer hardware-as-non-participant caveat in slide notes where needed.
- Use open opportunity strips, candidate badges, and a lightly ruled zebra matrix on the offload interpretation slide for scanability without stacked card blocks.
- Keep dense sequence slides diagram-first, with an open numbered side column and accent rules instead of stacked cards or large bottom callouts.
- Implement numbered side grids with Typst-native enumerations (`+` list items) and scoped `#set enum(numbering: ...)` marker styling for accent-colored bold numbers; do not use custom numbered-list helpers or manual grid-based numbering.
- For sequence/detail slides with numbered explanations, place matching numbered callout markers in the Chronos illustration near the corresponding arrow, participant span, or colored candidate span. Use the same highlight color for the diagram marker and the corresponding list number so the visual-to-text mapping is explicit.

## Slide blueprint

### Slide 1 — `How user code uses tonic`

- **Core takeaway:** The public surface is generated client/server glue plus `Request<T>` / `Response<T>`; the user normally does not touch frames, DATA, or codec bodies.
- **Components to show:**
  - Zebraw-rendered Rust client usage: connect, call generated method, unwrap `Response<T>`.
  - Zebraw-rendered Rust server usage: `Server::builder().add_service(...).serve(...)` plus handler signature.
- **Visual/delivery notes:**
  - Preserve 16:9 readability; use two open side-by-side code columns with thin title rules and a center hairline separator instead of card shells.
  - Add a bottom hairline boundary note listing what stays below generated API code: transport, codec, HTTP/2 body machinery, frames/DATA chunks.

### Slide 2 — `Down the tonic/prost call stack`

- **Core takeaway:** The call stack is tonic/prost code. `Grpc` names are tonic runtime types, not a separate gRPC library inside tonic.
- **Components to show:**
  - Large Zebraw call-stack block for client request encode: generated method → `tonic::client::Grpc::unary` → `Grpc::streaming` → `EncodeBody` → `ProstEncoder` → `prost`.
  - Large Zebraw call-stack block for server request decode: generated service dispatch → `tonic::server::Grpc::unary` → `Streaming` → `poll_decode_chunk` → `ProstDecoder` → handler.
  - Note that response encode/decode mirrors the same shape.
- **Visual/delivery notes:**
  - Keep the client/server stacks as equal-height open columns, with the response mirror as a full-width hairline footer note so the page reads as one composed call-stack view.

### Slide 3 — `What tonic does underneath for gRPC`

- **Core takeaway:** Under the tonic/prost call stack, tonic implements gRPC protocol work: lazy body polling, gRPC message framing, optional compression, HTTP/2 DATA, and trailers.
- **Components to show:**
  - Large gRPC message frame grid: `flag = 0`, `u32 len`, protobuf payload.
  - Bullets for body polling, protobuf payload, gRPC frame, compression boundary, and HTTP/2 integration.
- **Visual/delivery notes:**
  - Keep the frame anchor, then set protocol-work bullets as an open ruled list rather than a boxed protocol-work panel.

### Slide 4 — `Send path: typed Rust becomes DATA only at the edge`

- **Core takeaway:** Sending is a staged tonic handoff: `tonic` receives typed RPC state, `prost` serializes payload bytes, `tonic` applies the gRPC message-framing algorithm, and the HTTP/2 stack moves DATA bytes outward.
- **Components to show:**
  - Compact Chronos participants: app, tonic, prost, HTTP/2 stack, tokio/TCP/TLS. Collapse tower/hyper/h2 detail here; later slides provide the full mechanics.
  - Six numbered actions: typed RPC call; lazy Body; protobuf bytes; gRPC framing in tonic; HTTP/2 DATA; socket write.
  - Activation bars represent active function/future spans for this RPC poll, not package lifetime or ownership.
- **Bottom note:** Overview intentionally collapses the lower stack so the representation changes stay visible; `tower`/`hyper`/`h2`/`tokio` specifics appear in detail slides.
- **Visual/delivery notes:**
  - Use a captioned open diagram area next to a numbered explanation column so the slide scans as trace first, interpretation second without nested side cards.
  - Solid arrows are outbound calls; dashed arrows are returns/data acknowledgements.
  - Numbered side grid must map 1:1 to the diagram labels and keep each label to one action.

### Slide 5 — `Receive path: DATA becomes Rust only after deframe and decode`

- **Core takeaway:** Receiving reverses the shape anchor: HTTP/2 DATA returns upward, `tonic` reconstructs one complete gRPC-framed message, optional decompression happens at that protocol boundary, and `prost` rebuilds typed `T`.
- **Components to show:**
  - Same compact Chronos participants as the send overview for visual continuity.
  - Six numbered actions: app demand; poll body; socket read; DATA frames; gRPC deframe in tonic; protobuf decode.
  - Activation bars represent active function/future spans during receive polling.
- **Bottom note:** Final RPC success/error is carried in HTTP/2 trailers; typed protobuf decode and status/trailer handling are separate paths.
- **Visual/delivery notes:**
  - Keep the decode overview structurally parallel to the send overview, using the same open diagram area and numbered explanation-column treatment.
  - Numbered side grid must map 1:1 to the diagram labels and keep caveats unnumbered.

### Slide 6 — `Compression first; copies only when large; control stays CPU`

- **Core takeaway:** The plausible offload surface is narrow and ordered: IAX/IAA first for compression/decompression, DSA only for large unavoidable contiguous copies, and CPU for schema/control-heavy work.
- **Components to show:**
  - Three open opportunity strips: compression first, copies only when large, control stays CPU.
  - Lightweight matrix mapping stages to candidates and caveats:
    - `compress` / `decompress` → IAX/IAA.
    - large `bytes`/`string` field append during `prost` encode → DSA `data_move` candidate only for large contiguous payload copies.
    - body assembly or post-encode copy/integrity → DSA `data_move` / `copy_crc` only if the copy remains unavoidable.
    - protobuf tags/varints/schema traversal and typed decode → CPU.
    - h2/Tower/Tokio control → CPU.
  - Evidence note from repo reports: compression/decompression ranked highest; DSA copy/CRC is workload-size dependent; current Tonic+DSA notes only support trying DSA around ~1 MiB+ payloads, with 2–4 MiB more promising but still directional.
- **Visual/delivery notes:**
  - Keep this as the interpretation slide before showing offload mechanics.
  - Use caveats prominently; do not imply DSA generally accelerates tonic.
  - Put the stage matrix under a lightweight label/rule and use thin stage accent bars tied to the candidate color families.

### Slide 7 — `Send detail A: tonic encode decision points`

- **Core takeaway:** Send encoding is demand-driven: protobuf/schema work stays CPU, only large payload copies and compression are candidate spans, and the final gRPC flag/length prefix plus h2 handoff stay CPU.
- **Components to show:**
  - Chronos participants: request stream, `EncodedBytes`, `BytesMut`, prost, `compression.rs`, h2 body poller.
  - Six chronological labels: downstream pull; message pull; protobuf; large field copy; compression; write prefix.
  - Orange active span on the `compression.rs` software boundary: IAX/IAA candidate replacing `compress` / flate2.
  - Blue active spans on buffer/copy boundaries: DSA candidate for large `bytes`/`string` payload append; unnumbered copy/CRC note only if a large post-encode copy remains.
- **Visual/delivery notes:**
  - Keep this slide focused on tonic codec decision points, not hardware submission details.
  - Make clear that the apparent start from the right is downstream demand: h2/hyper polls the body, which pulls from `EncodedBytes` and then the request stream.
  - Number only chronological actions; keep DSA copy/CRC caveats in unnumbered note text.
  - Use short bold-label side bullets in an open accent-rule sequence column that maps 1:1 to the six diagram markers, with the caveat separated below as a lighter rule note.
  - Give the Chronos panel more of the row, add a clearer gutter, and reduce frame inset/stroke so the dense diagram reads larger without changing its chronology.

### Slide 8 — `Async encode: two polls, one owned operation`

- **Core takeaway:** Real DSA/IAX integration is a two-poll ownership shape: first poll prepares owned state and submits work, returns `Poll::Pending`, completion wakes the task without blocking the executor, and the second poll checks status/errors before emitting DATA.
- **Components to show:**
  - Dominant top state strip: first poll → in flight / pending → second poll, with short labels and no duplicate numeric markers.
  - Chronos participants: tokio/h2, tonic body, a visually grouped owned async state (`OwnedPendingOp` plus completion record), and HTTP body.
  - Seven chronological labels remain in the diagram: first poll; own buffers; submit work; return `Pending`; wake task; check status; emit DATA.
  - Orange active span is only the IAX/IAA-backed compression example. Blue DSA `copy_crc` uses the same control pattern and belongs in an unnumbered caveat, not on the submit arrow or as a participant.
- **Visual/delivery notes:**
  - Make the state strip the talk track: first poll submits/yields, in-flight state owns buffers while the executor is free, second poll emits or errors.
  - Do not color `Pending` orange; use a neutral gray/purple/blue-gray so orange remains reserved for IAX compression.
  - Enlarge Chronos text/markers, reduce small-arrow clutter, and use more whitespace so the page reads as a teaching diagram rather than a dense protocol trace.
  - Visually emphasize the async gap between `return Pending` and `wake body task`, preferably with a subtle band/callout reading “executor free; buffers owned by pending state.”
  - Replace the right-side seven-step repeat with a compact invariant box: own buffers across `Pending`; completion stores status/length and wakes; second poll observes success/error and emits DATA.
  - Keep the DSA copy/CRC note small and explicit: same two-poll shape, not another numbered flow or hardware participant.

### Slide 9 — `Receive detail A: frame accumulation and deframe`

- **Core takeaway:** Receive first accumulates HTTP/2 DATA into tonic buffers and parses the 5-byte envelope; DSA is plausible only for large copy-heavy assembly.
- **Components to show:**
  - Chronos participants: app, `StreamingInner`, HTTP body, `BytesMut`, read state, trailers.
  - Six chronological labels: demand; poll body; DATA bytes; append bytes; read prefix; wait body.
  - Trailer/status path remains unnumbered CPU/control metadata.
  - DSA caveat: only useful if fragmented-message assembly becomes a large real copy.
- **Visual/delivery notes:**
  - Keep header parsing visibly CPU/control, not an offload candidate.
  - Keep DSA/IAX participant caveats in unnumbered accent-rule note text below the sequence column.
  - Use the wider diagram panel and short bold-label side bullets that map 1:1 to the six diagram markers without enclosing the right rail in cards.

### Slide 10 — `Receive detail B: decompress and typed decode`

- **Core takeaway:** Once a full message is available, optional copy and decompression may be candidate spans, but `prost` typed decode and buffer advancement remain CPU work.
- **Components to show:**
  - Chronos participants: `StreamingInner`, message buffer, copy/assembly span, `compression.rs`, decompression buffer, prost, app.
  - Six chronological labels: full message; gather/copy; decompress; decode; advance; return.
  - Offloaded copy or decompression can return `Pending` and wake later, but Pending/wake annotations are unnumbered side paths.
  - Bottom caveat: evaluate `batch_n` and logical concurrency separately; colored spans are potential accelerator-backed replacements inside software lifelines, not hardware lifelines.
- **Visual/delivery notes:**
  - Use same orange/blue/gray semantics as the send-detail slides.
  - Emphasize IAX as the strongest receive-side candidate and DSA as copy-dependent.
  - Avoid modeling IAX/IAA or DSA as separate Chronos participants; represent them only through colored candidate spans and the legend.
  - Use the wider diagram panel and open sequence/caveat rule treatment; do not number meta-statements.
