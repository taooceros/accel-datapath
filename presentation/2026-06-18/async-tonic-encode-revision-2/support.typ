#import "../../template.typ": *
#import "@preview/zebraw:0.6.3": zebraw

#let bg-fill = rgb("#f7faff")
#let panel-stroke = rgb("#dce8f7")
#let panel-fill = rgb("#fdfeff")
#let panel-soft = rgb("#f4f8ff")
#let warm-soft = rgb("#fff8f1")
#let green-soft = rgb("#f1fcf6")
#let ink-soft = luma(58)
#let blue = palette.accent
#let orange = rgb("#f97316")
#let red = rgb("#dc2626")
#let green = rgb("#16a34a")
#let violet = rgb("#7c3aed")
#let gray = luma(120)

#let slide-head(n, title) = [
  #grid(
    columns: (auto, 1fr, auto),
    gutter: 11pt,
    align: horizon,
    [#block(radius: 999pt, inset: (x: 10pt, y: 4pt), fill: blue)[#text(size: 9.5pt, weight: "bold", fill: white)[#n]]],
    [#text(size: 19.5pt, weight: "bold", fill: palette.title)[#title]],
    [#text(size: 8.8pt, fill: palette.muted)[async Tonic encode]],
  )
  #v(0.08em)
  #grid(
    columns: (0.22fr, 0.78fr),
    gutter: 0pt,
    [#rect(width: 100%, height: 1pt, radius: 999pt, fill: blue)],
    [#rect(width: 100%, height: 1pt, radius: 999pt, fill: rgb("#dbeafe"))],
  )
]

#let codeblock(body, size: 8.2pt) = {
  set text(font: "DejaVu Sans Mono", size: size, fill: luma(34))
  zebraw(
    numbering: false,
    extend: false,
    radius: 7pt,
    inset: (top: 5pt, bottom: 5pt, left: 7pt, right: 7pt),
    background-color: (white, rgb("#f8fafc")),
    body,
  )
}

#let side-by-side(left, right, ratio: (1fr, 1fr), gutter: 15pt) = grid(
  columns: ratio,
  gutter: gutter,
  left,
  right
)

#let card(title, body, fill: panel-fill, accent: blue, title-size: 10.5pt, body-size: 9.2pt) = block(
  width: 100%,
  inset: (x: 12pt, y: 9pt),
  radius: 10pt,
  fill: fill,
  stroke: 0.55pt + panel-stroke,
)[
  #grid(columns: (auto, 1fr), gutter: 6pt, align: horizon)[
    #rect(width: 3.2pt, height: 13pt, radius: 999pt, fill: accent)
  ][
    #text(size: title-size, weight: "bold", fill: palette.title)[#title]
  ]
  #v(0.3em)
  #text(size: body-size, fill: ink-soft)[#body]
]

#let thesis(body, fill: green-soft, accent: green, size: 12.8pt) = block(
  width: 100%,
  radius: 11pt,
  inset: (x: 14pt, y: 8pt),
  fill: fill,
  stroke: 0.65pt + accent,
)[#align(center)[#text(size: size, weight: "bold", fill: palette.title)[#body]]]

#let chip(body, fill: panel-soft, accent: blue, size: 8.8pt) = block(
  radius: 999pt,
  inset: (x: 8pt, y: 3.5pt),
  fill: fill,
  stroke: 0.35pt + accent,
)[#text(size: size, weight: "bold", fill: accent)[#body]]

#let checkmark = text(fill: green, weight: "bold")[✓]
#let cross = text(fill: red, weight: "bold")[✗]
#let arrow = text(size: 15pt, weight: "bold", fill: palette.muted)[→]
#let alert-box(body) = block(
  width: 100%,
  inset: (x: 12pt, y: 8pt),
  radius: 9pt,
  fill: rgb("#fff8f1"),
  stroke: (left: 3.5pt + orange),
)[#text(size: 9.2pt, fill: ink-soft)[#body]]
