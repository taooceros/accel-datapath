#let palette = (
  title: rgb("#1e3a5f"),
  accent: rgb("#2563eb"),
  head: rgb("#eff6ff"),
  row: rgb("#f8fafc"),
  blue: rgb("#f0f9ff"),
  orange: rgb("#fff7ed"),
  green: rgb("#ecfdf5"),
  red: rgb("#fef2f2"),
  border: luma(185),
  muted: luma(120),
)

#let deck(
  body,
  margin: (x: 52pt, y: 42pt),
  font: "Latin Modern Sans",
  size: 17pt,
  leading: 0.9em,
  spacing: 1em,
  table-stroke: 0.4pt + luma(205),
  table-inset: 8pt,
) = {
  set page(
    paper: "presentation-16-9",
    margin: margin,
  )
  set text(font: font, size: size)
  set par(leading: leading, spacing: spacing)
  set table(stroke: table-stroke, inset: table-inset)
  body
}

#let slide-title(body, accent: palette.accent, fill: palette.title) = {
  block(width: 100%, below: 14pt, stroke: (bottom: 2.5pt + accent))[
    #text(size: 24pt, weight: "bold", fill: fill)[#body]
    #v(4pt)
  ]
}

#let zebra-fill(x, y, head: palette.head, row: palette.row) = {
  if y == 0 { head } else if calc.odd(y) { white } else { row }
}

#let callout(body, fill: palette.blue, stroke: palette.accent, inset: (x: 16pt, y: 11pt)) = block(
  width: 100%,
  radius: 5pt,
  inset: inset,
  fill: fill,
  stroke: (left: 3.5pt + stroke),
  body,
)

#let note(body) = callout(body, fill: palette.orange, stroke: rgb("#f97316"))

#let panel(body, width: auto, fill: luma(247), radius: 5pt, inset: (x: 16pt, y: 14pt)) = block(
  width: width,
  inset: inset,
  fill: fill,
  radius: radius,
  body,
)

#let card(title, body, fill: white, stroke: 0.6pt + palette.border, body-size: 13pt) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 12pt, y: 10pt),
  fill: fill,
  stroke: stroke,
)[
  #text(weight: "bold")[#title]
  #v(0.25em)
  #text(size: body-size, fill: luma(70))[#body]
]

#let stage-card(
  title,
  body,
  fit,
  fill: white,
  accent: palette.accent,
  stroke: 0.6pt + palette.border,
  body-size: 12.5pt,
  fit-size: 11pt,
) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 12pt, y: 10pt),
  fill: fill,
  stroke: stroke,
)[
  #text(weight: "bold")[#title]
  #v(0.2em)
  #text(size: body-size, fill: luma(70))[#body]
  #v(0.5em)
  #block(width: 100%, inset: (x: 8pt, y: 4pt), radius: 4pt, fill: accent)[
    #align(center)[#text(size: fit-size, weight: "bold", fill: white)[#fit]]
  ]
]

#let fit-badge(label, fill: palette.accent) = block(
  radius: 999pt,
  inset: (x: 10pt, y: 4pt),
  fill: fill,
)[#text(size: 11pt, weight: "bold", fill: white)[#label]]
