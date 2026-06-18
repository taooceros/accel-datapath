#import "../../../template.typ": *
#import "../support.typ": *

#let slide_12() = slide[
  #slide-head("12", [Reusable Design Rule], tag: [ownership rule])
  #v(0.4em)

  #thesis(
    [
      If an operation may be stored and polled after the call returns, anything needed after suspension must be owned by that operation.
    ],
    fill: green-soft,
    accent: green,
    size: 12.3pt,
  )

  #v(0.55em)

  #grid(
    columns: (1fr, auto, 1fr, auto, 1fr),
    gutter: 7pt,
    align: horizon,
    soft-card(
      [Tonic framing],
      [Stream driver owns header reservation, compression, size check, and final gRPC length.],
      fill: blue-soft,
      accent: gray,
      body-size: 7.35pt,
    ),
    arrow,
    soft-card(
      [Encode future],
      [Suspended operation owns message, `EncodeBuffer`, offload state, completion, retry, and drop/cancel state.],
      fill: blue-soft,
      accent: blue,
      body-size: 7.35pt,
    ),
    arrow,
    soft-card(
      [Prost field state],
      [Inner poll engine owns field/index/phase progress plus pending payload-copy state.],
      fill: green-soft,
      accent: green,
      body-size: 7.35pt,
    ),
  )

  #v(0.55em)

  #two-col(
    soft-card(
      [Apply it to async Tonic encode],
      [
        #set list(marker: [#text(fill: green)[•]], indent: 0.9em, body-indent: 0.35em)
        - Tonic stores one owned `T::Encode` future.
        - The future owns message + buffer + hardware state.
        - Prost owns exact wire-format resume position.
        - CPU encode remains a ready future.
      ],
      fill: white,
      accent: green,
      body-size: 8pt,
    ),
    block(
      width: 100%,
      inset: (x: 12pt, y: 13pt),
      radius: 11pt,
      fill: rgb("#0f172a"),
      stroke: 0.4pt + rgb("#334155"),
    )[
      #align(center)[
        #text(
          size: 11.2pt,
          weight: "bold",
          fill: white,
        )[Async encode is owned resumable state plus exact protocol resume points.]
      ]
    ],
    ratio: (1.2fr, 0.8fr),
    gutter: 12pt,
  )

  #v(0.55em)
]
