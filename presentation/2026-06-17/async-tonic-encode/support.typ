#import "../../template.typ": *
#import "@preview/zebraw:0.6.3": zebraw

#let bg-fill = rgb("#f7faff")
#let panel-stroke = rgb("#d9e7f7")
#let panel-fill = rgb("#fdfeff")
#let panel-soft = rgb("#f1f7ff")
#let blue-soft = rgb("#edf6ff")
#let orange-soft = rgb("#fff4e6")
#let red-soft = rgb("#fff1f2")
#let green-soft = rgb("#ecfdf5")
#let violet-soft = rgb("#f5f3ff")
#let ink = rgb("#243246")
#let ink-soft = rgb("#526173")
#let blue = palette.accent
#let orange = rgb("#f97316")
#let red = rgb("#dc2626")
#let green = rgb("#16a34a")
#let violet = rgb("#7c3aed")
#let gray = luma(120)

#let slide-head(n, title, tag: [async Tonic encode]) = [
  #grid(
    columns: (auto, 1fr, auto),
    gutter: 11pt,
    align: horizon,
    [#block(radius: 999pt, inset: (x: 10pt, y: 4pt), fill: blue)[#text(size: 9.5pt, weight: "bold", fill: white)[#n]]],
    [#text(size: 23pt, weight: "bold", fill: palette.title)[#title]],
    [#text(size: 8.8pt, fill: palette.muted)[#tag]],
  )
  #v(0.14em)
  #grid(
    columns: (0.26fr, 0.74fr),
    gutter: 0pt,
    [#rect(width: 100%, height: 1pt, radius: 999pt, fill: blue)],
    [#rect(width: 100%, height: 1pt, radius: 999pt, fill: rgb("#dbeafe"))],
  )
]

#let core-point(body, accent: blue, fill: panel-soft) = block(
  width: 100%,
  inset: (x: 13pt, y: 8pt),
  radius: 10pt,
  fill: fill,
  stroke: (left: 3.8pt + accent),
)[
  #text(size: 10.2pt, weight: "bold", fill: accent)[Core point]#h(0.75em)
  #text(size: 10.2pt, fill: ink-soft)[#body]
]


#let codeblock(body, size: 9pt) = {
  set text(font: "DejaVu Sans Mono", size: size, fill: luma(34))
  zebraw(
    numbering: false,
    extend: false,
    radius: 7pt,
    inset: (top: 6pt, bottom: 6pt, left: 8pt, right: 8pt),
    background-color: (white, rgb("#f8fafc")),
    body,
  )
}

#let card(
  title,
  body,
  fill: panel-fill,
  accent: blue,
  body-size: 10.6pt,
  title-size: 11pt,
  inset: (x: 13pt, y: 10pt),
) = block(
  width: 100%,
  inset: inset,
  radius: 11pt,
  fill: fill,
  stroke: 0.55pt + panel-stroke,
)[
  #grid(columns: (auto, 1fr), gutter: 7pt, align: horizon)[
    #rect(width: 3.6pt, height: 16pt, radius: 999pt, fill: accent)
  ][
    #text(size: title-size, weight: "bold", fill: palette.title)[#title]
  ]
  #v(0.38em)
  #text(size: body-size, fill: ink-soft)[#body]
]

#let big-card(title, body, fill: white, accent: blue, title-size: 17pt, body-size: 11.2pt) = block(
  width: 100%,
  inset: (x: 17pt, y: 14pt),
  radius: 14pt,
  fill: fill,
  stroke: 0.85pt + accent,
)[
  #align(center)[#text(size: title-size, weight: "bold", fill: accent)[#title]]
  #v(0.6em)
  #align(center)[#text(size: body-size, fill: ink)[#body]]
]

#let thesis(body, fill: green-soft, accent: green, size: 15pt) = block(
  width: 100%,
  radius: 13pt,
  inset: (x: 16pt, y: 10pt),
  fill: fill,
  stroke: 0.75pt + accent,
)[#align(center)[#text(size: size, weight: "bold", fill: palette.title)[#body]]]

#let chip(body, fill: blue-soft, accent: blue, size: 9.5pt) = block(
  radius: 999pt,
  inset: (x: 9pt, y: 4pt),
  fill: fill,
  stroke: 0.35pt + accent,
)[#text(size: size, weight: "bold", fill: accent)[#body]]

#let arrow = text(size: 18pt, weight: "bold", fill: palette.muted)[→]
#let down-arrow = text(size: 18pt, weight: "bold", fill: palette.muted)[↓]

#let step(body, fill: white, accent: blue, size: 10pt, inset: (x: 9pt, y: 7pt)) = block(
  width: 100%,
  inset: inset,
  radius: 9pt,
  fill: fill,
  stroke: 0.55pt + accent,
)[#align(center)[#text(size: size, weight: "bold", fill: ink)[#body]]]

#let danger(body) = block(
  width: 100%,
  inset: (x: 12pt, y: 8pt),
  radius: 9pt,
  fill: red-soft,
  stroke: (left: 3.8pt + red),
)[#text(size: 10.2pt, weight: "bold", fill: ink)[#body]]

#let two-col(left, right, gutter: 16pt, ratio: (1fr, 1fr)) = grid(
  columns: ratio,
  gutter: gutter,
  left,
  right,
)
