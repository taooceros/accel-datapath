// Prototype advisor presentation: async mechanisms for accelerator data paths
// Reader: advisor / project collaborator.
// Claim boundary: conceptual comparison only; no new benchmark or hardware claims.
// Sources:
// - docs/plan/2026-05-02/05.async-mechanism-advisor-slide.plan.md
// - docs/report/research/002.async_mechanism_description_sources.md

#import "../../template.typ": callout, card, deck, note, palette, panel

#show: deck.with(
  margin: (x: 40pt, y: 30pt),
  size: 13.2pt,
  leading: 0.84em,
  spacing: 0.62em,
)

#let c-title = palette.title
#let c-accent = palette.accent
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red
#let c-row = palette.row

#let flow-step(label, body, fill: c-row, accent: c-accent) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 9pt, y: 8pt),
  fill: fill,
  stroke: 0.55pt + palette.border,
)[
  #align(center)[#text(weight: "bold", fill: accent)[#label]]
  #v(0.25em)
  #align(center)[#text(size: 10pt, fill: luma(65))[#body]]
]

#let tiny-code(body) = block(
  width: 100%,
  radius: 5pt,
  inset: (x: 10pt, y: 7pt),
  fill: luma(245),
  stroke: 0.45pt + luma(205),
)[#text(font: "Latin Modern Mono", size: 10.4pt)[#body]]

#let code-lines(lines) = block(
  width: 100%,
  radius: 5pt,
  inset: (x: 10pt, y: 7pt),
  fill: luma(245),
  stroke: 0.45pt + luma(205),
)[
  #for line in lines [
    #text(font: "Latin Modern Mono", size: 10.6pt)[#line]
    #linebreak()
  ]
]

= Async mechanisms: same lifecycle, different proof obligations

#align(center + horizon)[
  #text(size: 18pt)[A prototype advisor deck]
  #v(0.55em)
  #text(size: 14pt, fill: luma(90))[`stdexec`, Rust Tokio, and C++ coroutine for accelerator-backed async work]
  #v(0.55em)
  #text(size: 12.5pt, fill: luma(125))[Hongtao Zhang · May 2026]
]

#v(0.65em)

#callout(fill: c-blue, stroke: c-accent)[
  The goal is not to teach every framework detail. The goal is to explain what each mechanism makes explicit when a DSA/IAX operation is outstanding.
]

== One async shape behind three mechanisms

#callout(fill: c-blue, stroke: c-accent)[
  Takeaway: all three mechanisms submit work without blocking the CPU, save what must happen next, and resume that continuation after completion.
]

#grid(
  columns: (1fr, 0.16fr, 1fr, 0.16fr, 1fr, 0.16fr, 1fr, 0.16fr, 1fr),
  gutter: 4pt,
  [#flow-step([submit async work], [descriptor / task / operation], fill: c-green)],
  [#align(center + horizon)[#text(size: 20pt, fill: c-accent)[→]]],
  [#flow-step([save continuation], [locals, buffers, next step], fill: c-blue)],
  [#align(center + horizon)[#text(size: 20pt, fill: c-accent)[→]]],
  [#flow-step([pending or suspend], [CPU can run other work], fill: c-orange)],
  [#align(center + horizon)[#text(size: 20pt, fill: c-accent)[→]]],
  [#flow-step([completion event], [record, poll, interrupt, wake], fill: c-row)],
  [#align(center + horizon)[#text(size: 20pt, fill: c-accent)[→]]],
  [#flow-step([resume], [dependent code runs], fill: c-green)],
)

#v(0.7em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 12pt,
  [#card([`stdexec`], [explicit operation graph + scheduler composition], fill: c-row, body-size: 10.8pt)],
  [#card([Tokio], [`Future::poll` + `Waker` + Rust lifetime proof], fill: c-row, body-size: 10.8pt)],
  [#card([C++ coroutine], [`co_await` + coroutine frame + library policy], fill: c-row, body-size: 10.8pt)],
)

#note[
  DSA/IAX mapping: hardware has pointers to buffers and a completion record while software wants to continue. The async interface must say who keeps that state valid and who resumes execution.
]

== `stdexec`: explicit operation graph

#callout(fill: c-blue, stroke: c-accent)[
  Takeaway: `stdexec` makes the async graph and scheduler boundary visible, which is useful for controlled C++ experiments.
]

#grid(
  columns: (1.25fr, 0.75fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Visual model]
    #v(0.45em)
    #grid(
      columns: (1fr, 0.18fr, 1fr, 0.18fr, 1fr),
      gutter: 4pt,
      [#flow-step([sender], [lazy description], fill: white)],
      [#align(center + horizon)[#text(size: 17pt, fill: c-accent)[→]]],
      [#flow-step([scheduler], [where work runs], fill: c-blue)],
      [#align(center + horizon)[#text(size: 17pt, fill: c-accent)[→]]],
      [#flow-step([receiver], [value / error / stopped], fill: white)],
    )
    #v(0.55em)
    #tiny-code[`just(...) | on(scheduler) | then(...) | connect(receiver) | start()`]
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[How to say it]
    #v(0.35em)
    + Describe work lazily; starting is a separate step.
    + Completion contract is explicit at the receiver.
    + Scheduler and composition layers can be swapped.
    + This matches the earlier layer-removal experiment style.
  ]],
)

#v(0.35em)

#note[
  Advisor framing: `stdexec` is good when the research question is “which software layer costs how much?”
]

== Tokio/Rust: polling, wakeups, and lifetime proof

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Takeaway: Tokio gives a natural task/waker integration point, but Rust forces us to prove that all state crossing `.await` remains valid.
]

#grid(
  columns: (1fr, 1fr),
  gutter: 14pt,
  [#panel(fill: c-blue)[
    #text(weight: "bold", fill: c-title)[Runtime view]
    #v(0.35em)
    #tiny-code[`poll(Pin<&mut Future>, Context) -> Pending`]
    #v(0.35em)
    + `Pending` does not block the thread.
    + The future registers a `Waker` in the context.
    + Completion calls wake; Tokio schedules another poll.
    + State used after `.await` is stored in the task/future.
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[Lifetime view for hardware]
    #v(0.35em)
    #grid(
      columns: (1fr, 1fr),
      gutter: 6pt,
      [#flow-step([src buffer], [must not disappear], fill: white, accent: rgb("#dc2626"))],
      [#flow-step([dst buffer], [not complete until done], fill: white, accent: rgb("#dc2626"))],
      [#flow-step([completion record], [must stay address-valid], fill: white, accent: rgb("#dc2626"))],
      [#flow-step([task state], [`Send` if spawned], fill: white, accent: rgb("#dc2626"))],
    )
    #v(0.35em)
    `tokio::spawn` pushes toward `'static + Send`; `Pin` matters after polling.
  ]],
)

#v(0.35em)

#callout(fill: c-red, stroke: rgb("#dc2626"))[
  Project-specific point: hardware may still own pointers while Rust wants to move or drop values. The async API must make that impossible, not merely undocumented.
]

== C++ coroutine: syntax-level suspension

#callout(fill: c-blue, stroke: c-accent)[
  Takeaway: C++ coroutines make async code look sequential, but libraries still provide scheduling and I/O policy.
]

#grid(
  columns: (0.95fr, 1.05fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[What the programmer sees]
    #v(0.4em)
    #code-lines((
      "submit_descriptor();",
      "co_await completion;",
      "continue_with_result();",
    ))
    #v(0.45em)
    Sequential-looking code, with `co_await` marking a possible suspension point.
  ]],
  [#panel(fill: c-green)[
    #text(weight: "bold", fill: c-title)[What the mechanism stores]
    #v(0.4em)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 7pt,
      [#flow-step([locals], [saved across suspend], fill: white)],
      [#flow-step([suspend point], [where to resume], fill: white)],
      [#flow-step([handle], [resume / destroy], fill: white)],
    )
    #v(0.45em)
    Awaiter/promise code decides whether to suspend and how resumption is triggered.
  ]],
)

#v(0.4em)

#note[
  Advisor framing: coroutine syntax is a clean way to express continuation, but it is not by itself an accelerator runtime.
]

== Advisor comparison: what each mechanism makes explicit

#callout(fill: c-blue, stroke: c-accent)[
  Takeaway: choose the mechanism based on which boundary we want to study: graph composition, Rust safety/lifetimes, or syntax-level suspension.
]

#table(
  columns: (0.86fr, 1.18fr, 1.05fr, 1.35fr),
  inset: (x: 6pt, y: 5pt),
  stroke: 0.4pt + luma(205),
  [#text(weight: "bold")[Mechanism]],
  [#text(weight: "bold")[What is explicit?]],
  [#text(weight: "bold")[Who resumes?]],
  [#text(weight: "bold")[Project value / risk]],

  [`stdexec`],
  [Operation graph, receiver contract, scheduler composition.],
  [Scheduler starts operation; receiver receives completion signal.],
  [Good for controlled layer-removal experiments.],

  [Tokio/Rust],
  [`Future::poll`, `Waker`, task state, ownership/lifetimes.],
  [Runtime polls again after wakeup.],
  [Best fit for Rust integration; hard part is buffers and completion records across `.await`.],

  [C++ coroutine],
  [`co_await`, coroutine frame, awaiter/promise hooks.],
  [Awaiter/runtime resumes coroutine handle.],
  [Good syntax model; policy and runtime still need to be supplied.],
)

#v(0.45em)

#callout(fill: c-green, stroke: rgb("#16a34a"))[
  Same lifecycle, different proof obligations. For this project, the distinctive Rust work is making lifetime and ownership obligations visible in the API.
]

== Backup: the Rust lifetime challenge in one example

#callout(fill: c-orange, stroke: rgb("#f97316"))[
  Use this only if the advisor asks why the Tokio path is harder than “just wake a task.”
]

#grid(
  columns: (0.9fr, 1.1fr),
  gutter: 14pt,
  [#panel(fill: c-row)[
    #text(weight: "bold", fill: c-title)[Pseudo API shape]
    #v(0.45em)
    #code-lines((
      "memmove(&src, &mut dst).await",
      "// hardware is still running",
    ))
    #v(0.5em)
    The borrow spans the async gap: software is suspended, but hardware may still read or write memory.
  ]],
  [#panel(fill: c-red)[
    #text(weight: "bold", fill: c-title)[What must remain true]
    #v(0.35em)
    + `src` cannot be dropped while hardware may read it.
    + `dst` cannot be treated as complete until completion.
    + Completion record must stay valid and address-stable.
    + Spawned task state must be `'static + Send` when it can move across runtime threads.
    + `Pin` prevents invalid movement of address-sensitive future state.
  ]],
)

#v(0.4em)

#callout(fill: c-blue, stroke: c-accent)[
  Rust makes these constraints visible. The design task is to encode them cleanly instead of relying on programmer discipline.
]
