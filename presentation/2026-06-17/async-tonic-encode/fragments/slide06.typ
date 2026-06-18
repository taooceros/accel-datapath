#import "../../../template.typ": *
#import "../support.typ": *

#let boxlet(title, body, fill: white, accent: blue) = block(
  width: 100%,
  inset: (x: 10pt, y: 8pt),
  radius: 10pt,
  fill: fill,
  stroke: 0.55pt + accent,
)[
  #align(center)[#text(size: 10.2pt, weight: "bold", fill: accent)[#title]]
  #v(0.25em)
  #align(center)[#text(size: 8.8pt, fill: ink-soft)[#body]]
]

#let slide_06() = slide[
  #slide-head("06", [What Lives Inside `T::Encode`])

  #v(0.42em)
  #block(width: 100%, inset: (x: 16pt, y: 13pt), radius: 15pt, fill: blue-soft, stroke: 0.9pt + blue)[
    #align(center)[
      #text(size: 18pt, weight: "bold", fill: palette.title)[`DsaAsyncProstEncode<T>`]
      #v(0.12em)
      #text(size: 10.2pt, weight: "semibold", fill: blue)[the object Tonic stores]
    ]
    #v(0.75em)
    #grid(columns: (1fr, 1fr, 1fr), gutter: 8pt)[
      #boxlet([state], [`PollEncodeState` + payload state], fill: violet-soft, accent: violet)
    ][
      #boxlet([item], [`Option<T>`], fill: white, accent: blue)
    ][
      #boxlet([sink], [`DsaProstEncodeSink` + buffer], fill: green-soft, accent: green)
    ][
      #boxlet([options], [length-delimited mode], fill: white, accent: gray)
    ][
      #boxlet([stage], [`Start` / `Body` / `Done`], fill: white, accent: blue)
    ][
      #boxlet([payload cleanup], [descriptor + completion drop path], fill: orange-soft, accent: orange)
    ]
    #v(0.75em)
    #grid(columns: (1fr, auto, 1fr), gutter: 8pt, align: horizon)[
      #step([Pending: keep all compartments], fill: orange-soft, accent: orange, size: 10.4pt)
    ][#arrow][
      #step([Ready: `sink.into_inner()` returns `EncodeBuffer`], fill: green-soft, accent: green, size: 10.4pt)
    ]
  ]

  #v(0.65em)
  #danger[Drop order is part of the safety contract: pending payload state may reference item bytes and destination storage.]

  #v(1fr)
]
