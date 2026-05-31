// Shared support for the DSA submission bottleneck slide modules.

#import "../../template.typ": callout, deck, palette
#import "@preview/chronos:0.3.0" as chronos
#import "@preview/zebraw:0.6.3": *
#import "@preview/lilaq:0.6.0" as lq

#let c-title = palette.title
#let c-accent = palette.accent
#let c-row = palette.row
#let c-blue = palette.blue
#let c-green = palette.green
#let c-orange = palette.orange
#let c-red = palette.red

#let source-line(body) = text(size: 7.3pt, fill: luma(115))[#body]

#let soft-box(body, fill: c-row, stroke: palette.border, inset: (x: 14pt, y: 9pt)) = block(
  width: 100%,
  radius: 7pt,
  inset: inset,
  fill: fill,
  stroke: 0.55pt + stroke,
  body,
)

#let section-label(body, color: c-title) = text(size: 15.5pt, weight: "bold", fill: color)[#body]

#let phase(label, body, color) = [
  #text(weight: "bold", fill: color)[#label]
  #h(0.35em)
  #text(fill: luma(65))[#body]
]

#let compact-table(..args) = table(
  inset: (x: 8pt, y: 5.5pt),
  stroke: 0.42pt + palette.border,
  ..args,
)

#let metric-pill(body, color: c-title) = block(
  width: 100%,
  radius: 7pt,
  inset: (x: 10pt, y: 6pt),
  stroke: 0.55pt + palette.border,
  fill: white,
  text(font: "Latin Modern Mono", size: 10.2pt, fill: color)[#body],
)

#let chip(body, color: c-title) = text(weight: "bold", fill: color)[#body]

#let swatch(color) = box(width: 8pt, height: 8pt, fill: color, stroke: 0.35pt + luma(60))

#let fill-key(..items) = compact-table(
  columns: (1fr,),
  inset: (x: 7pt, y: 4pt),
  table.header([#text(size: 9.8pt, weight: "bold", fill: c-title)[lifeline fill = measured region]]),
  ..items,
)

#let fill-item(color, body) = [#swatch(color)#h(0.45em)#body]

#let seqbox(body) = scale(x: 74%, y: 74%)[#body]

#let code-note(body) = text(size: 8.6pt, fill: luma(90))[#body]

#let lq-diagram(..args) = lq.diagram(
  width: 7.4cm,
  height: 4.0cm,
  margin: 6%,
  legend: (position: left + top),
  xaxis: (subticks: none),
  yaxis: (subticks: none),
  ..args,
)

#let lq-plot = lq.plot
#let lq-bar = lq.bar
