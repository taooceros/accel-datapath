#import "../../../template.typ": *
#import "../support.typ": *

#let layer(title, subtitle, body, fill: white, accent: blue) = block(
  width: 100%,
  inset: (x: 13pt, y: 9pt),
  radius: 12pt,
  fill: fill,
  stroke: 0.65pt + accent,
)[
  #text(size: 11.5pt, weight: "bold", fill: accent)[#title]
  #h(0.7em)#text(size: 9.2pt, weight: "semibold", fill: ink-soft)[#subtitle]
  #v(0.45em)
  #body
]

#let slide_07() = slide[
  #slide-head("07", [Prost Records the Byte Resume Point])

  #v(0.4em)
  #thesis(
    [Tonic polls one future; Prost remembers the byte-position checkpoint inside it.],
    fill: green-soft,
    accent: green,
    size: 13.8pt,
  )

  #v(0.6em)
  #grid(columns: (1.05fr, 0.95fr), gutter: 14pt)[
    #block(width: 100%, inset: (x: 12pt, y: 10pt), radius: 14pt, fill: white, stroke: 0.55pt + panel-stroke)[
      #layer(
        [Tonic body driver],
        [coarse poll slot],
        [#step([stores and polls one `T::Encode`], fill: white, accent: blue, size: 9.6pt)],
        fill: blue-soft,
        accent: blue,
      )
      #v(0.32em)
      #align(center)[#down-arrow]
      #v(0.24em)
      #layer(
        [owned `T::Encode` future],
        [per in-flight message],
        [
          #step([owns message + buffer + offload state], fill: white, accent: blue, size: 9.4pt)
          #v(0.35em)
          #layer(
            [`PollEncodeState`],
            [nested inside the future],
            [#step([byte-level continuation], fill: white, accent: violet, size: 9.4pt)],
            fill: violet-soft,
            accent: violet,
          )
        ],
        fill: green-soft,
        accent: green,
      )
    ]
  ][
    #card(
      [`PollEncodeFrame` + payload state],
      [
        #grid(columns: (1fr, 1fr), gutter: 7pt)[
          #chip([field], fill: white, accent: violet, size: 10pt)
        ][
          #chip([index], fill: white, accent: violet, size: 10pt)
        ][
          #chip([phase], fill: white, accent: violet, size: 10pt)
        ][
          #chip([payload state], fill: orange-soft, accent: orange, size: 10pt)
        ]
        #v(0.65em)
        #text(size: 10.3pt)[CPU writes keys, lengths, and scalars immediately.]
        #v(0.35em)
        #text(size: 10.3pt, weight: "bold", fill: orange)[Payload copy may return `Pending`.]
      ],
      fill: white,
      accent: violet,
      body-size: 10.3pt,
      title-size: 12pt,
    )
  ]

  #v(1fr)
]
