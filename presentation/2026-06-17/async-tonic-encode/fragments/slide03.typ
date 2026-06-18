#import "../../../template.typ": *
#import "../support.typ": *

#let slide_03() = slide[
  #slide-head("03", [Borrowed Async Encode Cannot Be Stored])

  #v(0.45em)
  #grid(columns: (0.92fr, 1.08fr), gutter: 14pt)[
    #card(
      [Tempting but wrong],
      [
        #codeblock(
          raw(
            "async fn encode(\n  &mut self,\n  item,\n  dst: &mut EncodeBuf,\n) -> Result<()>",
            block: true,
            lang: "rust",
          ),
          size: 9pt,
        )
      ],
      fill: white,
      accent: red,
      body-size: 10pt,
    )
  ][
    #card(
      [What Tonic must store],
      [
        #align(center)[#text(size: 16pt, weight: "bold", fill: blue)[`in_flight: Option<T::Encode>`]]
        #v(0.5em)
        #grid(columns: (1fr, 1fr, 1fr), gutter: 7pt)[
          #chip([one future], fill: white, accent: blue, size: 9.6pt)
        ][
          #chip([per message], fill: white, accent: blue, size: 9.6pt)
        ][
          #chip([`Send + 'static`], fill: white, accent: blue, size: 9.6pt)
        ]
      ],
      fill: blue-soft,
      accent: blue,
      body-size: 10pt,
    )
  ]

  #v(0.65em)
  #block(width: 100%, inset: (x: 15pt, y: 12pt), radius: 14pt, fill: red-soft, stroke: (left: 4.5pt + red))[
    #text(size: 13.2pt, weight: "bold", fill: red)[Failure at `Pending`]
    #v(0.55em)
    #grid(columns: (1fr, auto, 1fr, auto, 1fr, auto, 1.05fr), gutter: 7pt, align: horizon)[
      #step([caller stack owns `&mut` borrows], fill: white, accent: red, size: 9.8pt)
    ][
      #text(size: 16pt, weight: "bold", fill: red)[→]
    ][
      #step([future returns `Pending`], fill: orange-soft, accent: orange, size: 9.8pt)
    ][
      #text(size: 16pt, weight: "bold", fill: red)[→]
    ][
      #step([future still borrows `dst` / `self`], fill: white, accent: red, size: 9.8pt)
    ][
      #text(size: 16pt, weight: "bold", fill: red)[→]
    ][
      #step([cannot store as `T::Encode: Send + 'static`], fill: white, accent: red, size: 9.8pt)
    ]
  ]

  #v(0.65em)
  #thesis([Correct storage unit: owned future + owned `EncodeBuffer`.], fill: green-soft, accent: green, size: 13.8pt)

  #v(1fr)
]
