#import "../../../template.typ": *
#import "../support.typ": *

#let slide_02() = slide[
  #slide-head("02", [Sync Encode: Complete Body, Then Frame])

  #v(0.35em)
  #thesis(
    [Sync Tonic frames only after body bytes are complete and the `&mut EncodeBuf` borrow is gone.],
    fill: green-soft,
    accent: green,
    size: 13.1pt,
  )

  #v(0.55em)
  #card(
    [Completed-buffer contract],
    [
      #grid(columns: (0.85fr, auto, 1.1fr, auto, 0.95fr, auto, 1.05fr, auto, 0.9fr), gutter: 6pt, align: horizon)[
        #step([message], fill: white, accent: gray, size: 10.5pt)
      ][#arrow][
        #step([borrow `&mut EncodeBuf`], fill: blue-soft, accent: blue, size: 10.2pt)
      ][#arrow][
        #step([write full body], fill: blue-soft, accent: blue, size: 10.5pt)
      ][#arrow][
        #step([return complete bytes], fill: green-soft, accent: green, size: 10.2pt)
      ][#arrow][
        #step([Tonic frames], fill: panel-soft, accent: gray, size: 10.5pt)
      ]
    ],
    fill: white,
    accent: green,
    body-size: 11pt,
    title-size: 13pt,
    inset: (x: 15pt, y: 13pt),
  )

  #v(0.75em)
  #grid(columns: (1.05fr, 0.95fr), gutter: 14pt)[
    #card(
      [Code evidence],
      [
        #codeblock(
          raw("fn encode(&mut self, item, dst: &mut EncodeBuf) -> Result<()>", block: true, lang: "rust"),
          size: 9.7pt,
        )
        #v(0.45em)
        #text(size: 10.5pt)[The `&mut` borrow ends when the call returns.]
      ],
      fill: panel-fill,
      accent: blue,
      body-size: 10.5pt,
    )
  ][
    #card(
      [After return],
      [
        #grid(columns: (1fr, 1fr), gutter: 7pt)[
          #chip([size-check], fill: white, accent: gray, size: 9.4pt)
        ][
          #chip([optional compression], fill: white, accent: gray, size: 9.4pt)
        ][
          #chip([flags + length], fill: white, accent: gray, size: 9.4pt)
        ][
          #chip([yield frame], fill: white, accent: green, size: 9.4pt)
        ]
      ],
      fill: panel-soft,
      accent: gray,
      body-size: 10.5pt,
    )
  ]

  #v(1fr)
]
