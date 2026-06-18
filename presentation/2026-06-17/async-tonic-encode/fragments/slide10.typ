#import "../../../template.typ": *
#import "../support.typ": *

#let slide_10() = slide[
  #slide-head("10", [Reusable Rule], tag: [ownership rule])

  #v(0.45em)
  #thesis(
    [If an operation may be stored and polled after the call returns, anything needed after suspension must be owned by that operation.],
    fill: green-soft,
    accent: green,
    size: 14.2pt,
  )

  #v(0.75em)
  #grid(columns: (1fr, auto, 1fr, auto, 1fr), gutter: 8pt, align: horizon)[
    #card(
      [Tonic owns frame boundary],
      [header reservation, compression, size check, flags, length],
      fill: blue-soft,
      accent: gray,
      body-size: 9.5pt,
      title-size: 10pt,
    )
  ][#arrow][
    #card(
      [`T::Encode` owns suspended work],
      [message, `EncodeBuffer`, offload state, completion, cancel/drop cleanup],
      fill: blue-soft,
      accent: blue,
      body-size: 9.5pt,
      title-size: 10pt,
    )
  ][#arrow][
    #card(
      [Prost owns wire checkpoint],
      [field, index, phase, and payload-copy continuation],
      fill: violet-soft,
      accent: violet,
      body-size: 9.5pt,
      title-size: 10pt,
    )
  ]

  #v(0.75em)
  #grid(columns: (1.05fr, 0.95fr), gutter: 13pt)[
    #card(
      [Measurement is downstream],
      [Architecture creates the chance to remove one staged CPU payload copy. Benchmarks decide whether offload submission, wakeup, and resume bookkeeping are smaller.],
      fill: white,
      accent: orange,
      body-size: 10.5pt,
      title-size: 11pt,
    )
  ][
    #block(width: 100%, inset: (x: 14pt, y: 15pt), radius: 13pt, fill: rgb("#0f172a"), stroke: 0.45pt + rgb("#334155"))[
      #align(center)[#text(
        size: 13pt,
        weight: "bold",
        fill: white,
      )[Async encode = owned resumable state + exact protocol resume points.]]
    ]
  ]

  #v(1fr)
]
