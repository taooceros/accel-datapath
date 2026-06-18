#import "../../../template.typ": *
#import "../support.typ": *

#let phase(label, detail, fill: white, accent: violet) = block(
  width: 100%,
  inset: (x: 8pt, y: 8pt),
  radius: 10pt,
  fill: fill,
  stroke: 0.6pt + accent,
)[
  #align(center)[
    #text(size: 10.4pt, weight: "bold", fill: palette.title)[#label]
    #v(0.22em)
    #text(size: 8.4pt, fill: ink-soft)[#detail]
  ]
]

#let bad(label) = block(
  width: 100%,
  inset: (x: 7pt, y: 5pt),
  radius: 8pt,
  fill: red-soft,
  stroke: 0.5pt + red,
)[#align(center)[#text(size: 8.5pt, weight: "bold", fill: red)[#label]]]

#let slide_08() = slide[
  #slide-head("08", [One Suspended Length-Delimited Field])

  #v(0.45em)
  #block(width: 100%, inset: (x: 14pt, y: 12pt), radius: 15pt, fill: violet-soft, stroke: 0.85pt + violet)[
    #text(size: 13pt, weight: "bold", fill: violet)[Checkpoint after payload copy returns `Pending`]
    #v(0.65em)
    #grid(columns: (1fr, auto, 1fr, auto, 1fr, auto, 1.1fr, auto, 1.15fr), gutter: 5pt, align: horizon)[
      #phase([field N selected], [cursor chooses this field], fill: white, accent: violet)
    ][#arrow][
      #phase([tag emitted], [do not write again], fill: white, accent: violet)
    ][#arrow][
      #phase([length emitted], [do not write again], fill: white, accent: violet)
    ][#arrow][
      #phase([payload copy `Pending`], [copy stopped mid-payload], fill: orange-soft, accent: orange)
    ][#arrow][
      #phase([next poll resumes payload only], [legal continuation], fill: green-soft, accent: green)
    ]
    #v(0.48em)
    #grid(columns: (1fr, 1fr, 1fr, 1.1fr), gutter: 8pt)[
      #bad([wrong field: skip/repeat])
    ][
      #bad([wrong phase: duplicate tag])
    ][
      #bad([wrong phase: duplicate length])
    ][
      #bad([wrong offset: corrupt payload])
    ]
  ]

  #v(0.75em)
  #thesis(
    [The checkpoint is protocol state: tag and length stay written; only the unfinished payload copy continues.],
    fill: green-soft,
    accent: green,
    size: 13.2pt,
  )

  #v(1fr)
]
