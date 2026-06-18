#import "../../../template.typ": *
#import "../support.typ": *

#let slide_01() = slide[
  #slide-head("01", [Async Encode Was an Ownership Redesign])

  #v(0.42em)
  #thesis([Async encode was an ownership redesign.], fill: green-soft, accent: green, size: 17pt)

  #v(0.65em)
  #grid(columns: (1fr, auto, 1fr), gutter: 14pt, align: horizon)[
    #big-card(
      [SYNC],
      [
        #codeblock(raw("encode(item, &mut EncodeBuf)", block: true, lang: "rust"), size: 10.5pt)
        #v(0.45em)
        #text(size: 11pt)[borrow caller buffer]
        #v(0.25em)
        #text(size: 11pt, weight: "bold", fill: green)[return after body bytes exist]
      ],
      fill: blue-soft,
      accent: blue,
    )
  ][
    #align(center)[
      #text(size: 27pt, weight: "bold", fill: orange)[→]
      #v(0.25em)
      #chip([Pending can happen], fill: orange-soft, accent: orange, size: 9.2pt)
    ]
  ][
    #big-card(
      [ASYNC],
      [
        #codeblock(raw("encode(item, EncodeBuffer)\n  -> T::Encode", block: true, lang: "rust"), size: 10.5pt)
        #v(0.45em)
        #text(size: 11pt)[move buffer into future]
        #v(0.25em)
        #text(size: 11pt, weight: "bold", fill: green)[Ready returns EncodeBuffer]
      ],
      fill: green-soft,
      accent: green,
    )
  ]

  #v(0.8em)
  #grid(columns: (1fr, 1fr), gutter: 12pt)[
    #card(
      [Tonic stores],
      [one owned `T::Encode` per in-flight message],
      fill: white,
      accent: blue,
      body-size: 12pt,
    )
  ][
    #card(
      [Prost owns],
      [exact field / tag / length / payload resume state inside that future],
      fill: white,
      accent: violet,
      body-size: 12pt,
    )
  ]

  #v(1fr)
]
