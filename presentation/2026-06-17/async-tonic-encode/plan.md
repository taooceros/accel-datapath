# Slide plan: async Tonic encode

## Context

- **Goal:** Teach the actual design change behind async Tonic encode: a stack-local borrowed-buffer encode call became one owned future stored by the Tonic body driver until `Ready` returns `EncodeBuffer`.
- **Audience:** Systems/Rust researchers who know async Rust, Tonic/gRPC, Prost, and accelerator motivation, but have not internalized this repo's async encode boundary.
- **Source grounding:**
  - `accel-rpc/tonic/tonic/src/codec/mod.rs`: `Encoder::Encode: Future<Output = Result<EncodeBuffer, Error>> + Send + 'static`; `encode(self: Pin<&mut Self>, item, dst: EncodeBuffer) -> Result<Self::Encode, Error>`.
  - `accel-rpc/tonic/tonic/src/codec/encode.rs`: `EncodedBytes` stores `in_flight: Option<T::Encode>` and `in_flight_offset`; reserves the gRPC header before starting encode; polls the future; finishes compression/header/size after `Ready`.
  - `accel-rpc/tonic/tonic-dsa-bytes-async/src/prost_codec.rs`: `DsaAsyncProstEncode<T>` owns `state`, `item`, `sink`, `options`, and `stage`; field drop order is part of the safety contract.
  - `accel-rpc/tonic/prost/prost/src/message.rs` and `transfer.rs`: `Message::poll_encode_raw`, `AsyncEncodeTarget`, and `PollEncodeState` split CPU protobuf structure writes from suspendable payload copies.
  - `accel-rpc/tonic/prost/prost-derive/src/field/scalar.rs`: generated async encoding writes tag + length, sets a pending phase, then polls payload copy; a later poll resumes the payload path rather than rewriting the prefix.

## Story spine

1. Start with the conversion, not with DSA: borrowed sync call → owned async future → returned `EncodeBuffer`.
2. Explain why sync was simple: completed body bytes existed before Tonic framed anything.
3. Show the ownership failure: a borrowed async future cannot become `T::Encode: Send + 'static` after `Pending`.
4. Introduce the actual Tonic boundary: the body driver stores one future per in-flight message and polls it later.
5. Walk the driver lifecycle in code order: reserve header, move buffer into encode, store `in_flight`, poll, finish frame.
6. Open the stored future: it owns message, sink/buffer, Prost state, options, stage, offload/completion/drop cleanup.
7. Zoom into Prost state: Tonic polls coarsely; `PollEncodeState` records field/index/phase/payload continuation.
8. Make the checkpoint concrete with one length-delimited field suspended after tag+length and before payload completion.
9. Show why DSA/IAX matters only as a consequence: raw addresses can outlive the call, so the stored future must own source, destination, and completion cleanup.
10. Close with the reusable rule and a cautious measurement framing.

## Visual language

- **Blue:** Tonic API/driver ownership.
- **Green:** safe completion, returned buffer, final reusable rule.
- **Orange:** `Pending`, offload, and hardware consequence.
- **Red:** invalid borrowed suspension or memory-safety hazard.
- **Violet:** Prost byte-level resume state.
- **Gray:** gRPC framing/protocol mechanics.
- Touying usage: each slide is one `#slide` with one dominant layout object; wide flows use grid/composer-style columns; visible text is kept large; no speaker-note/takeaway footer is used. Any essential teaching sentence is promoted into the main slide body.

## Slide-by-slide blueprint

### Slide 1 — Async Encode Was an Ownership Redesign

**Teaching purpose**
Make the old→new contract the first read.

**Audience takeaway**
Async Tonic encode is not “add `.await`”; it changes who owns the in-flight message state.

**Layout**
Header. Top thesis callout. Center old→new conversion with two large cards and one orange `Pending can happen` bridge. Lower compact memory hook with `Tonic stores` and `Ready returns`; no bottom strip.

**Exact visible text**
- Thesis: `Async encode was an ownership redesign.`
- Old card: `SYNC` / `encode(item, &mut EncodeBuf)` / `borrow caller buffer` / `return after body bytes exist`
- Bridge: `Pending can happen`
- New card: `ASYNC` / `encode(item, EncodeBuffer) -> T::Encode` / `move buffer into future` / `Ready returns EncodeBuffer`
- Memory hook: `Tonic stores one owned T::Encode per in-flight message.` / `Prost owns exact field/tag/length/payload resume state inside it.`

**Visual treatment**
The conversion dominates. DSA/IAX is not visible on this slide; the audience should first learn the ownership boundary.


**Transition**
Now show why the old sync shape was safe.

### Slide 2 — Sync Encode: Complete Body, Then Frame

**Teaching purpose**
Teach the completed-buffer contract that sync Tonic relied on.

**Audience takeaway**
In sync encode, Tonic frames only after body bytes already exist and the `&mut EncodeBuf` borrow is gone.

**Layout**
Dominant horizontal flow. Small code evidence strip below. The sync-safety sentence is visible in the main body; no bottom strip.

**Exact visible text**
- Flow title: `Completed-buffer contract`
- Flow: `message` → `borrow &mut EncodeBuf` → `write full body` → `return complete bytes` → `Tonic frames`
- Code evidence: `fn encode(&mut self, item, dst: &mut EncodeBuf) -> Result<()>`
- Detail: `After return: size-check, optional compression, flags + length, yield frame.`

**Visual treatment**
Green marks the safe completed bytes; gray marks Tonic framing.


**Transition**
Ask what happens when encode can return `Pending` before body bytes exist.

### Slide 3 — Borrowed Async Encode Cannot Be Stored

**Teaching purpose**
Show why `async fn encode(&mut self, ..., &mut EncodeBuf)` is the wrong shape.

**Audience takeaway**
A future that borrows caller-stack `&mut` values cannot fill Tonic's `T::Encode: Send + 'static` storage slot.

**Layout**
Top wrong-signature evidence. Dominant red failure band: caller stack → `Pending` → borrowed future → Tonic storage slot rejects it. The correct storage-unit contrast is visible in the main body.

**Exact visible text**
- Wrong signature: `async fn encode(&mut self, item, dst: &mut EncodeBuf) -> Result<()>`
- Red headline: `Failure at Pending`
- Failure flow: `caller stack owns &mut borrows` → `future returns Pending` → `future still borrows dst/self` → `cannot store as T::Encode: Send + 'static`
- Correct contrast: `Correct storage unit: owned future + owned EncodeBuffer.`

**Visual treatment**
Red failure band dominates; code is only the setup.


**Transition**
Show the API that encodes this ownership cutover.

### Slide 4 — The New Tonic Boundary

**Teaching purpose**
Introduce the exact API and driver storage field.

**Audience takeaway**
`encode` moves the buffer into a future; the body driver stores exactly that future until it returns the buffer.

**Layout**
Two-column Touying-style split: left API evidence, right driver slot evidence. Bottom lifecycle sentence.

**Exact visible text**
- Left title: `Encoder API`
- Code: `type Encode: Future<Output = Result<EncodeBuffer, Error>> + Send + 'static`
- Code: `fn encode(self: Pin<&mut Self>, item, dst: EncodeBuffer) -> Result<Self::Encode, Error>`
- Right title: `Body driver slot`
- Code: `in_flight: Option<T::Encode>`
- Code: `in_flight_offset: Option<usize>`
- Lifecycle sentence: `start: move item + buffer in` / `poll: Pending keeps future stored` / `ready: buffer returns`

**Visual treatment**
Blue for API and driver ownership; green for `Ready`.


**Transition**
Walk the driver in the order it actually executes.

### Slide 5 — Driver Lifecycle: Reserve, Store, Poll, Finish

**Teaching purpose**
Make `EncodedBytes` behavior concrete without drowning in code.

**Audience takeaway**
Tonic reserves protocol space before encode, gives ownership of the active buffer to the future, and only fills the gRPC header after completion.

**Layout**
One wide five-step process. Under it, one small code-anchor row naming source lines/fields plus an in-body frame-finalization core point; no bottom strip.

**Exact visible text**
- Flow title: `One message through EncodedBytes`
- Steps: `reserve 5-byte header gap` → `take active BytesMut into EncodeBuffer` → `store in_flight = Some(T::Encode)` → `poll Pending / Ready` → `finish compression + header`
- Code anchors: `buf.reserve(HEADER_SIZE)` / `encoder.encode(item, dst)` / `ready!(encode.poll(cx))` / `finish_encode_buffer(...)`

**Visual treatment**
A single pipeline is dominant. Gray header gap and final framing bookend the blue future step.


**Transition**
Open the future to show what state it owns.

### Slide 6 — What Lives Inside `T::Encode`

**Teaching purpose**
Show the stored future as the owner of all state that must survive suspension.

**Audience takeaway**
The stored future owns item/message, sink/buffer, Prost poll state, options/stage, and offload cleanup; this is why drop order matters.

**Layout**
Dominant cutaway box labelled `DsaAsyncProstEncode<T>`. Six compartments inside. Red safety note below.

**Exact visible text**
- Cutaway title: `DsaAsyncProstEncode<T>` / `the object Tonic stores`
- Compartments: `state: PollEncodeState` / `item: Option<T>` / `sink: DsaProstEncodeSink` / `options` / `stage` / `descriptor + completion cleanup`
- Readiness footer inside cutaway: `Pending: keep all compartments` / `Ready: sink.into_inner() returns EncodeBuffer`
- Red note: `Drop order is part of the safety contract: pending payload state may reference item bytes and destination storage.`

**Visual treatment**
Blue outer future, violet Prost state, orange offload/completion, red safety note.


**Transition**
Zoom into the Prost state compartment.

### Slide 7 — Prost Records the Byte Resume Point

**Teaching purpose**
Explain the granularity split: Tonic owns coarse polling; Prost owns precise protobuf continuation.

**Audience takeaway**
`PollEncodeState` stores field/index/phase plus payload state so a later poll resumes exactly where the wire-format encoder stopped.

**Layout**
Two-level map: Tonic poll slot above, `T::Encode` middle, Prost state nested inside. Right side shows `PollEncodeFrame` fields.

**Exact visible text**
- Top sentence: `Tonic polls one future; Prost remembers the byte-position checkpoint inside it.`
- Layer labels: `Tonic body driver` / `owned T::Encode future` / `PollEncodeState`
- State fields: `field` / `index` / `phase` / `payload state`
- Detail: `CPU writes keys, lengths, scalars immediately.` / `Payload copy may return Pending.`

**Visual treatment**
Nested boxes, not peer cards: violet must visibly live inside green/blue future.


**Transition**
Make that checkpoint concrete with one field.

### Slide 8 — One Suspended Length-Delimited Field

**Teaching purpose**
Show the exact legal resume path after a payload-copy `Pending`.

**Audience takeaway**
If tag and length are already emitted, the next poll must resume payload copy only; rewriting prefix corrupts the protobuf stream.

**Layout**
Dominant violet checkpoint timeline with green legal path and red wrong-resume labels underneath.

**Exact visible text**
- Timeline title: `Checkpoint after payload copy returns Pending`
- Timeline: `field N selected` → `tag emitted` → `length emitted` → `payload copy Pending` → `next poll resumes payload only`
- Red labels: `wrong field: skip/repeat` / `wrong phase: duplicate tag` / `wrong phase: duplicate length` / `wrong offset: corrupt payload`

**Visual treatment**
The green legal path is the main read. Red labels attach under the exact timeline positions they corrupt.


**Transition**
Explain why offload makes this ownership physically necessary.

### Slide 9 — Offload Makes Ownership Physical

**Teaching purpose**
Place DSA/IAX as the consequence, not the main architecture.

**Audience takeaway**
When hardware may keep raw source/destination addresses after the Rust call returns, the owned future must keep memory and completion cleanup alive.

**Layout**
Top red thesis. Center owned-future container with source, destination, descriptor/completion. Small orange DSA/IAX tag. Bottom three invariants.

**Exact visible text**
- Thesis: `Pending offload means raw addresses can outlive the encode call.`
- Container: `owned T::Encode holds the physical state` / `source bytes` / `EncodeBuffer storage` / `descriptor + completion`
- DSA tag: `DSA/IAX is the reason Pending can outlive the call.`
- Invariants: `source valid` / `destination stable` / `completion drained on Ready or drop`

**Visual treatment**
Orange is a consequence accent only; blue owned-future container remains dominant.


**Transition**
Close with the general rule and how to evaluate performance.

### Slide 10 — Reusable Rule

**Teaching purpose**
Generalize the design into a rule that applies beyond this encoder.

**Audience takeaway**
Stored/polled operations must own all state needed after suspension; benchmarks only decide whether the offload tradeoff is worth it.

**Layout**
Top large rule. Middle three ownership cards: Tonic, Future, Prost. Bottom measurement caveat and final sentence.

**Exact visible text**
- Rule: `If an operation may be stored and polled after the call returns, anything needed after suspension must be owned by that operation.`
- Cards: `Tonic owns frame boundary` / `T::Encode owns suspended message work` / `Prost owns wire-format checkpoint`
- Measurement caveat: `Architecture removes one staged CPU payload copy opportunity; benchmarks decide whether offload overhead, wakeup, and resume bookkeeping are smaller.`
- Final sentence: `Async encode = owned resumable state + exact protocol resume points.`

**Visual treatment**
Rule is the largest text in the deck. Measurement is deliberately secondary.


**Transition**
None.
