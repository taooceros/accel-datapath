// Prototype tutorial deck: Tokio / Rust async fundamentals
// Reader: advisor / systems collaborator.
// Claim boundary: general tutorial, no project hardware or performance claims.
// Sources:
// - docs/plan/2026-05-02/06.tokio-general-tutorial-deck.plan.md
// - docs/report/research/002.async_mechanism_description_sources.md
// - Eric Niebler, "What are senders good for anyway?", 2024

#import "../template.typ": callout, card, deck, note, palette, panel
#import "@preview/mmdr:0.2.2": mermaid
#import "@preview/zebraw:0.6.3": *

#show: deck.with(
  margin: (x: 40pt, y: 30pt),
  size: 13.1pt,
  leading: 0.84em,
  spacing: 0.58em,
)



#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red
#let c-row = palette.row

#let box-step(label, body, fill: c-row, accent: c-accent, body-size: 9.7pt) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 9pt, y: 7pt),
  fill: fill,
  stroke: 0.5pt + palette.border,
)[
  #align(center)[#text(weight: "bold", fill: accent)[#label]]
  #v(0.2em)
  #align(center)[#text(size: body-size, fill: luma(65))[#body]]
]


#let arrow = align(center + horizon)[#text(size: 19pt, fill: c-accent)[→]]


= Introduction to async and Tokio

#align(center + horizon)[

  #text(
    size: 15pt,
    fill: luma(90),
  )[`async fn` creates a `Future`; Tokio polls it, parks it when it returns `Pending`, and wakes it when I/O or timers may progress.]
]

#v(0.65em)

#grid(
  columns: (1fr, 0.12fr, 1fr, 0.12fr, 1fr, 0.12fr, 1fr),
  gutter: 4pt,
  [#box-step([Rust async], [`Future` + `.await`], fill: c-blue)],
  [#arrow],
  [#box-step([executor], [polls futures], fill: c-green)],
  [#arrow],
  [#box-step([Waker], [requeue task], fill: c-orange)],
  [#arrow],
  [#box-step([Tokio], [runtime + scheduler], fill: c-row)],
)

#v(0.55em)

#callout(fill: c-blue, stroke: c-accent)[
  Rust defines the async contract (`Future`, `.await`, `Waker`); Tokio supplies the runtime machinery: ready queues, worker threads, timers, and I/O events.
]

#pagebreak()

= Problem 1: Blocking calls do not scale to many waits

== Blocking APIs are simple until many calls wait

#callout(fill: c-blue, stroke: c-accent)[
  The traditional API story starts with blocking calls: code is easy to read because the stack itself remembers where execution should continue.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Blocking style: one flow, one stack]
    #v(0.3em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```c
      int n = read(fd, buf, cap);
      write(fd, buf, n);
      ```,
    )
    #v(0.35em)
    While `read` waits, the OS thread is parked: it keeps a stack, occupies scheduler state, and must be woken later.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Potential solution: multiplex one worker]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.12fr, 1fr, 0.12fr, 1fr),
      gutter: 5pt,
      [#box-step([many fds], [connections], fill: white)],
      [#arrow],
      [#box-step([event loop], [poll events], fill: c-blue)],
      [#arrow],
      [#box-step([dispatch], [only ready work], fill: white)],
    )
    #v(0.45em)
    One worker can run whichever operation can make progress instead of blocking on one fd.
  ]],
)

#v(0.5em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 10pt,
  [#card(
    [Blocking API],
    [Sequential and readable; the call stack stores the continuation.],
    fill: c-row,
    body-size: 10pt,
  )],
  [#card(
    [Many waiting threads],
    [
      - Thread creation costs; stacks consume memory;
      - wakeups and context switches go through the kernel scheduler.
    ],
    fill: c-red,
    body-size: 9.6pt,
  )],
  [#card(
    [Solution: Multiplexing],
    [Use fewer workers for many waits; move each paused operation's continuation out of the OS thread stack.],
    fill: c-green,
    body-size: 9.6pt,
  )],
)

#callout[
  Interrupt is slow, so RDMA/dpdk use polling instead of interrupts. Blocking is not even an option.
]

#pagebreak()

== Multiplexing creates a continuation problem

#callout(fill: c-blue, stroke: c-accent)[
  Once one thread serves many operations, the stack can no longer be the only place that remembers every paused operation.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[What the event loop knows]
    #v(0.35em)
    #box(height: 50%)[
      #mermaid(
        "graph TD; A[epoll / I/O driver] --> B[poll events]; B --> C[I/O event]; C --> D[which operation?]; D --> E[run continuation];",
        base-theme: "default",
        theme: (background: "transparent"),
      )
    ]

  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What each operation must keep]
    #v(0.35em)
    #box-step([where was I?], [next instruction / state], fill: c-blue)
    #v(0.3em)
    #box-step([what did I keep?], [locals, buffers, offsets], fill: c-orange)
    #v(0.3em)
    #box-step([who calls me?], [callback or waker], fill: c-green)
  ]],
)

#note[
  This is the suspension problem. C callbacks, Go goroutines, and Rust/C++ stackless coroutines are different ways to represent the paused continuation.
]

#pagebreak()

= Problem 2: Paused work needs a continuation

Multiplexing frees the worker thread, but it also removes the thread stack as the place that remembers each operation's progress.


== C callbacks make the continuation explicit

#callout(fill: c-blue, stroke: c-accent)[
  In manual event-loop code, the programmer moves the needed locals into a context struct, starts an async operation, and passes a callback as “the rest of the function.”
]

#grid(
  columns: (0.78fr, 1.22fr),
  gutter: 12pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Manual transformation]
    #v(0.35em)
    #box-step([locals], [fields in `Ctx`], fill: c-orange)
    #v(0.35em)
    #box-step([next line], [callback function], fill: c-blue)
    #v(0.35em)
    #box-step([wake path], [event loop calls callback], fill: c-green)
    #v(0.45em)
    The callback is the continuation: it receives the context and starts the next async operation.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Read, then write, in callback style]
    #v(0.2em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```c
      typedef struct { int fd; char *buf; } Ctx;

      void start(Ctx *c) {
        read_async(c, on_read);   // returns now
      }

      void on_read(Ctx *c) {      // callback
        write_async(c, on_write);
      }
      ```,
    )
  ]],
)

#pagebreak()

== Problem: Composability

#callout(fill: c-blue, stroke: c-accent, inset: (x: 12pt, y: 8pt))[
  Different async providers have different callback protocols. The programmer must hand-glue them together.
]

#panel(fill: c-row, inset: (x: 12pt, y: 9pt))[
  #text(weight: "bold", fill: c-title)[The logical operation is simple]
  #v(0.3em)
  #grid(
    columns: (1.12fr, 0.12fr, 0.9fr, 0.12fr, 0.9fr),
    gutter: 5pt,
    [#box-step([race], [`read_file` vs 100 ms timer], fill: c-orange, body-size: 8.8pt)],
    [#arrow],
    [#box-step([then parse], [CPU pool], fill: c-blue, body-size: 8.8pt)],
    [#arrow],
    [#box-step([then report], [event loop], fill: c-green, body-size: 8.8pt)],
  )
  #v(0.3em)
  #align(center)[#text(
    size: 9.2pt,
    fill: luma(80),
  )[One operation: sequencing + race + scheduler hops + errors + cleanup.]]
]

#v(0.45em)

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  [#panel(fill: c-green, inset: (x: 12pt, y: 9pt))[
    #text(weight: "bold", fill: c-title)[What the programmer must write]
    #v(0.15em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```c
      LoadCtx *ctx = new LoadCtx(...); // owns buf
      read_file(f, buf, n, &ctx->io, on_read);
      start_timer(loop, 100, on_timeout, ctx);

      on_read(...) {
        if (!claim_winner(ctx)) return;
        post_work(pool, parse, ctx);
      }
      on_timeout(...) {
        if (!claim_winner(ctx)) return;
        cancel_read(&ctx->io);
      }
      ```,
    )
  ]],
  [#panel(fill: c-row, inset: (x: 12pt, y: 9pt))[
    #text(weight: "bold", fill: c-title)[Why this hurts the programmer]
    #v(0.3em)
    #grid(
      columns: (1fr, 1fr),
      gutter: 6pt,
      [#box-step([invent state], [`LoadCtx` owns everything], fill: white, body-size: 8.2pt)],
      [#box-step([prove race], [exactly one winner], fill: c-orange, body-size: 8.2pt)],

      [#box-step([late events], [cancel may still call back], fill: c-blue, body-size: 8.2pt)],
      [#box-step([re-glue], [repeat for each API mix], fill: white, body-size: 8.2pt)],
    )
    #v(0.35em)
    #text(
      size: 9pt,
    )[These are correctness chores, not the application logic. Miss one and you get use-after-free, double completion, leaked buffers, wrong-thread continuation, or lost errors. A uniform operation shape lets `timeout(read)` and `then(parse)` be reusable steps instead.]
  ]],
)



#pagebreak()

== The glue is a hand-written protocol

#callout(fill: c-blue, stroke: c-accent, inset: (x: 12pt, y: 8pt))[
  To compose the callbacks, the programmer has to create the missing operation object. We specifically face many problems like this from the *SG-IOV* work.
]

#grid(
  columns: (0.92fr, 1.08fr),
  gutter: 12pt,
  [#panel(fill: c-row, inset: (x: 12pt, y: 9pt))[
    #text(weight: "bold", fill: c-title)[1. Invent shared operation state]
    #v(0.15em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```c
      typedef struct LoadCtx {
        overlapped io;
        timer_handle *timer;
        pool *cpu; loop *ev;
        char *buf; int len;

        atomic_bool done;
        atomic_int refs;
        void (*finish)(Result, void *);
        void *user;
      } LoadCtx;
      ```,
    )
    #v(0.2em)
    #text(
      size: 8.8pt,
    )[This struct exists only because the call stack can no longer hold the operation's continuation and lifetime.]
  ]],
  [#panel(fill: c-green, inset: (x: 12pt, y: 9pt))[
    #text(weight: "bold", fill: c-title)[2. Make every callback obey it]
    #v(0.15em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```c
      bool claim(LoadCtx *c) {
        return !atomic_exchange(&c->done, true);
      }

      void on_read(int st, int n, overlapped *io) {
        LoadCtx *c = container_of(io, LoadCtx, io);
        if (!claim(c)) { release(c); return; }
        cancel_timer(c->timer);
        if (st) finish_error(c, st);
        else post_work(c->cpu, parse, c);
      }

      void on_timeout(void *p) {
        LoadCtx *c = p;
        if (!claim(c)) { release(c); return; }
        cancel_read(&c->io); // callback may still arrive
        finish_error(c, ETIMEDOUT);
      }
      ```,
    )
  ]],
)


#pagebreak()

== Keep the async story in one place

#callout(fill: c-blue, stroke: c-accent, inset: (x: 12pt, y: 8pt))[
  The win is local control flow: read with timeout, then parse, then report. The API carries the continuation machinery.
]

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 9pt,
  [#panel(fill: c-row, inset: (x: 10pt, y: 8pt))[
    #text(weight: "bold", fill: c-title)[Go: goroutine + `select`]
    #v(0.15em)
    #zebraw(
      numbering: false,
      inset: (x: 3pt, y: 1.5pt),
      comment-font-args: (size: 6.5pt),
      ```go
      go func() {
        select {
        case r := <-readDone:
          parseOnPool(r)
        case <-time.After(100*ms):
          cancelRead()
        }
      }()
      ```,
    )
    #v(0.2em)
    #text(size: 8.5pt)[One goroutine body; `select` names the race.]
  ]],
  [#panel(fill: c-green, inset: (x: 10pt, y: 8pt))[
    #text(weight: "bold", fill: c-title)[Rust/Tokio: `Future` + `.await`]
    #v(0.15em)
    #zebraw(
      numbering: false,
      inset: (x: 3pt, y: 1.5pt),
      comment-font-args: (size: 6.5pt),
      ```rust
      let n = timeout(dur, read()).await??;
      let x = spawn_blocking(|| parse(n)).await??;
      report(x).await?;
      ```,
    )
    #v(0.2em)
    #text(size: 8.5pt)[One async block; `.await` marks where it may pause.]
  ]],
  [#panel(fill: c-orange, inset: (x: 10pt, y: 8pt))[
    #text(weight: "bold", fill: c-title)[C++: coroutine or sender]
    #v(0.15em)
    #zebraw(
      numbering: false,
      inset: (x: 3pt, y: 1.5pt),
      comment-font-args: (size: 6.5pt),
      ```cpp
      timeout(async_read(), 100ms)
        | let_value(parse_on(cpu))
        | continues_on(loop)
        | then(report);
      ```,
    )
    #v(0.2em)
    #text(size: 8.5pt)[One pipeline; adapters name the next step.]
  ]],
)

#pagebreak()

== Go is stackful; Rust/C++ futures are stackless

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Go preserves a suspended call stack, like a lightweight userspace thread. Rust and C++ async/coroutine styles compile suspension points into state-machine objects.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Go: stackful goroutine]
    #v(0.35em)
    #box-step([goroutine stack], [call frames + locals + next instruction], fill: c-orange)
    #v(0.35em)
    + More like a userspace thread: it has its own stack.
    + Suspension keeps the call stack intact across function calls.
    + The Go runtime parks, grows, moves, and resumes stacks.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Rust/C++: stackless future/coroutine]
    #v(0.35em)
    #box-step([future value], [enum-like state machine + live locals], fill: c-blue)
    #v(0.35em)
    + Suspension is explicit: Rust `.await`; C++ `co_await`, `co_return`, `co_yield`.
    + The compiler builds an object/state machine instead of requiring a runtime stack.
    + Zero-cost principle: pay mainly for the state you save and the scheduling you use.
  ]],
)

#note[
  The shared idea is stackless suspension: the compiler rewrites a sequential-looking body into an object that can stop and resume at explicit suspension points.
]

#pagebreak()

== Rust (and also C++ coroutines) stores the continuation in a `Future`

#callout(fill: c-blue, stroke: c-accent)[
  Rust's language-level answer is a stackless `Future` (let the compiler to write the dirty codes): `async fn` creates a value that stores saved locals and nothing makes progress until some executor polls it.
]

#grid(
  columns: (1fr, 0.15fr, 1fr, 0.15fr, 1fr),
  gutter: 7pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[1. Language syntax]
    #v(0.35em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      async fn fetch() -> Bytes {
        read().await
      }
      ```,
    )
    `async fn` does not choose an event loop. It returns a value implementing `Future`.
  ]],
  [#arrow],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[2. Future trait]
    #v(0.35em)
    #box-step([`poll`], [`Ready` or `Pending`], fill: white)
    #v(0.35em)
    A future is a state machine plus a standard `poll` interface.
  ]],
  [#arrow],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[3. Executor needed]
    #v(0.35em)
    #box-step([runtime], [Tokio, async-std, custom], fill: white)
    #v(0.35em)
    The executor decides when and where to poll futures.
  ]],
)

#note[
  Keep the boundary clear: Rust provides `Future`, `.await`, and `Poll`; Tokio connects them to timers, sockets, worker threads, and scheduling.
]

#pagebreak()



== The sequential illusion

#callout(fill: c-blue, stroke: c-accent)[
  `async fn` looks sequential, but calling it creates a future; `.await` is where that future may give control back to Tokio.
]

#grid(
  columns: (0.95fr, 1.05fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[What we write]
    #v(0.4em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      async fn handle() {
        let a = read().await;
        let b = compute(a);
        write(b).await;
      }
      ```,
    )
    #v(0.4em)
    Calling `handle()` alone does not run all of this immediately; it produces a future that must be awaited or spawned.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What happens over time]
    #v(0.45em)
    #grid(
      columns: (1fr, 0.14fr, 1fr, 0.14fr, 1fr, 0.14fr, 1fr),
      gutter: 4pt,
      [#box-step([run], [until first wait], fill: white)],
      [#arrow],
      [#box-step([Pending], [yield to runtime], fill: c-orange)],
      [#arrow],
      [#box-step([resume], [continue after `.await`], fill: white)],
      [#arrow],
      [#box-step([Done], [return output], fill: c-blue)],
    )
    #v(0.5em)
    When the future returns `Pending`, the thread can run another task instead of blocking.
  ]],
)

== One future stores state; `.await` defines transitions

#callout(fill: c-blue, stroke: c-accent)[
  A future is not a hidden thread. It is a value that stores “which await am I blocked on?” plus the child future and locals needed to resume.
]

#grid(
  columns: (0.92fr, 1.08fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Source: only `.await` is the suspension marker]
    #v(0.25em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      highlight-lines: (
        2,
        3,
        4,
        (7, [`spawn`: hand the future to Tokio]),
      ),
      ```rust
      async fn handle() {
        let req = read_req().await;
        let resp = route(req).await;
        write_resp(resp).await;
      }

      tokio::spawn(handle());
      ```,
    )
    #v(0.25em)
    #text(size: 9.4pt, fill: luma(80))[`spawn` does not run the body inline; it registers the future as a Tokio task.]
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Pseudo-code: compiler-built future]
    #v(0.2em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      // state = Start | Read(f) | Route(f) | Write(f)
      fn poll(&mut self, cx: &mut Cx) -> Poll<()> {
        loop { match &mut self.state {
          Start => self.state = Read(read_req()),
          Read(f) => match f.poll(cx) {
            Pending => return Pending, // keep Read
            Ready(req) => self.state = Route(route(req)),
          },
          Route(f) => match f.poll(cx) {
            Pending => return Pending, // keep Route
            Ready(resp) => self.state = Write(write_resp(resp)),
          },
          Write(f) => return f.poll(cx),
        }}
      }
      ```,
    )
  ]],
)

#note[
  The future owns the current state and live locals. `.await` is the boundary where polling may yield `Pending` and resume from the saved state later.
]


= Problem 4: Futures do not run themselves

A `Future` is inert until a runtime polls it; Tokio supplies the workers, event loop, ready queue, and wakers.


== Tokio adds scheduling and I/O events

#callout(fill: c-blue, stroke: c-accent)[
  Tokio connects Rust futures to a concrete executor, task scheduler, and event sources such as timers or I/O.
]

#grid(
  columns: (1fr, 0.15fr, 1fr, 0.15fr, 1fr),
  gutter: 7pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[1. Your async code]
    #v(0.35em)
    #box-step([Future], [stores progress and local variables], fill: white)
    #v(0.35em)
    Returns `Ready` when done, or `Pending` when it must wait.
  ]],
  [#arrow],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[2. Tokio executor]
    #v(0.35em)
    #box-step([Task scheduler], [polls ready futures], fill: white)
    #v(0.35em)
    It does not spin forever; it waits for a wakeup signal.
  ]],
  [#arrow],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[3. I/O or timer event source]
    #v(0.35em)
    #box-step([Waker], [requeue this task later], fill: white)
    #v(0.35em)
    Timers, sockets, or other events call wake when progress may be possible.
  ]],
)

#note[
  Future = saved computation; Tokio executor = scheduler; Waker = callback path back into the scheduler.
]

#pagebreak()


== Event loop code and Tokio task code share the same idea

#callout(fill: c-blue, stroke: c-accent)[
  Traditional network code makes the event loop explicit; Tokio hides that loop behind `async` tasks while keeping the same I/O event idea underneath.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Traditional event loop]
    #v(0.35em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      loop {
        evs = epoll_wait(epfd);
        for ev in evs {
          if ev.readable { rd(fd); }
          if ev.writable { wr(fd); }
        }
      }
      ```,
    )
    The programmer explicitly asks the OS which file descriptors are ready, then dispatches handlers.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Tokio task style]
    #v(0.35em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      tokio::spawn(async move {
        let n = sock.read(&mut buf).await?;
        sock.write_all(&buf[..n]).await?;
        Ok::<_, io::Error>(())
      });
      ```,
    )
    The task reads as sequential code. The runtime owns the event loop and wakes the task when the socket may progress.
  ]],
)

#note[
  Read `.await` as: try the operation now; if it would block, save this task's continuation and let Tokio run something else.
]

#pagebreak()

== `Pending` saves a `Waker` for later

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  When a future cannot finish now, it returns `Pending` and records a `Waker` so something else can ask Tokio to poll it later.
]

#grid(
  columns: (1.05fr, 0.95fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Conceptual future implementation]
    #v(0.35em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      fn poll(self: Pin<&mut Self>) {
        if operation_is_ready() {
          Poll::Ready(result)
        } else {
          save(waker);
          Poll::Pending
        }
      }
      ```,
    )
  ]],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[What the saved waker does]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
      gutter: 5pt,
      [#box-step([not ready], [save waker], fill: white)],
      [#arrow],
      [#box-step([event arrives], [timer / socket], fill: c-orange)],
      [#arrow],
      [#box-step([wake task], [poll again], fill: white)],
    )
    #v(0.45em)
    The waker is not the result. It is a handle for saying: “this task may now be worth polling again.”
  ]],
)

#pagebreak()

== I/O events find work; wakers requeue tasks

#callout(fill: c-blue, stroke: c-accent)[
  Tokio combines two familiar ideas from network programming: event polling asks “which socket or timer changed?”, while wakers make task notification explicit.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[1. Polling / reactive style]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
      gutter: 5pt,
      [#box-step([`poll/epoll`], [ask kernel what is ready], fill: white)],
      [#arrow],
      [#box-step([event loop], [dispatch handler], fill: c-blue)],
      [#arrow],
      [#box-step([try I/O], [may still block?], fill: white)],
    )
    #v(0.45em)
    Traditional server code often loops over I/O events, then calls the operation that should now make progress.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[2. Waker / proactive style]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
      gutter: 5pt,
      [#box-step([Future], [returns `Pending`], fill: white)],
      [#arrow],
      [#box-step([store `Waker`], [how to requeue me], fill: c-orange)],
      [#arrow],
      [#box-step([wake], [push task ready], fill: white)],
    )
    #v(0.45em)
    Tokio still waits for I/O and timer events under the hood, but each pending future leaves behind a precise callback path to the task that should be polled again.
  ]],
)

#note[
  `epoll` tells the runtime that a socket is ready; the `Waker` tells the runtime which suspended Rust task should be put back on the ready queue.
]

#pagebreak()

== Poll, Waker, and Tokio executor

#callout(fill: c-blue, stroke: c-accent)[
  Tokio's `poll` is the “try to make progress” step; `Waker` is the requeue handle registered when progress is not possible yet.
]

#grid(
  columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
  gutter: 5pt,
  [#box-step([Tokio executor], [owns ready tasks], fill: c-green)],
  [#arrow],
  [#box-step([poll future], [`Poll::Ready` or `Pending`], fill: c-blue)],
  [#arrow],
  [#box-step([register Waker], [if not ready], fill: c-orange)],
)
#v(0.35em)
#align(center)[#text(size: 16pt, fill: c-accent)[↓ I/O event calls wake]]
#v(0.35em)
#grid(
  columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
  gutter: 5pt,
  [#box-step([wake task], [mark ready], fill: c-orange)],
  [#arrow],
  [#box-step([schedule], [put in run queue], fill: c-row)],
  [#arrow],
  [#box-step([poll again], [resume from saved state], fill: c-green)],
)

#v(0.45em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 8pt,
  [#box-step([`Ready(x)`], [future completed], fill: c-green)],
  [#box-step([`Pending`], [try again later], fill: c-orange)],
  [#box-step([`Waker`], [how to requeue task], fill: c-blue)],
)

#pagebreak()

= Problem 5: Runtime-owned tasks need type guarantees

A spawned future can outlive the stack frame that created it, so Rust must prove what its saved state owns, borrows, and can move.


== Borrow vs `async move`: view vs captured value

#callout(fill: c-blue, stroke: c-accent)[
  `async move` is like a move closure: variables used inside are captured by value into the future object. Non-`Copy` values move; `Copy` values are copied.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Borrow: future depends on caller]
    #v(0.25em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      let cfg = Cfg::load();

      let fut = async {
        read_with(&cfg).await;
      };
      ```,
    )
    #v(0.25em)
    `fut` may use `&cfg`, so `cfg` must stay alive until `fut` is done.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[`async move`: future captures by value]
    #v(0.25em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      let cfg = Cfg::load();

      let fut = async move {
        read_with(&cfg).await;
      };
      ```,
    )
    #v(0.25em)
    `cfg` becomes a field inside `fut`. The `&cfg` inside the block borrows that stored field, not the caller's stack slot.
  ]],
)

#pagebreak()

== Lifetimes: how long a borrow may be used

#callout(fill: c-blue, stroke: c-accent)[
  Rust lifetimes answer one concrete question: can this reference still point to live data at every place it is used?
]

#grid(
  columns: (0.95fr, 1.05fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[A reference is only a view]
    #v(0.3em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      let cfg = Cfg::load();   // owner

      {
        let r = &cfg;        // borrow
        use_cfg(r);
      }                     // borrow ends

      drop(cfg);             // owner ends
      ```,
    )
    #v(0.25em)
    `&Cfg` does not own a `Cfg`; it is valid only while the owner is still alive.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What the compiler checks]
    #v(0.45em)
    #grid(
      columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
      gutter: 5pt,
      [#box-step([owner alive], [`cfg` storage exists], fill: white)],
      [#arrow],
      [#box-step([borrow used], [`&cfg` is safe], fill: c-blue)],
      [#arrow],
      [#box-step([owner ends], [reference invalid], fill: c-orange)],
    )
    #v(0.5em)
    + A lifetime is a proof window, not a runtime timer.
    + An annotation names a relationship; it does not make data live longer.
  ]],
)

#pagebreak()

== Async challenge: `.await` can store a borrow for later

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  An async function with a borrowed argument creates a future whose saved state may contain that borrow across `.await`.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[What the future may contain]
    #v(0.25em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      async fn read_cfg(cfg: &Cfg) {
        read_with(cfg).await;
      }

      // conceptual shape
      struct ReadCfg<'a> {
        cfg: &'a Cfg,
        state: State,
      }
      ```,
    )
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[When is that okay?]
    #v(0.25em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      // OK: future completes here
      read_cfg(&cfg).await;

      // Problem: task may outlive this stack
      tokio::spawn(read_cfg(&cfg));

      // Solution: give the task owned state
      tokio::spawn(async move {
        read_cfg(&owned_cfg).await;
      });
      ```,
    )
    #v(0.2em)
    For shared config, move an `Arc<Cfg>` into the task.
  ]],
)

#pagebreak()

== Tokio tasks and `spawn`

#callout(fill: c-blue, stroke: c-accent)[
  `spawn` crosses an ownership boundary: the task may continue after the function that created it has returned.
]

#grid(
  columns: (1.05fr, 0.95fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Before spawn: caller owns the stack]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.12fr, 1fr),
      gutter: 5pt,
      [#box-step([caller frame], [`cfg`, request, locals], fill: white)],
      [#arrow],
      [#box-step([create future], [captures values], fill: c-blue)],
    )
    #v(0.45em)
    Local variables can disappear when the caller returns.
  ]],
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[After spawn: runtime owns the task]
    #v(0.35em)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 7pt,
      [#box-step([worker 1], [task A], fill: white)],
      [#box-step([worker 2], [task A later], fill: white)],
      [#box-step([worker 3], [other tasks], fill: white)],
    )
    #v(0.45em)
    `tokio::spawn` stores the future in the runtime. The spawned future and its output must be `Send + 'static` on the multi-thread runtime.
  ]],
)

#pagebreak()

== `async move` transfers ownership into the task

#callout(fill: c-blue, stroke: c-accent)[
  `spawn` means the runtime owns the task, so captured values must be safe to keep after the caller returns.
]

#zebraw(
  numbering: false,
  inset: (x: 4pt, y: 2pt),
  comment-font-args: (size: 7pt),
  ```rust
  let shared = Arc::new(State::new());
  let task_state = shared.clone();

  tokio::spawn(async move {
      task_state.handle().await;
  });
  ```,
)

#pagebreak()

== The lifetime challenge

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Lifetime errors prevent a saved future from containing references to stack data that may disappear before the future resumes.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[OK: awaited in the same scope]
    #v(0.45em)
    #grid(
      columns: (1fr, 0.15fr, 1fr, 0.15fr, 1fr),
      gutter: 5pt,
      [#box-step([borrow `cfg`], [short lifetime], fill: white)],
      [#arrow],
      [#box-step([future], [contains reference], fill: c-blue)],
      [#arrow],
      [#box-step([`.await` now], [`cfg` still alive], fill: white)],
    )
    #v(0.45em)
    A borrowed future can be fine when it is awaited before the borrowed value goes away.
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[Harder: spawned task]
    #v(0.45em)
    #grid(
      columns: (1fr, 0.15fr, 1fr, 0.15fr, 1fr),
      gutter: 5pt,
      [#box-step([stack `cfg`], [caller owns it], fill: white, accent: rgb("#dc2626"))],
      [#arrow],
      [#box-step([spawn task], [may outlive caller], fill: c-orange, accent: rgb("#dc2626"))],
      [#arrow],
      [#box-step([borrow?], [not enough], fill: white, accent: rgb("#dc2626"))],
    )
    #v(0.45em)
    A spawned task usually needs owned data: move the value, clone it, or use `Arc`.
  ]],
)

#v(0.35em)

#callout(fill: c-red, stroke: rgb("#dc2626"))[
  `static` in this context means “the future contains no short-lived borrowed references,” not “the value leaks forever.”
]

#pagebreak()

== Local await bounds the borrow; spawn may let it escape

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Borrowing across `.await` is okay only when the future cannot outlive the borrowed value.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Local await: borrow is bounded]
    #v(0.35em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      async fn use_cfg(cfg: &Cfg) {
          read_with(cfg).await;
      }

      use_cfg(&cfg).await;
      ```,
    )
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[Spawn: borrow may escape]
    #v(0.35em)
    #zebraw(
      numbering: false,
      inset: (x: 4pt, y: 2pt),
      comment-font-args: (size: 7pt),
      ```rust
      tokio::spawn(async {
          read_with(&cfg).await;
      }); // not 'static

      tokio::spawn(async move {
          read_with(&owned_cfg).await;
      });
      ```,
    )
  ]],
)

#pagebreak()

== `Send` / `Sync`: safe to cross thread boundaries

#callout(fill: c-blue, stroke: c-accent)[
  Rust uses marker traits to decide what may cross threads: `Send` for moving ownership to another thread, `Sync` for sharing references between threads.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Rust's question]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.13fr, 1fr),
      gutter: 5pt,
      [#box-step([`Send`], [can move `T` to another thread], fill: c-blue, body-size: 8.8pt)],
      [#arrow],
      [#box-step([`Sync`], [can share `&T` with another thread], fill: c-green, body-size: 8.8pt)],
    )
    #v(0.45em)
    These are compile-time guarantees about the type. They do not add locks or make unsafe sharing safe at runtime.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[Why Tokio requires them]
    #v(0.35em)
    #grid(
      columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
      gutter: 5pt,
      [#box-step([poll], [worker 1], fill: white)],
      [#arrow],
      [#box-step([`.await`], [state saved], fill: c-orange)],
      [#arrow],
      [#box-step([resume], [worker 2], fill: white)],
    )
    #v(0.45em)
    Multi-thread `tokio::spawn` may store a task, wake it later, and run it on another worker. Therefore the spawned future and output must be safe to move across threads.
  ]],
)

#note[
  If a task must keep `!Send` state, use single-thread-local execution such as `LocalSet` / `spawn_local` instead of multi-thread `tokio::spawn`.
]

#pagebreak()

== `Pin` protects future state after polling starts

#callout(fill: c-blue, stroke: c-accent)[
  `Pin` says that after a future starts running, the memory its saved state may depend on must not be moved. This is address stability.
]

#grid(
  columns: (1fr, 0.13fr, 1fr, 0.13fr, 1fr),
  gutter: 7pt,
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[1. Created]
    #v(0.35em)
    #box-step([Future value], [ordinary movable value], fill: white)
  ]],
  [#arrow],
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[2. First poll]
    #v(0.35em)
    #box-step([State initialized], [locals saved inside], fill: white)
  ]],
  [#arrow],
  [#panel(fill: c-orange)[
    #text(weight: "bold", fill: c-title)[3. Pinned]
    #v(0.35em)
    #box-step([Stable address], [safe to resume later], fill: white)
  ]],
)

#v(0.55em)

#callout(fill: c-orange, stroke: rgb("#f97316"), inset: (x: 12pt, y: 7pt))[
  Many accelerators require submitted descriptors or buffers to stay at a stable address until completion. You cannot move the state after the device may use that address.
]

#pagebreak()

== The `Future::poll` signature receives pinned state

#callout(fill: c-blue, stroke: c-accent)[
  Application code usually awaits futures, but executor/library code polls a pinned future so saved state is not moved unexpectedly.
]

#zebraw(
  numbering: false,
  inset: (x: 4pt, y: 2pt),
  comment-font-args: (size: 7pt),
  ```rust
  trait Future {
      type Output;

      fn poll(
          self: Pin<&mut Self>,
          cx: &mut Context<'_>,
      ) -> Poll<Self::Output>;
  }

  let value = some_future.await;
  ```,
)

#pagebreak()

= Close: Reason from the saved continuation

The same question explains every later rule: what continuation is saved, who can wake it, and where may that saved state live or move?


== Rules of thumb for Tokio code

#callout(fill: c-blue, stroke: c-accent)[
  Good Tokio code makes ownership and wakeup boundaries explicit.
]

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 10pt,
  [#card(
    [Own when spawning],
    [Use `async move`; clone or move owned values into tasks. Use `Arc` for shared long-lived state.],
    fill: c-green,
    body-size: 10.2pt,
  )],
  [#card(
    [Keep borrows local],
    [Borrow across `.await` only when the future is awaited within a scope that proves the borrow stays valid.],
    fill: c-blue,
    body-size: 10.2pt,
  )],
  [#card(
    [Watch cross-await state],
    [Values used after `.await` become saved task state and may need `Send`.],
    fill: c-orange,
    body-size: 10.2pt,
  )],

  [#card(
    [Avoid locks across await],
    [Do not hold blocking locks across `.await`; prefer scoped locks or async-aware synchronization.],
    fill: c-row,
    body-size: 10.2pt,
  )],
  [#card(
    [Use channels],
    [Communicate between tasks with channels instead of shared mutable state when possible.],
    fill: c-row,
    body-size: 10.2pt,
  )],
  [#card(
    [Plan cancellation],
    [Dropping a future or task can cancel work; make cleanup and ownership behavior intentional.],
    fill: c-row,
    body-size: 10.2pt,
  )],
)

#v(0.45em)

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  When code reaches `.await`, ask three questions: what state is saved, who wakes it, and can that saved state outlive or move with the task?
]

