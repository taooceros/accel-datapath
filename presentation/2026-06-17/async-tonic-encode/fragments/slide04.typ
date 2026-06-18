#import "../../../template.typ": *
#import "../support.typ": *

#let slide_04() = slide[
  #slide-head("04", [The New Tonic Boundary])

  #v(0.35em)
  #thesis(
    [`encode` moves the buffer into a future; `Ready` returns it for framing.],
    fill: green-soft,
    accent: green,
    size: 13.4pt,
  )

  #v(0.55em)
  #grid(columns: (1fr, 0.9fr), gutter: 14pt)[
    #card(
      [Encoder API],
      [
        #codeblock(
          raw(
            "type Encode: Future<Output = Result<EncodeBuffer, Self::Error>>\n  + Send + 'static\n\nfn encode(self: Pin<&mut Self>, item, dst: EncodeBuffer)\n  -> Result<Self::Encode, Self::Error>",
            block: true,
            lang: "rust",
          ),
          size: 9.8pt,
        )
      ],
      fill: blue-soft,
      accent: blue,
      body-size: 10pt,
      title-size: 12.5pt,
    )
  ][
    #card(
      [Body driver slot],
      [
        #codeblock(
          raw("#[pin]\nin_flight: Option<T::Encode>\nin_flight_offset: Option<usize>", block: true, lang: "rust"),
          size: 9pt,
        )
        #v(0.55em)
        #align(center)[#chip([stored future, not borrowed buffer], fill: white, accent: blue, size: 9.3pt)]
      ],
      fill: white,
      accent: blue,
      body-size: 10pt,
      title-size: 12pt,
    )
  ]

  #v(0.8em)
  #card(
    [Per-message lifecycle],
    [
      #grid(columns: (1fr, auto, 1fr, auto, 1fr), gutter: 8pt, align: horizon)[
        #step([start: move item + buffer in], fill: blue-soft, accent: blue, size: 10.2pt)
      ][#arrow][
        #step([poll: `Pending` keeps future stored], fill: orange-soft, accent: orange, size: 10.2pt)
      ][#arrow][
        #step([ready: buffer returns], fill: green-soft, accent: green, size: 10.2pt)
      ]
    ],
    fill: white,
    accent: green,
    title-size: 12.5pt,
    body-size: 10pt,
    inset: (x: 15pt, y: 12pt),
  )

  #v(1fr)
]
