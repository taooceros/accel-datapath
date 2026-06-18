#import "../../template.typ": *

// ─── Theme Metadata Config (Global Show Rules) ───
#show: deck.with(
  margin: (x: 45pt, y: 35pt),
  size: 15.5pt, // Larger baseline font size
  leading: 0.9em,
  spacing: 0.8em,
  footer: [#text(size: 9.5pt, fill: palette.muted)[Async Tonic Encode: An Ownership Redesign]],
)

// Global page & block style overrides
#set page(fill: rgb("#f7faff"))

// Customize how code blocks look globally (no borders, simple shaded background)
#show raw.where(block: true): it => block(
  width: 100%,
  radius: 6pt,
  inset: 12pt,
  fill: rgb("#f8fafc"),
)[
  #set text(font: "DejaVu Sans Mono", size: 12pt, fill: luma(34))
  #it
]

// Custom helpers for visual layout (no structural borders)
#let side-by-side(left, right, ratio: (1fr, 1fr)) = grid(
  columns: ratio,
  gutter: 30pt,
  left,
  right
)

#let section(title, body) = block(
  width: 100%,
  inset: (y: 5pt),
)[
  #text(size: 17pt, weight: "bold", fill: palette.title)[#title]
  #v(0.3em)
  #body
]

#let alert-text(body) = block(
  width: 100%,
  inset: (left: 10pt),
  stroke: (left: 3.5pt + rgb("#f97316")),
)[#text(size: 15pt, fill: luma(40))[#body]]

#let checkmark = text(fill: rgb("#16a34a"), weight: "bold")[✓]
#let cross = text(fill: rgb("#dc2626"), weight: "bold")[✗]
#let arrow = text(size: 16pt, weight: "bold", fill: palette.muted)[→]

// ─── Slides ───

= Part I: The Synchronous Baseline and Limits

== Original Tonic Send Path

#v(0.6em)
#side-by-side[
  #section("The Layer Sequence", [
    1. *Application message* is created.
    2. *Prost* encodes the protobuf body.
    3. *Tonic* prepends the gRPC frame header.
    4. *HTTP/2* transfers the frame bytes.
  ])
][
  #section("Tonic's Classical API", [
    ```rust
    // Synchronous execution on the CPU thread
    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut EncodeBuf
    ) -> Result<(), Self::Error>;
    ```
  ])
]

#v(1.2em)
#align(center)[_This is the path we start from before introducing accelerator copies._]


// Slide 2
== Where the Copy Cost Appears

#v(0.6em)
#side-by-side[
  #section("Inside Prost Encode", [
    - *Write tags and lengths*: Tiny varint writes (very cheap, fits in CPU registers).
    - *Copy payload bytes*: Large contiguous memory copies (very expensive, CPU stalls on bus).
  ])
][
  #section("Prost Code Pattern", [
    ```rust
    // CPU writes tags/lengths directly,
    // then performs costly memory copy:
    pb_write_tag(dst, tag)?;
    pb_write_varint(dst, payload.len())?;
    dst.put_slice(&payload); // <- CPU block copy
    ```
  ])
]

#v(1.2em)
#alert-text([We target the payload memory copy, leaving structured field writes to the CPU.])


// Slide 3
== Async Means Multiplexing

#v(0.6em)
#side-by-side[
  #section("Blocking vs Multiplexing", [
    - *Blocking:* CPU thread is idle, locked on hardware completion.
    - *Multiplexing:* CPU submits copies and switches immediately.
  ])
][
  #section("Submission & Poll Logic", [
    ```rust
    // Async offload pattern:
    let fut = dsa_device.submit_copy(src, dst);
    // CPU thread is now free to poll other tasks!
    ```
  ])
]

#v(1.2em)
#alert-text([Async encode is a multiplexing mechanism to keep accelerator lanes fully occupied.])
// Slide 4
= Part II: Multiplexing and Thread Re-use

== How Async Encode Multiplexes Work

#v(1em)
#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 20pt,
  section("1. Start Work", [
    Submit payload copy to DSA/IAX. Tonic moves the message and output buffer into one owned encode operation.
  ]),
  section("2. Pending", [
    This stream cannot finish now, so it returns `Pending` to give the runtime thread permission to poll other work.
  ]),
  section("3. Resume", [
    When the hardware completes, the waker is notified. Tonic polls the operation again and finishes the gRPC frame.
  ]),
)

#v(2em)
#alert-text(
  [When encode returns `Pending`, the executor runtime thread is released immediately to multiplex other ready operations.],
)


// Slide 5
== Rust Async Multiplexes One Thread

#v(1em)
#side-by-side[
  #section("Cooperative Scheduling Loop", [
    1. Poll encode future A #arrow returns `Pending`.
    2. Runtime thread runs ready operations B or C.
    3. Hardware completion interrupt fires for A.
    4. Runtime thread polls future A again #arrow returns `Ready`.
  ])
][
  #section("Tonic Stream Implementation", [
    - `Encoder::Encode` is one message's encode future.
    - `Pending` parks the message's copy work.
    - `EncodedBytes.in_flight` stores the parked future.
    - `Ready(EncodeBuffer)` returns completed bytes to Tonic.
  ])
]

#v(2em)
#alert-text([Rust async multiplexes a thread: park the blocked encode, run ready operations, then resume.])


= Part III: Memory Safety and Ownership

== Why Async Forces Ownership: The Multiplexing Reality

#v(0.8em)
#side-by-side[
  #section("The Multiplexing Requirement", [
    To keep slow hardware lanes fully occupied, a single CPU thread must switch between many concurrent requests.

    - The encode future *cannot* live on the caller's stack frame.

    - It must be stored in the driver to survive suspension.

    - This forces a `'static` lifetime bound on the future.
  ])
][
  #section("The Compiler Logic", [
    ```rust
    // 1. Sync Signature: Borrowed Stack Buffer
    fn encode(
      &mut self,
      item: Item,
      dst: &mut EncodeBuf
    ) -> Result<(), Error>;

    // 2. Async Signature: Owned Buffer & Future
    fn encode(
      self: Pin<&mut Self>,
      item: Self::Item,
      dst: EncodeBuffer,
    ) -> Result<Self::Encode, Self::Error>;
    // where Self::Encode: Future + 'static
    ```
  ])
]

#alert-text([To park a request and multiplex the thread, the future must own its buffer and message.])


// Slide 7
== Storing the Resumable State

#v(0.6em)
#side-by-side[
  #section("State Storage", [
    The driver stores the owned future in `EncodedBytes::in_flight`.

    The stack-frame is long gone, so the future keeps all data, progress, and wakers alive.
  ])
][
  #section("Owned Future Fields", [
    ```rust
    pub struct DsaAsyncProstEncode<T> {
        // Owned state must survive .await:
        state: PollEncodeState<DsaSink>,
        item: Option<T>,
        sink: Option<DsaSink>,
        stage: DsaEncodeStage,
    }
    ```
  ])
]

#v(1.2em)
#alert-text([The future must own all data compartments to safely survive suspension crossing `.await`.])


// Slide 8
== Synthesis: How it Works

#v(0.6em)
#side-by-side[
  #section("1. Driver Poll Loop", [
    Tonic driver polls the future. If `Pending`, the thread is released.
    When completion fires, the waker schedules another poll.
  ])
][
  #section("2. Prost Resumable State", [
    Prost checks the phase index to avoid rewriting tag/length metadata when resuming.

    ```rust
    if state.phase() == 0 {
        pb_write_tag(dst, tag)?;
        pb_write_varint(dst, len)?;
        state.set_phase(1); // Set checkpoint!
    }
    // Resumes here if phase == 1:
    poll_copy_payload(dst, state, cx)
    ```
  ])
]


#v(1.2em)
#alert-text([Async encode = owned resumable state + exact protocol resume points.])


= Part IV: Turning Prost Async: Resumable Serialization

// Slide 9
== Split-Phase Serialization: CPU vs Async

#v(0.6em)
#side-by-side[
  #section("CPU Structure Phase", [
    Protobuf metadata and scalar fields are written synchronously by the CPU directly to the mutable buffer.

    - *Tags and wire types*: Compact varints (1-5 bytes).
    - *Length delimiters*: Size prefixes for nested messages, strings, and bytes.
    - *Scalars*: Integers, floats, and booleans.
  ])
  :][
  #section("Offloaded Payload Phase", [
    Large contiguous byte arrays and strings are offloaded asynchronously, allowing the CPU thread to yield.

    - *Dynamic selection*: Small payloads fall back to CPU memory copies immediately.
    - *Async copy*: Large payloads submit to the DSA/IAX work queue and return `Pending`.
  ])
]

#v(1.2em)
#alert-text(
  [Protobuf encoding is split: CPU writes structure synchronously, while hardware copies payloads asynchronously.],
)


// Slide 10
== State Tracking & Hierarchical Nesting

#v(0.6em)
#side-by-side[
  #section("State Cursor & Nesting Stack", [
    To resume serialization at the exact byte boundary after a `Pending` yield, `PollEncodeState` tracks:
    - *field*: The index of the current field in the message.
    - *index*: The iteration index for repeated/packed fields.
    - *phase*: The execution checkpoint within a single field.
    - *nested*: A frame stack tracking parent message contexts during recursion.
  ])
  :][
  #section("Nesting and Recursion State", [
    ```rust
    pub struct PollEncodeState<S: AsyncEncodeTarget> {
        root: PollEncodeFrame,        // Current message cursor
        nested: Vec<PollEncodeFrame>, // Stack for parent messages
        depth: usize,                 // Current depth level
        payload: S::PayloadState,     // Hardware copy state
    }

    struct PollEncodeFrame {
        field: usize,
        index: usize,
        phase: u8,
    }
    ```
  ])
]

#v(1.2em)
#alert-text([Hierarchical messages enter a nested state: push parent frame, poll sub-message, and pop on completion.])


// Slide 11
== The Generated Resumable State Machine

#v(0.6em)
#side-by-side[
  #section("Resume Execution Flow", [
    The generated `poll_encode_raw` uses a flat state machine:
    - *Branch to Field*: Select field via `state.field()`.
    - *Phase Checkpoint*: Check `state.phase()`. If `0`, perform CPU metadata writes and advance phase to `1`.
    - *Yield/Resume*: Call `poll_write_payload`. If `Pending`, yield. On wakeup, resume directly at phase `1` (skipping metadata writes).
  ])
  :][
  #section("Generated Code Pattern", [
    ```rust
    if state.field() == 0 { // string field
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
  ])
]

#v(1.2em)
#alert-text([Phase transitions ensure that metadata prefixes are written exactly once, even across multiple yields.])
