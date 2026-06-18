#import "../../../template.typ": *
#import "../support.typ": *

#let inv(title, body, accent: blue, fill: white) = card(
  title,
  body,
  fill: fill,
  accent: accent,
  body-size: 9.4pt,
  title-size: 10.2pt,
  inset: (x: 11pt, y: 9pt),
)

#let slide_09() = slide[
  #slide-head("09", [Offload Makes Ownership Physical])

  #v(0.4em)
  #thesis([Pending offload means raw addresses can outlive the encode call.], fill: red-soft, accent: red, size: 14.5pt)

  #v(0.65em)
  #block(width: 100%, inset: (x: 16pt, y: 13pt), radius: 15pt, fill: blue-soft, stroke: 0.9pt + blue)[
    #grid(columns: (1fr, auto), gutter: 8pt, align: horizon)[
      #text(size: 13pt, weight: "bold", fill: blue)[owned `T::Encode` holds the physical state]
    ][
      #chip([DSA/IAX supplies the pending copy], fill: orange-soft, accent: orange, size: 8.8pt)
    ]
    #v(0.75em)
    #grid(columns: (1fr, 1fr, 1fr), gutter: 10pt)[
      #step([source bytes: read address stays valid], fill: white, accent: blue, size: 10pt)
    ][
      #step([`EncodeBuffer` storage: write address stays stable], fill: green-soft, accent: green, size: 10pt)
    ][
      #step([payload state: descriptor + completion stay pinned], fill: orange-soft, accent: orange, size: 10pt)
    ]
    #v(0.65em)
    #danger[If ownership is split, a Rust lifetime bug becomes hardware memory corruption.]
  ]

  #v(0.7em)
  #grid(columns: (1fr, 1fr, 1fr), gutter: 10pt)[
    #inv([source valid], [payload backing memory stays allocated while hardware may read it], accent: blue, fill: white)
  ][
    #inv(
      [destination stable],
      [`EncodeBuffer` storage does not move while hardware may write it],
      accent: green,
      fill: white,
    )
  ][
    #inv(
      [payload cleanup drained],
      [payload state's descriptor and completion are owned until Ready or drop cleanup],
      accent: orange,
      fill: white,
    )
  ]

  #v(1fr)
]
