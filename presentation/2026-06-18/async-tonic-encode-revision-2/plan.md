# Slide plan: async Tonic encode — revision 2

## Context

- **Goal:** Explain async Tonic encode to a systems research advisor by first establishing the original synchronous gRPC/Tonic encode path, then showing why that path cannot exploit concurrent DSA/IAX hardware resources.
- **Audience:** Systems research advisor. Assumes general systems background and high-level familiarity with RPC/protobuf ideas, but not Rust async, Tonic internals, Prost internals, or this repo's encode path.
- **Source grounding:**
  - `accel-rpc/tonic/tonic/src/codec/mod.rs`: `Encoder::Encode: Future<Output = Result<EncodeBuffer, Error>> + Send + 'static`; `encode(self: Pin<&mut Self>, item, dst: EncodeBuffer) -> Result<Self::Encode, Error>`.
  - `accel-rpc/tonic/tonic/src/codec/encode.rs`: `EncodedBytes` stores `in_flight: Option<T::Encode>` and `in_flight_offset`; reserves the gRPC header before starting encode; polls the future; finishes compression/header/size after `Ready`.
  - `accel-rpc/tonic/tonic-dsa-bytes-async/src/prost_codec.rs`: `DsaAsyncProstEncode<T>` owns `state`, `item`, `sink`, `options`, and `stage`; field drop order is part of the safety contract.
  - `accel-rpc/tonic/prost/prost/src/message.rs` and `transfer.rs`: `Message::poll_encode_raw`, `AsyncEncodeTarget`, and `PollEncodeState` split CPU protobuf structure writes from suspendable payload copies.
  - `accel-rpc/tonic/prost/prost-derive/src/field/scalar.rs`: generated async encoding writes tag + length, sets a pending phase, then polls payload copy; a later poll resumes the payload path rather than rewriting the prefix.

## Story spine

1. Start with the original path as a concrete data path: application message → Prost protobuf body → Tonic gRPC header → HTTP/2 bytes.
2. Point out the baseline assumption only after the path is visible: protobuf encoding runs to completion on the CPU before send continues.
3. Then locate the cost: large payload bytes are copied during protobuf encode.
4. Show why this is not sufficient for DSA/IAX: the device can run multiple payload copies concurrently, but the original path naturally waits inside one message.
5. Name the lost opportunity: hardware lanes exist, but the synchronous encode boundary exposes no place to park unfinished copy work while other messages progress.
6. Introduce async encode as the overlap mechanism: submit payload-copy work, return control to the runtime, and later resume when completion arrives.
7. Explain the new correctness problem created by overlap: encoding can pause halfway through a protobuf field while other work runs.
8. Explain why pausing is hard: some protobuf prefix bytes may already be written, but the payload copy is still incomplete.
9. Then introduce the design answer: turn encode into an owned operation that Tonic can store and poll until completion.
10. Close with the reusable systems rule: using concurrent hardware from an async software stack requires both overlap and exact resumable state.

## Visual language

- **Gray:** ordinary gRPC/protobuf/network pipeline mechanics.
- **Orange:** hardware/offload/waiting-later consequence.
- **Violet:** precise protobuf byte-level checkpoint and resume state.
- **Blue:** Tonic's stored operation / ownership boundary.
- **Green:** completed bytes, correct resume, returned buffer, final reusable rule.
- **Red:** corrupt resume, invalid lifetime, or memory-safety hazard.
- Touying usage: each slide is one `#slide` with one dominant layout object; visible text stays large; no speaker-note/takeaway footer is used. Essential teaching sentences are promoted into the main slide body.

## Slide-by-slide blueprint

### Slide 1 — Original Tonic Send Path

**Teaching purpose**
Establish the baseline data path before criticizing it: how one application message becomes bytes on the network in the original Tonic/protobuf stack.

**Audience takeaway**
Before acceleration, Tonic sends a message by letting Prost finish the protobuf body on the CPU, then adding the gRPC header and handing bytes to HTTP/2.

**Layout**
Use a horizontal Mermaid graph across the top: application message → Prost: encode protobuf body → Tonic: add gRPC header → HTTP/2: send bytes. Under it, add two description cards: a larger `What this baseline does` card explaining the layer sequence, and a smaller `Baseline assumption` card saying CPU encoding finishes before the send path moves on. Bottom sentence anchors the deck: `This is the path we start from before introducing accelerator copies.`

**Exact visible text**
- Title: `Original Tonic Send Path`
- Mermaid labels: `Application message` → `Prost: encode protobuf body` → `Tonic: add gRPC header` → `HTTP/2: send bytes`
- Description: `Prost turns the application message into a protobuf body. Tonic prepends the gRPC frame header. HTTP/2 sends the resulting bytes.`
- Baseline assumption: `CPU encoding finishes before the send path moves on.`
- Bottom sentence: `This is the path we start from before introducing accelerator copies.`

**Visual treatment**
Gray Mermaid pipeline for ordinary data movement. Blue description card for the baseline explanation. Keep hardware and async out of this slide; the audience should first understand the baseline path.

**Speaker message**
`The first slide is only the baseline: where the bytes come from, which layers touch them, and when the send path moves on.`

**Transition**
Next show where the expensive copy appears inside this baseline path.

### Slide 2 — Where the Cost Appears

**Teaching purpose**
Connect the baseline path to the accelerator motivation by locating the expensive memory movement inside protobuf encoding.

**Audience takeaway**
The candidate for acceleration is not the gRPC header; it is large payload bytes copied while Prost builds the protobuf body.

**Layout**
Reuse the Slide 1 pipeline without wrapping it in a decorative card. Below it, use two plain text columns: left column expands `Inside Prost encode`; right column explains the acceleration target. Only `copy payload bytes` gets a small orange highlight because it is the actual object of attention.

**Exact visible text**
- Title: `Where the Copy Cost Appears`
- Pipeline context: `application message` → `Prost encode` → `Tonic frame` → `HTTP/2 send`
- Zoom title: `inside Prost encode`
- Sub-steps: `write tags + lengths` / `copy payload bytes`
- Payload examples: `large tensors` / `byte arrays` / `message payloads`
- Bottom thesis: `The expensive part is a copy inside the body-encoding step.`

**Visual treatment**
No unnecessary cards. Keep the main pipeline gray and subdued. Make only the payload-copy step orange. Do not introduce async yet; this slide only identifies the target operation.

**Speaker message**
`Now we know where to look: acceleration targets the payload copy buried inside protobuf body construction.`

**Transition**
Next show why simply replacing that copy with a blocking DSA/IAX call still does not use the hardware well.

### Slide 3 — Async Means Multiplexing

**Teaching purpose**
Make the deck theme explicit: async encode is not about making one copy faster; it is about multiplexing many message copies over concurrent DSA/IAX resources.

**Audience takeaway**
Blocking offload lets one message own the path. Async encode parks unfinished copy work so other messages can submit work and keep hardware lanes busy.

**Layout**
Two-column contrast with minimal containers. Left: blocking encode timeline `copy A → wait → complete A`, then `copy B → wait → complete B`, labeled as one message owning the software path. Right: multiplexing target with DSA/IAX lanes showing `copy A`, `copy B`, and `copy C` in flight. Bottom thesis names async encode as the multiplexing mechanism.

**Exact visible text**
- Title: `Async Means Multiplexing`
- Left title: `Blocking: one message owns the path`
- Left flows: `copy A` → `wait` → `complete A`; `copy B` → `wait` → `complete B`
- Left note: `The software path waits instead of multiplexing.`
- Right title: `Async goal: multiplex copies`
- Right label: `many messages keep device lanes busy`
- Right lanes: `lane 1: copy A in flight`; `lane 2: copy B in flight`; `lane 3: copy C in flight`
- Bottom thesis: `Async encode is a multiplexing mechanism: park unfinished copies, then run other work.`

**Visual treatment**
No decorative wrapper cards. Use orange for submitted copy work and multiplexing, red for waits/serialization, and green only for completed bodies. Keep Rust/Tonic API details out; this slide motivates the need for async shape.

**Speaker message**
`The theme of async here is multiplexing: one copy can be unfinished while the software stack starts or progresses other messages.`

**Transition**
Next introduce the new software shape: start work, store unfinished encode state, and resume on hardware completion.

### Slide 4 — How Async Encode Multiplexes Work

**Teaching purpose**
Explain the practical meaning of `Pending`: it is not idle time; it gives the runtime permission to stop waiting on this message and run other ready work.

**Audience takeaway**
When encode returns `Pending`, the current stream cannot produce bytes yet, but the thread is not blocked. The unfinished encode is remembered in `in_flight`, and the runtime can poll other RPC streams, other message encodes, or completion handling until this operation is woken and resumed.

**Layout**
Use three large steps across the slide: `Start hardware work` → `Pending is not idle` → `Resume on completion`. Under the steps, add two notes: what can run while this encode is pending, and what state must be remembered to resume correctly.

**Exact visible text**
- Title: `How Async Encode Multiplexes Work`
- Step 1: `Start hardware work` / `submit payload copy` / `Tonic moves the message and output buffer into one owned encode operation.`
- Step 2: `Pending is not idle` / `give runtime a choice` / `This stream cannot finish now, so it returns `Pending` instead of blocking the thread.`
- Step 3: `Resume on completion` / `return completed buffer` / `When DSA completes, Tonic polls the same operation again and finishes the gRPC frame.`
- Can run now: `other RPC streams · other message encodes · completion handling`
- Remembered state: ``EncodedBytes.in_flight` keeps `DsaAsyncProstEncode`: message + output buffer + Prost resume point + DSA completion state.`
- Bottom thesis: `When encode returns `Pending`, the runtime can stop waiting on this message and multiplex other work.`

**Visual treatment**
Make `Pending` the conceptual center of the slide. Avoid a code-symbol-heavy pipeline; code identifiers appear only to explain what preserves the unfinished operation.

**Speaker message**
`The key point is what Pending allows: the unfinished encode is parked, so the executor can run other ready work instead of blocking on one hardware copy.`


**Transition**
Next explain the Rust async mechanism that makes this park-and-resume behavior possible.

### Slide 5 — Rust Async Multiplexes One Thread

**Teaching purpose**
Explain Rust async without assuming language background: it lets one runtime thread switch among many unfinished operations instead of blocking on the first one that cannot progress.

**Audience takeaway**
Async in Rust is cooperative multiplexing. A future gets polled; if it returns `Pending`, the runtime parks that future, runs other ready work on the same thread, and polls the parked future again after a wakeup.

**Layout**
Use a simple two-column slide. Left column teaches the generic runtime behavior: one thread polls work A; A returns `Pending`; the runtime runs ready work B/C; a wakeup makes A pollable again. Right column maps that behavior to this encode path: `Encoder::Encode` is one message's future, `EncodedBytes.in_flight` stores the parked future, and `DsaAsyncProstEncode::poll` resumes encode when hardware progress is possible. Bottom thesis states the multiplexing rule.

**Exact visible text**
- Title: `5. Rust Async Multiplexes One Thread`
- Left title: `One runtime thread, many unfinished operations`
- Left flow labels: `poll encode A` / `Pending` / `run ready work B / C` / `wake A` / `poll A again`
- Left note: `The thread is reused; the unfinished future is stored, not waited on.`
- Right title: `Apply it to encode`
- Right bullets:
  - ``Encoder::Encode` is one message's encode future.`
  - ``Pending` parks that message's copy work.`
  - ``EncodedBytes.in_flight` stores the parked future.`
  - ``Ready(EncodeBuffer)` returns completed bytes to Tonic.`
- Bottom thesis: `Rust async multiplexes a thread across concurrent work: park the blocked encode, run other ready operations, then resume when hardware wakes it.`

**Visual treatment**
Keep this as a mechanism slide, not a code dump. The left multiplexing flow is the dominant visual object. Use orange for `Pending`, green for runnable/woken work, blue for the runtime thread and stored future. No extra cards beyond a light box around the flow if needed for readability.

**Speaker message**
`Rust async is cooperative multiplexing: a task that cannot progress returns Pending, and the runtime uses the same thread for other ready work until a wakeup brings the task back.`

**Transition**
Next define the Rust words `borrow` and `own` before using them to justify the async encode design.

### Slide 6 — Why Async Forces Ownership: The 'static Constraint

**Teaching purpose**
Explain the precise compiler logic that makes ownership mandatory: Tonic's `'static` bound on the future forbids any borrows, forcing the encoder to take ownership of both the buffer and the message.

**Audience takeaway**
Rust's async runtimes need to move futures across thread boundaries. To support this, Tonic enforces `type Encode: Future + 'static`. A `'static` future cannot borrow any caller-local variables, so it must own all the data it uses.

**Layout**
Use a two-column logic comparison. Left column shows the trait boundary constraint: Tonic's `type Encode: Future + 'static` requirement and the scheduling reason. Right column shows the code signature conflict: trying to return a future that borrows a buffer (rejected by E0759) versus passing the buffer by value (accepted). Under the columns, list the exact 3-step logic chain and the bottom thesis.

**Exact visible text**
- Title: `6. Why Async Forces Ownership: The 'static Constraint`
- Left title: `Tonic constraint: Thread scheduling`
- Left trait code:
  ```rust
  type Encode: Future<...> + Send + 'static;
  ```
- Left note: `To multiplex threads, the future must be free to move out of the caller stack frame.`
- Right title: `The Compiler Logic`
- Right code (Conflict vs Solution):
  ```rust
  // 1. Rejected: Borrow
  fn encode(&mut self, dst: &mut Buffer) -> Future
  // E0759: borrow does not satisfy 'static bound

  // 2. Accepted: Own
  fn encode(self, dst: Buffer) -> Future + 'static
  ```
- Right note: `Passing the buffer by value satisfies 'static.`
- Logic chain:
  1. `Tonic requires type Encode: Future + 'static to support thread-safe scheduling.`
  2. `Rust's borrow checker enforces that a 'static future cannot contain references.`
  3. `Therefore, the future must own the buffer (dst) and the message (item) to survive.`
- Bottom thesis: `Tonic's 'static constraint leaves no choice: to suspend and multiplex, the future must own the data.`

**Visual treatment**
Use red highlighting for the rejected borrow signature and green for the accepted owned signature. Highlight the `'static` keyword to draw immediate attention.

**Speaker message**
`Tonic forces the future to be 'static because it must survive after the caller returns. The borrow checker rejects any reference across this boundary, leaving ownership as the only path.`

**Transition**
Next connect the owned future back to the concrete `DsaAsyncProstEncode` fields and the exact resume point inside protobuf encoding.

### Slide 7 — Storing the Resumable State

**Teaching purpose**
Detail the concrete struct layout and drop safety of `DsaAsyncProstEncode<T>`.

**Audience takeaway**
The driver stores the owned future in `EncodedBytes::in_flight`. The future's drop order is critical to memory safety: because pending payload state references the item and sink, the state must drain descriptors before they are dropped.

**Layout**
Two-column layout. Left column describes State Storage in `EncodedBytes::in_flight`. Right column shows the `DsaAsyncProstEncode<T>` struct definition. Bottom alert highlights drop order safety.

**Exact visible text**
- Title: `Storing the Resumable State`
- Left text: `The driver stores the owned future in EncodedBytes::in_flight. The stack-frame is long gone, so the future keeps all data, progress, and wakers alive.`
- Right code:
  ```rust
  pub struct DsaAsyncProstEncode<T> {
      // Owned state must survive .await:
      state: PollEncodeState<DsaSink>,
      item: Option<T>,
      sink: Option<DsaSink>,
      stage: DsaEncodeStage,
  }
  ```
- Bottom alert: `The future must own all data compartments to safely survive suspension crossing .await.`

**Visual treatment**
Structure-focused blue accents. The drop-order note should be visually separated.

**Speaker message**
`To safely suspend and resume, we store the owned future containing all message fields, driver buffer sinks, and completion state directly in the driver.`

**Transition**
Next show the synthesis of driver loop polling and Prost resumable checkpoints.

### Slide 8 — Synthesis: How it Works

**Teaching purpose**
Synthesize the interaction between the Tonic driver poll loop and the Prost resumable phase index.

**Audience takeaway**
Tonic driver polls the future; if pending, the thread is released. Prost uses the phase index to avoid re-writing prefix metadata when resuming later.

**Layout**
Two columns. Left column describes the driver poll loop. Right column shows the phase-checking resume logic code. Bottom alert captures the synthesis of owned state and exact resume points.

**Exact visible text**
- Title: `Synthesis: How it Works`
- Left text: `1. Driver Poll Loop: Tonic driver polls the future. If Pending, the thread is released. When completion fires, the waker schedules another poll.`
- Right code:
  ```rust
  if state.phase() == 0 {
      pb_write_tag(dst, tag)?;
      pb_write_varint(dst, len)?;
      state.set_phase(1); // Set checkpoint!
  }
  // Resumes here if phase == 1:
  poll_copy_payload(dst, state, cx)
  ```
- Bottom alert: `Async encode = owned resumable state + exact protocol resume points.`

**Visual treatment**
Contrast driver execution steps on the left with exact compiler resume checkpoints on the right.

**Speaker message**
`The driver poll loop drives execution while the generated encoder phase checks prevent re-running completed synchronous writes when resuming.`

**Transition**
Next, drill down into how the Prost codec split-phase serialization separates structure and payload steps.

### Slide 9 — Split-Phase Serialization: CPU vs Async

**Teaching purpose**
Teach how Protobuf encoding is split into CPU-written structure metadata and asynchronously offloaded payload copies.

**Audience takeaway**
Large bytes and string payloads are copied asynchronously via DSA/IAX while small metadata headers (tags, wire types, lengths) and primitive scalars are synchronously written by the CPU.

**Layout**
Two-column layout. Left column defines the CPU structure phase. Right column defines the offloaded payload phase. Bottom alert emphasizes the CPU/hardware split.

**Exact visible text**
- Title: `Split-Phase Serialization: CPU vs Async`
- Left title: `CPU Structure Phase`
- Left text: `Protobuf metadata and scalar fields are written synchronously by the CPU directly to the mutable buffer: tags, wire types, length delimiters, and primitive scalars.`
- Right title: `Offloaded Payload Phase`
- Right text: `Large contiguous byte arrays and strings are offloaded asynchronously to hardware: dynamic selection falls back to CPU for small copies; large copies submit to DSA/IAX and return Pending.`
- Bottom alert: `Protobuf encoding is split: CPU writes structure synchronously, while hardware copies payloads asynchronously.`

**Visual treatment**
Gray accents for CPU metadata, bright orange/violet accents for the offloaded payload path.

**Speaker message**
`We split protobuf encoding: the CPU handles cheap metadata formatting, while the expensive memory movement is offloaded asynchronously.`

**Transition**
Next show how the state cursor keeps track of this split-phase progress.

### Slide 10 — State Tracking & Hierarchical Nesting

**Teaching purpose**
Detail the structure and recursion stack mechanics of `PollEncodeState<S>`.

**Audience takeaway**
`PollEncodeState` tracks the active field, repeated field indices, execution phase, and nesting frames to successfully resume serialization at the exact byte boundary of sub-messages.

**Layout**
Two columns. Left column describes the state cursor components and nesting stack. Right column displays the definitions of `PollEncodeState<S>` and `PollEncodeFrame`. Bottom alert highlights sub-message recursion management.

**Exact visible text**
- Title: `State Tracking & Hierarchical Nesting`
- Left text: `To resume serialization at the exact byte boundary after a Pending yield, PollEncodeState tracks: field index, iteration index, execution phase, and a nested frame stack for hierarchical parent message context.`
- Right code:
  ```rust
  pub struct PollEncodeState<S: AsyncEncodeTarget> {
      root: PollEncodeFrame,
      nested: Vec<PollEncodeFrame>,
      depth: usize,
      payload: S::PayloadState,
  }
  struct PollEncodeFrame {
      field: usize, index: usize, phase: u8,
  }
  ```
- Bottom alert: `Hierarchical messages enter a nested state: push parent frame, poll sub-message, and pop on completion.`

**Visual treatment**
Violet accents to represent resumable state metadata.

**Speaker message**
`The state cursor uses a push/pop stack frame to manage sub-messages, allowing deep recursion without losing the suspension checkpoint.`

**Transition**
Finally, examine the generated code pattern of the compiler's resumable state machine.

### Slide 11 — The Generated Resumable State Machine

**Teaching purpose**
Examine a concrete generated code example inside `poll_encode_raw`.

**Audience takeaway**
The compiler-generated encoder checks the field index and phase checkpoint to avoid re-writing headers and only resume the pending payload copies.

**Layout**
Two-column layout. Left column details the step-by-step resume flow (Branch, Phase Checkpoint, Yield/Resume). Right column displays the generated code pattern. Bottom alert reinforces metadata single-write correctness.

**Exact visible text**
- Title: `The Generated Resumable State Machine`
- Left text: `1. Branch to Field: state.field() selects the active field. 2. Phase Checkpoint: if phase is 0, CPU writes tag and length, then sets phase to 1. 3. Yield/Resume: poll_write_payload yields Pending or advances on completion.`
- Right code:
  ```rust
  if state.field() == 0 {
      if state.phase() == 0 {
          let mut buf = sink.buf_mut();
          encode_key(1, WireType::LengthDelimited, &mut buf);
          encode_varint(self.name.len() as u64, &mut buf);
          state.set_phase(1);
      }
      match sink.poll_write_payload(self.name.as_bytes(),
                                    state.payload_pin_mut(), cx) {
          Poll::Ready(Ok(())) => {
              state.set_phase(0);
              state.advance_field();
          }
          Poll::Pending => return Poll::Pending,
      }
  }
  ```
- Bottom alert: `Phase transitions ensure that metadata prefixes are written exactly once, even across multiple yields.`

**Visual treatment**
Green highlight for successful/complete paths, violet for phase states, code-focused presentation.

**Speaker message**
`The generated code uses phase checkpoints to ensure metadata is written exactly once, jumping directly to the payload poll on resume.`
