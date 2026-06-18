#import "../../../template.typ": *
#import "../support.typ": *

#let slide_11() = slide[
  #slide-head("11", [Measurement Is a Consequence, Not the Lesson])
  #v(0.45em)

  #thesis(
    [The architecture makes a cost removable; benchmarks decide whether the new async/offload costs are smaller.],
    fill: violet-soft,
    accent: violet,
    size: 10pt,
  )
  #v(0.65em)

  #grid(columns: (1.05fr, 0.95fr), gutter: 10pt)[
    #block(width: 100%, inset: (x: 10pt, y: 9pt), radius: 12pt, fill: white, stroke: 0.35pt + panel-stroke)[
      #align(center)[#text(size: 8pt, weight: "bold", fill: gray)[balance to measure, not promise]]
      #v(0.35em)
      #grid(columns: (1fr, auto, 1fr), gutter: 8pt, align: horizon)[
        #soft-card(
          [removable cost],
          [On the uncompressed path, async DSA can write directly into the bytes Tonic will publish, avoiding one CPU-managed staged payload copy.],
          fill: green-soft,
          accent: green,
          body-size: 7.45pt,
        )
      ][
        #block(width: 62pt)[
          #align(center)[
            #rect(width: 60pt, height: 3.2pt, radius: 999pt, fill: gray)
            #v(0.05em)
            #rect(width: 5pt, height: 26pt, radius: 2pt, fill: gray)
            #v(0.02em)
            #rect(width: 40pt, height: 4pt, radius: 999pt, fill: gray)
          ]
        ]
      ][
        #soft-card(
          [new costs],
          [Submission, completion, wakeup, retry, and field-resume bookkeeping still remain.],
          fill: orange-soft,
          accent: orange,
          body-size: 7.45pt,
        )
      ]
      #v(0.55em)
      #warning-band([Large payloads may amortize async costs; small or latency-bound encodes may lose.])
    ]
  ][
    #soft-card(
      [What the measurement is actually testing],
      [
        - Does avoided CPU staging outweigh offload overhead?
        - Does resume-state bookkeeping dominate small messages?
        - Does compression force the old staging boundary back in?
      ],
      fill: panel-fill,
      accent: violet,
      body-size: 7.75pt,
    )
    #v(0.55em)
    #block(
      width: 100%,
      inset: (x: 12pt, y: 12pt),
      radius: 11pt,
      fill: rgb("#0f172a"),
      stroke: 0.4pt + rgb("#334155"),
    )[
      #align(center)[
        #text(
          size: 10.2pt,
          weight: "bold",
          fill: white,
        )[Architecture lesson remains: owned async operation + exact protobuf resume state.]
      ]
    ]
  ]

  #v(1fr)
]
