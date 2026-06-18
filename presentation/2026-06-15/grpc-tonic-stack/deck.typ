#import "../../template.typ": *
#import "@preview/chronos:0.3.0"
#import "@preview/zebraw:0.6.3": zebraw

#let bg-fill = rgb("#f7faff")
#let panel-stroke = rgb("#dce8f7")
#let panel-fill = rgb("#fdfeff")
#let panel-soft = rgb("#f4f8ff")
#let warm-soft = rgb("#fff8f1")
#let green-soft = rgb("#f1fcf6")
#let ink-soft = luma(58)

#let diagram-frame(body) = block(
  width: 100%,
  inset: (x: 2pt, y: 2pt),
)[#body]

#let slide-head(body) = [
  #grid(
    columns: (1fr, auto),
    gutter: 10pt,
    align: horizon,
    [
      #grid(
        columns: (auto, 1fr),
        gutter: 9pt,
        align: horizon,
        [#rect(width: 4pt, height: 17pt, radius: 999pt, fill: palette.accent)],
        [#text(size: 18.7pt, weight: "bold", fill: palette.title)[#body]],
      )
    ],
    [#text(size: 6.8pt, fill: palette.muted)[tonic · h2 · tokio · DSA/IAX]],
  )
  #v(0.08em)
  #grid(
    columns: (0.18fr, 0.82fr),
    gutter: 0pt,
    [#rect(width: 100%, height: 0.75pt, radius: 999pt, fill: palette.accent)],
    [#rect(width: 100%, height: 0.75pt, radius: 999pt, fill: rgb("#dbeafe"))],
  )
]

#let legend-chip(glyph, label) = block(
  radius: 999pt,
  inset: (x: 7pt, y: 3pt),
  fill: white,
  stroke: 0.32pt + panel-stroke,
)[
  #grid(
    columns: (auto, auto),
    gutter: 4pt,
    align: horizon,
    [#glyph], [#text(size: 7.65pt, fill: ink-soft)[#label]],
  )
]

#let graph-legend = block(
  width: 100%,
  radius: 8pt,
  inset: (x: 7pt, y: 3.5pt),
  fill: panel-soft,
  stroke: 0.32pt + panel-stroke,
)[
  #grid(
    columns: (auto, auto, auto, 1fr),
    gutter: 5pt,
    align: horizon,
    [#legend-chip([#text(size: 10pt, fill: palette.muted)[┆]], [component lifeline])],
    [#legend-chip(
      [#rect(width: 3pt, height: 13pt, radius: 1.5pt, fill: luma(215), stroke: 0.15pt + luma(185))],
      [active call/future span],
    )],
    [#legend-chip([#text(size: 7.8pt, fill: palette.muted)[╌╌▶]], [return/data path])],
    [#align(right)[#text(
      size: 7.55pt,
      fill: palette.muted,
    )[Chronos notation: software lifelines, not Rust or packet lifetimes.]]],
  )
]

#let cpu-fill = luma(225)
#let iax-fill = rgb("#f97316")
#let dsa-fill = palette.accent
#let callout-fill = rgb("#334155")

#let cpu-style = (fill: cpu-fill)
#let iax-style = (fill: iax-fill)
#let dsa-style = (fill: dsa-fill)

#let stack-step-colors = (
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
)
#let send-a-step-colors = (callout-fill, callout-fill, callout-fill, callout-fill, callout-fill, callout-fill)
#let send-b-step-colors = (
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
  callout-fill,
)
#let recv-a-step-colors = (callout-fill, callout-fill, callout-fill, callout-fill, callout-fill, callout-fill)
#let recv-b-step-colors = (callout-fill, callout-fill, callout-fill, callout-fill, callout-fill, callout-fill)

#let step-color(colors, n) = if n <= colors.len() { colors.at(n - 1) } else { colors.last() }
#let step-number(colors) = n => text(weight: "bold", fill: step-color(colors, n))[#numbering("1.", n)]
#let step-marker(n, fill) = box(width: 10pt, height: 10pt, baseline: -2.1pt)[
  #block(
    width: 10pt,
    height: 10pt,
    radius: 999pt,
    inset: 0pt,
    fill: fill,
    stroke: 0.45pt + white,
  )[#align(center + horizon)[#text(size: 5.9pt, weight: "bold", fill: white)[#str(n)]]]
]
#let step-label(n, fill, body) = [#step-marker(n, fill)#h(0.16em)#body]

#let offload-chip(label, fill, tint: white) = block(
  radius: 999pt,
  inset: (x: 6.2pt, y: 2.4pt),
  fill: tint,
  stroke: 0.24pt + panel-stroke,
)[
  #grid(
    columns: (auto, auto),
    gutter: 3.8pt,
    align: horizon,
    [#rect(width: 3.1pt, height: 12pt, radius: 999pt, fill: fill, stroke: 0.12pt + palette.border)],
    [#text(size: 7.65pt, weight: "semibold", fill: ink-soft)[#label]],
  )
]

#let offload-legend = block(
  width: 100%,
  radius: 8pt,
  inset: (x: 7pt, y: 2.6pt),
  fill: panel-soft,
  stroke: 0.28pt + panel-stroke,
)[
  #grid(
    columns: (auto, auto, auto, 1fr),
    gutter: 5.6pt,
    align: horizon,
    [#offload-chip([orange = IAX compression], iax-fill, tint: warm-soft)],
    [#offload-chip([blue = DSA copy/CRC], dsa-fill, tint: rgb("#eef6ff"))],
    [#offload-chip([gray = CPU control], cpu-fill, tint: rgb("#f7f7f8"))],
    [#align(right)[#text(
      size: 7.15pt,
      fill: palette.muted,
    )[candidate spans, not participants]]],
  )
]


#let state-step(title, body, fill: white, accent: palette.accent) = block(
  width: 100%,
  radius: 9pt,
  inset: (x: 10pt, y: 7pt),
  fill: fill,
  stroke: 0.38pt + panel-stroke,
)[
  #text(size: 8.55pt, weight: "bold", fill: accent)[#title]
  #v(0.14em)
  #text(size: 7.65pt, fill: ink-soft)[#body]
]

#let side-explain(title, body, accent: palette.accent, fill: white) = block(
  width: 100%,
  inset: (x: 2pt, y: 3pt),
)[
  #grid(
    columns: (auto, 1fr),
    gutter: 6pt,
    align: horizon,
    [#rect(width: 3.2pt, height: 14pt, radius: 999pt, fill: accent)],
    [#text(size: 9.05pt, weight: "bold", fill: palette.title)[#title]],
  )
  #v(0.28em)
  #set text(size: 8.05pt, fill: ink-soft)
  #body
]

#let mini-note(title, body, accent: palette.accent, fill: panel-soft) = block(
  width: 100%,
  radius: 10pt,
  inset: (x: 9pt, y: 6pt),
  fill: fill,
  stroke: 0.34pt + panel-stroke,
)[
  #text(size: 7.85pt, weight: "bold", fill: accent)[#title]
  #v(0.12em)
  #text(size: 7.65pt, fill: ink-soft)[#body]
]

#let boundary-note(title, body, accent: palette.accent) = block(
  width: 100%,
  inset: (x: 0pt, y: 5pt),
  stroke: (top: 0.32pt + panel-stroke),
)[
  #grid(
    columns: (auto, 1fr),
    gutter: 6pt,
    align: horizon,
    [#text(size: 8.45pt, weight: "bold", fill: accent)[#title]], [#text(size: 8.45pt, fill: ink-soft)[#body]],
  )
]

#let candidate-badge(label, fill, fg: white) = block(
  radius: 999pt,
  inset: (x: 8pt, y: 3pt),
  fill: fill,
)[#text(size: 8.55pt, weight: "bold", fill: fg)[#label]]

#let evidence-note(body) = block(
  width: 100%,
  radius: 10pt,
  inset: (x: 12pt, y: 8pt),
  fill: warm-soft,
  stroke: (left: 3.2pt + iax-fill),
)[#text(size: 8.75pt, fill: ink-soft)[#body]]

#let flow-arrow = align(center + horizon)[#text(size: 15pt, weight: "bold", fill: luma(125))[→]]

#let shape-card(kicker, title, body, accent: palette.accent, fill: white) = block(
  width: 100%,
  radius: 13pt,
  inset: (x: 12pt, y: 12pt),
  fill: fill,
  stroke: 0.42pt + panel-stroke,
)[
  #text(size: 8.1pt, weight: "bold", fill: accent)[#kicker]
  #v(0.18em)
  #text(size: 13.6pt, weight: "bold", fill: palette.title)[#title]
  #v(0.24em)
  #text(size: 8.95pt, fill: ink-soft)[#body]
]

#let call-line(depth, body, root: false) = block(width: 100%)[
  #grid(
    columns: (auto, 1fr),
    gutter: 2.7pt,
    align: top,
    [
      #h(depth * 6.4pt)
      #if root {
        text(size: 6pt, fill: palette.accent)[●]
      } else {
        text(font: "DejaVu Sans Mono", size: 5.8pt, fill: luma(120))[└─]
      }
    ],
    [#text(font: "DejaVu Sans Mono", size: 6.25pt, fill: luma(38))[#body]],
  )
]

#let callstack-card(kicker, title, body, accent: palette.accent, fill: white) = block(
  width: 100%,
  radius: 10pt,
  inset: (x: 9pt, y: 6.4pt),
  fill: fill,
  stroke: 0.34pt + panel-stroke,
)[
  #block(width: 100%)[
    #grid(
      columns: (auto, 1fr),
      gutter: 5pt,
      align: horizon,
      [#rect(width: 3.1pt, height: 16pt, radius: 999pt, fill: accent)],
      [
        #text(size: 7.65pt, weight: "bold", fill: accent)[#kicker]
        #v(0.04em)
        #text(size: 8.85pt, weight: "bold", fill: palette.title)[#title]
      ],
    )
  ]
  #v(0.20em)
  #align(left)[#block(width: 100%)[#body]]
]

#let payload-cell(label, bytes, body, fill, accent) = block(
  width: 100%,
  radius: 5pt,
  inset: (x: 3.4pt, y: 3.4pt),
  fill: fill,
  stroke: 0.28pt + accent.lighten(45%),
)[
  #align(center)[
    #text(size: 6.15pt, weight: "bold", fill: accent)[#label]
    #v(0.03em)
    #text(size: 6.95pt, weight: "bold", fill: palette.title)[#bytes]
    #v(0.03em)
    #text(size: 5.75pt, fill: ink-soft)[#body]
  ]
]

#let envelope-anchor = block(
  width: 176pt,
  inset: (x: 1pt, y: 3pt),
)[
  #align(center)[#text(size: 7pt, weight: "bold", fill: iax-fill)[gRPC message frame · uncompressed]]
  #v(0.22em)
  #grid(
    columns: (0.68fr, 0.82fr, 1.50fr),
    gutter: 2.2pt,
    [
      #payload-cell([flag], [1 B], [`0`], rgb("#fff7ed"), iax-fill)
    ],
    [
      #payload-cell([length], [4 B], [`u32` BE], rgb("#eff6ff"), palette.accent)
    ],
    [
      #payload-cell([payload], [N B], [protobuf bytes], rgb("#ecfdf5"), rgb("#059669"))
    ],
  )
]

#let wire-arrow(label, reverse: false) = block(width: 100%)[
  #align(center + horizon)[
    #text(size: 8.6pt, fill: luma(45))[#label]
    #v(-0.10em)
    #text(size: 17pt, fill: luma(115))[#if reverse { [⟵] } else { [⟶] }]
  ]
]

#let down-arrow(label) = block(width: 100%)[
  #align(center + horizon)[
    #text(size: 8.6pt, fill: luma(45))[#label]
    #v(0.04em)
    #text(size: 17pt, fill: luma(115))[↓]
  ]
]

#let overview-section(number, title, body, accent: palette.accent, height: auto) = block(
  width: 100%,
  height: height,
  inset: (x: 0pt, y: 1pt),
)[
  #grid(
    columns: (auto, auto, 1fr),
    gutter: 5.8pt,
    align: horizon,
    [#text(size: 7.3pt, weight: "bold", fill: accent)[#number]],
    [#rect(width: 19pt, height: 0.55pt, radius: 999pt, fill: accent.lighten(35%))],
    [#text(size: 8.35pt, weight: "bold", fill: accent)[#title]],
  )
  #v(0.38em)
  #body
]

#let compact-code(body, size: 6.25pt) = {
  set text(font: "DejaVu Sans Mono", size: size, fill: luma(36))
  zebraw(
    numbering: false,
    extend: false,
    radius: 7pt,
    inset: (top: 4.5pt, bottom: 4.5pt, left: 6pt, right: 6pt),
    background-color: (white, rgb("#f8fafc")),
    body,
  )
}

#let work-bullet(title, body) = block(
  width: 100%,
  inset: (x: 0pt, y: 2pt),
)[
  #grid(
    columns: (auto, 1fr),
    gutter: 6pt,
    align: horizon,
    [#rect(width: 2pt, height: 11pt, radius: 999pt, fill: palette.accent)],
    [
      #text(size: 8.0pt, weight: "bold", fill: palette.title)[#title]
      #text(size: 8.0pt, fill: ink-soft)[ — #body]
    ],
  )
]

#let rpc-participants = {
  import chronos: *
  _par("App", display-name: "App")
  _par("Tonic", display-name: "tonic")
  _par("Prost", display-name: "prost")
  _par("Http", display-name: "HTTP/2\nstack")
  _par("Net", display-name: "tokio\nTCP/TLS")
}

#deck(
  margin: (x: 30pt, y: 20pt),
  size: 11.0pt,
  leading: 0.78em,
  spacing: 0.28em,
  footer: [#text(size: 8pt, fill: palette.muted)[tonic message stack]],
)[
  #set page(fill: bg-fill)

  #slide[
    #slide-head[1 · How user code uses tonic]
    #v(0.24em)
    #text(
      size: 9pt,
      fill: ink-soft,
    )[The public surface is generated client/server glue plus `Request<T>` / `Response<T>`.]
    #v(0.56em)

    #grid(
      columns: (1fr, auto, 1fr),
      gutter: 11pt,
      [
        #overview-section(
          [C],
          [Client: generated method],
          [
            #compact-code(
              raw(
                "let mut client = GreeterClient::connect(dst).await?;\n\nlet response = client\n  .say_hello(Request::new(HelloRequest { name }))\n  .await?;\n\nlet reply: HelloReply = response.into_inner();",
                block: true,
                lang: "rust",
              ),
              size: 9.65pt,
            )
            #v(0.46em)
            #text(
              size: 8.45pt,
              fill: ink-soft,
            )[The generated method hides the transport, codec, and HTTP/2 body machinery.]
          ],
          accent: palette.accent,
          height: 210pt,
        )
      ],
      [
        #align(center)[#rect(width: 0.34pt, height: 204pt, fill: panel-stroke)]
      ],
      [
        #overview-section(
          [S],
          [Server: generated service],
          [
            #compact-code(
              raw(
                "Server::builder()\n  .add_service(GreeterServer::new(MyGreeter))\n  .serve(addr)\n  .await?;\n\nimpl Greeter for MyGreeter {\n  async fn say_hello(&self, request: Request<HelloRequest>)\n    -> Result<Response<HelloReply>, Status>\n  { /* user handler */ }\n}",
                block: true,
                lang: "rust",
              ),
              size: 8.45pt,
            )
            #v(0.46em)
            #text(size: 8.45pt, fill: ink-soft)[The handler sees typed Rust values, not frames or DATA chunks.]
          ],
          accent: rgb("#059669"),
          height: 210pt,
        )
      ],
    )

    #v(0.72em)
    #boundary-note(
      [What stays below the generated API],
      [transport · codec · HTTP/2 body machinery · frames/DATA chunks],
      accent: palette.accent,
    )
  ]

  #slide[
    #slide-head[2 · Down the tonic/prost call stack]
    #v(0.24em)
    #text(
      size: 9pt,
      fill: ink-soft,
    )[The call stack is tonic/prost code. The `Grpc` names below are tonic runtime types, not a separate gRPC library.]
    #v(0.58em)

    #grid(
      columns: (1fr, auto, 1fr),
      gutter: 10.5pt,
      [
        #overview-section(
          [C],
          [Client request path],
          [
            #compact-code(
              raw(
                "GreeterClient::say_hello\n└─ tonic::client::Grpc::unary\n   └─ Grpc::streaming\n      └─ EncodeBody::new_client\n         └─ Body::poll_frame\n            └─ EncodeBody::poll_frame\n               └─ ProstEncoder::encode\n                  └─ prost::Message::encode",
                block: true,
              ),
              size: 9.15pt,
            )
          ],
          accent: palette.accent,
          height: 190pt,
        )
      ],
      [
        #align(center)[#rect(width: 0.34pt, height: 184pt, fill: panel-stroke)]
      ],
      [
        #overview-section(
          [S],
          [Server request path],
          [
            #compact-code(
              raw(
                "generated Service::call\n└─ tonic::server::Grpc::unary\n   └─ Streaming::new_request\n      └─ Streaming::message().await\n         └─ StreamingInner::poll_next\n            └─ poll_decode_chunk\n               └─ ProstDecoder::decode\n                  └─ user handler",
                block: true,
              ),
              size: 9.15pt,
            )
          ],
          accent: rgb("#059669"),
          height: 190pt,
        )
      ],
    )

    #v(0.70em)
    #boundary-note(
      [Response mirrors the same shape],
      [`server::Grpc::map_response` builds `EncodeBody::new_server`; the client returns through `Grpc::create_response` and `Streaming::message().await`.],
      accent: iax-fill,
    )
  ]

  #slide[
    #slide-head[3 · What tonic does underneath for gRPC]
    #v(0.24em)
    #text(
      size: 9pt,
      fill: ink-soft,
    )[This is protocol work implemented inside tonic, below the user-facing API and around prost.]
    #v(0.56em)

    #diagram-frame[
      #block(width: 100%, height: 190pt)[
        #align(center + horizon)[
          #grid(
            columns: (0.9fr, 1.45fr),
            gutter: 16pt,
            align: (center, horizon),
            [
              #align(center)[#scale(x: 158%, y: 158%, reflow: true)[#envelope-anchor]]
            ],
            [
              #block(
                width: 100%,
                inset: (x: 9pt, y: 2pt),
                stroke: (left: 0.42pt + panel-stroke),
              )[
                #text(size: 8.55pt, weight: "bold", fill: palette.title)[Protocol work inside tonic]
                #v(0.28em)
                #work-bullet([Lazy body polling], [bytes appear only when hyper/h2 polls tonic bodies.])
                #v(0.10em)
                #work-bullet([Protobuf payload], [`prost` serializes/deserializes typed messages.])
                #v(0.10em)
                #work-bullet([gRPC frame], [tonic writes/parses `[flag:1][len:4][payload]`.])
                #v(0.10em)
                #work-bullet(
                  [Compression boundary],
                  [optional compression/decompression runs over complete protobuf bytes.],
                )
                #v(0.10em)
                #work-bullet([HTTP/2 integration], [DATA carries bytes; trailers carry final status/metadata.])
              ]
            ],
          )
        ]
      ]
    ]

    #v(0.70em)
    #boundary-note(
      [Async caveat],
      [The logical stack crosses poll points: encode/decode work runs when the body is polled, not necessarily when the user first calls the generated method.],
      accent: palette.accent,
    )
  ]


  #let overview-legend-item(glyph, label) = grid(
    columns: (auto, auto),
    gutter: 4pt,
    align: horizon,
    [#glyph], [#text(size: 7.65pt, fill: ink-soft)[#label]],
  )

  #let overview-legend = block(width: 100%, inset: (x: 2pt, y: 1.5pt))[
    #grid(
      columns: (auto, auto, auto, 1fr),
      gutter: 9pt,
      align: horizon,
      [#overview-legend-item([#text(size: 10pt, fill: palette.muted)[┆]], [component lifeline])],
      [#overview-legend-item(
        [#rect(width: 3pt, height: 13pt, radius: 1.5pt, fill: luma(215), stroke: 0.15pt + luma(185))],
        [active call/future span],
      )],
      [#overview-legend-item([#text(size: 7.8pt, fill: palette.muted)[╌╌▶]], [return/data path])],
      [#align(right)[#text(
        size: 7.55pt,
        fill: palette.muted,
      )[Chronos notation: software lifelines, not Rust or packet lifetimes.]]],
    )
    #v(0.12em)
    #rect(width: 100%, height: 0.34pt, fill: panel-stroke)
  ]

  #let overview-diagram-area(kicker, trail, body) = block(width: 100%, inset: (x: 2pt, y: 0pt))[
    #grid(
      columns: (auto, 1fr, auto),
      gutter: 6pt,
      align: horizon,
      [#rect(width: 2.6pt, height: 11pt, radius: 999pt, fill: palette.accent)],
      [#text(size: 7.45pt, weight: "bold", fill: palette.accent)[#kicker]],
      [#text(size: 6.95pt, fill: palette.muted)[#trail]],
    )
    #v(0.16em)
    #rect(width: 100%, height: 0.34pt, fill: panel-stroke)
    #v(0.18em)
    #body
  ]

  #let overview-side-note(title, body, note-title, note-body, accent: palette.accent) = block(
    width: 100%,
    inset: (x: 3pt, y: 2pt),
  )[
    #grid(
      columns: (auto, 1fr),
      gutter: 6pt,
      align: horizon,
      [#rect(width: 2.8pt, height: 15pt, radius: 999pt, fill: accent)],
      [#text(size: 9.05pt, weight: "bold", fill: palette.title)[#title]],
    )
    #v(0.28em)
    #set text(size: 8.05pt, fill: ink-soft)
    #body
    #v(0.34em)
    #rect(width: 100%, height: 0.36pt, fill: panel-stroke)
    #v(0.22em)
    #text(size: 7.85pt, weight: "bold", fill: accent)[#note-title]
    #v(0.10em)
    #text(size: 7.65pt, fill: ink-soft)[#note-body]
  ]

  #slide[
    #slide-head[Send path: typed Rust becomes DATA only at the edge]
    #v(0.12em)
    #overview-legend
    #v(0.16em)

    #grid(
      columns: (1.85fr, 1.25fr),
      gutter: 13pt,
      [
        #overview-diagram-area([overview trace], [typed → protobuf → gRPC frame → DATA])[
          #set text(size: 7.0pt)
          #align(center)[#scale(x: 96%, y: 96%, reflow: true)[#chronos.diagram({
            import chronos: *
            rpc-participants

            _seq(
              "App",
              "Tonic",
              comment: step-label(1, stack-step-colors.at(0), "typed RPC call"),
              enable-dst: true,
            )
            _seq("Tonic", "Tonic", comment: step-label(2, stack-step-colors.at(1), "lazy Body"))
            _seq(
              "Tonic",
              "Prost",
              comment: step-label(3, stack-step-colors.at(2), "protobuf bytes"),
              enable-dst: true,
            )
            _seq("Prost", "Tonic", comment: "EncodeBuffer", dashed: true, disable-src: true)
            _seq("Tonic", "Tonic", comment: step-label(4, stack-step-colors.at(3), "gRPC frame"))
            _seq(
              "Tonic",
              "Http",
              comment: step-label(5, stack-step-colors.at(4), "HTTP/2 DATA"),
              enable-dst: true,
            )
            _seq(
              "Http",
              "Net",
              comment: step-label(6, stack-step-colors.at(5), "socket write"),
              enable-dst: true,
            )

            _seq("Net", "Http", comment: "write accepted", dashed: true, disable-src: true)
            _seq("Http", "Tonic", comment: "body polled", dashed: true, disable-src: true)
            _seq("Tonic", "App", comment: "request in flight", dashed: true, disable-src: true)
          })]]
        ]
      ],
      [
        #overview-side-note(
          [Numbers map to the graph],
          [
            #set text(size: 7.25pt)
            #set par(leading: 0.86em)
            #set enum(numbering: step-number(stack-step-colors))
            + *typed RPC call* — Generated client code starts from a Rust request value.
            + *lazy Body* — `tonic` stores request state; bytes appear only when the body is polled.
            + *protobuf bytes* — `prost` serializes fields into payload bytes; schema walk stays CPU.
            + *gRPC framing* — Tonic's codec adds `[flag:1][len:4]` and may compress the complete payload.
            + *HTTP/2 DATA* — Tower/Hyper/h2 frame body bytes as DATA.
            + *socket write* — Tokio/TLS/TCP moves bytes; protobuf semantics are gone.
          ],
          [Overview cut],
          [Tower/hyper/h2/tokio detail is collapsed so the representation changes stay visible; codec and async-poll mechanics reopen on later slides.],
          accent: palette.accent,
        )
      ],
    )
  ]
  #slide[
    #slide-head[Receive path: DATA becomes Rust only after deframe and decode]
    #v(0.12em)
    #overview-legend
    #v(0.16em)

    #grid(
      columns: (1.85fr, 1.25fr),
      gutter: 13pt,
      [
        #overview-diagram-area([reverse trace], [DATA → gRPC message → protobuf → Rust T])[
          #set text(size: 7.0pt)
          #align(center)[#scale(x: 96%, y: 96%, reflow: true)[#chronos.diagram({
            import chronos: *
            rpc-participants

            _seq(
              "App",
              "Tonic",
              comment: step-label(1, stack-step-colors.at(0), "app demand"),
              enable-dst: true,
            )
            _seq(
              "Tonic",
              "Http",
              comment: step-label(2, stack-step-colors.at(1), "poll body"),
              enable-dst: true,
            )
            _seq(
              "Http",
              "Net",
              comment: step-label(3, stack-step-colors.at(2), "socket read"),
              enable-dst: true,
            )

            _seq(
              "Net",
              "Http",
              comment: step-label(4, stack-step-colors.at(3), "DATA frames"),
              dashed: true,
              disable-src: true,
            )
            _seq("Http", "Tonic", comment: "Body frame", dashed: true, disable-src: true)
            _seq("Tonic", "Tonic", comment: step-label(5, stack-step-colors.at(4), "gRPC deframe"))
            _seq(
              "Tonic",
              "Prost",
              comment: step-label(6, stack-step-colors.at(5), "protobuf decode"),
              enable-dst: true,
            )
            _seq("Prost", "Tonic", comment: "T", dashed: true, disable-src: true)
            _seq("Tonic", "App", comment: "Response<T> / Status", dashed: true, disable-src: true)
          })]]
        ]
      ],
      [
        #overview-side-note(
          [Numbers map to the graph],
          [
            #set text(size: 7.25pt)
            #set par(leading: 0.86em)
            #set enum(numbering: step-number(stack-step-colors))
            + *app demand* — The app asks `tonic` for the next response message.
            + *poll body* — `tonic` polls the HTTP body when no complete message is buffered.
            + *socket read* — The lower stack may need Tokio/TLS/TCP bytes before a frame can return.
            + *DATA frames* — HTTP/2 DATA and trailers return upward; flow control stays lower.
            + *gRPC deframe* — `tonic` parses the 5-byte prefix and waits for the announced payload.
            + *protobuf decode* — Optional decompression happens first; then `prost` rebuilds typed `T`.
          ],
          [Reverse shape],
          [DATA bytes become one gRPC-framed message, then protobuf payload bytes, then a typed Rust value or a separate status path.],
          accent: palette.accent,
        )
      ],
    )
  ]

  #slide[
    #let stage-label(accent, body) = grid(
      columns: (auto, 1fr),
      gutter: 5pt,
      align: horizon,
      [#rect(width: 2.6pt, height: 17pt, radius: 999pt, fill: accent)], [#body],
    )
    #let table-head(body) = text(size: 8.7pt, weight: "bold", fill: palette.title)[#body]
    #let opportunity-line(kicker, title, body, accent) = block(width: 100%, inset: (x: 4pt, y: 3pt))[
      #grid(
        columns: (auto, 1fr),
        gutter: 7pt,
        align: top,
        [#rect(width: 2.8pt, height: 43pt, radius: 999pt, fill: accent)],
        [
          #text(size: 7.25pt, weight: "bold", fill: accent)[#kicker]
          #v(0.08em)
          #text(size: 10.45pt, weight: "bold", fill: palette.title)[#title]
          #v(0.15em)
          #text(size: 8.55pt, fill: ink-soft)[#body]
        ],
      )
    ]

    #slide-head[Compression first; copies only when large; control stays CPU]
    #v(0.20em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 13pt,
      [
        #opportunity-line(
          [01 · compression first],
          [IAX/IAA is the strongest fit],
          [Use it only where `tonic` already runs gRPC compression/decompression over complete protobuf bytes, and only when the configured gRPC encoding matches hardware constraints.],
          iax-fill,
        )
      ],
      [
        #opportunity-line(
          [02 · copies only when large],
          [DSA is workload dependent],
          [Try `data_move` or `copy_crc` only for large unavoidable contiguous copies: large `bytes`/`string` append, body assembly, or copy+CRC.],
          dsa-fill,
        )
      ],
      [
        #opportunity-line(
          [03 · control stays CPU],
          [Do not offload semantics],
          [`prost` varints/schema traversal, Tower policy, HTTP/2 flow control, and Tokio scheduling are branchy control paths, not bulk transforms.],
          rgb("#059669"),
        )
      ],
    )

    #v(0.22em)
    #rect(width: 100%, height: 0.42pt, fill: panel-stroke)
    #v(0.34em)

    #block(width: 100%, inset: (x: 2pt, y: 3pt))[
      #grid(
        columns: (1fr, auto),
        gutter: 8pt,
        align: horizon,
        [
          #grid(
            columns: (auto, 1fr),
            gutter: 6pt,
            align: horizon,
            [#rect(width: 3.4pt, height: 16pt, radius: 999pt, fill: palette.accent)],
            [#text(size: 8.85pt, weight: "bold", fill: palette.title)[Offload surface by stage]],
          )
        ],
        [#text(size: 7.2pt, fill: palette.muted)[ordered by fit, not blanket acceleration]],
      )
      #v(0.20em)
      #rect(width: 100%, height: 0.42pt, fill: panel-stroke)
      #v(0.24em)
      #set text(size: 8.75pt)
      #set table(inset: (x: 7pt, y: 5pt))
      #table(
        fill: zebra-fill,
        stroke: 0.18pt + rgb("#edf3fb"),
        columns: (0.94fr, 0.62fr, 1.18fr, 1.46fr),
        align: horizon,
        [#table-head[Stage]], [#table-head[Candidate]], [#table-head[Why]], [#table-head[Caveat]],

        [#stage-label(iax-fill)[`compress` / `decompress`]],
        [#candidate-badge([IAX/IAA], iax-fill)],
        [bulk byte transform at explicit tonic boundary],
        [must match gRPC compression format and IAX/IAA Deflate limits],

        [#stage-label(dsa-fill)[large `bytes`/`string` append]],
        [#candidate-badge([DSA], dsa-fill)],
        [payload copy can be contiguous],
        [large payload only; tags/varints/schema walk stay CPU],

        [#stage-label(dsa-fill)[body assembly or copy/CRC]],
        [#candidate-badge([DSA], dsa-fill)],
        [bulk memory move or `copy_crc`],
        [only if copy remains unavoidable and completion cost is amortized],

        [#stage-label(luma(190))[`prost` tags/varints/decode]],
        [#candidate-badge([CPU], luma(228), fg: luma(45))],
        [schema-dependent branches],
        [keep schema semantics on CPU],

        [#stage-label(luma(190))[h2/Tokio/Tower control]],
        [#candidate-badge([CPU], luma(228), fg: luma(45))],
        [state machines, flow control, scheduling],
        [measure for attribution, not offload],
      )
    ]

    #v(0.32em)
    #evidence-note[
      Evidence grounding: repo architecture notes rank compression/decompression highest and DSA copy/CRC conditional; current Tonic+DSA notes only support trying DSA around ~1 MiB+ payloads, with 2–4 MiB more promising but still directional.
    ]
  ]

  #let detail-diagram-frame(body) = block(
    width: 100%,
    inset: (x: 3pt, y: 4pt),
    stroke: (top: 0.24pt + panel-stroke, bottom: 0.24pt + panel-stroke),
  )[#body]

  #let detail-sequence(title, accent, body) = block(
    width: 100%,
    inset: (left: 8pt, right: 0pt, top: 1pt, bottom: 1pt),
    stroke: (left: 2.4pt + accent),
  )[
    #text(size: 8.75pt, weight: "bold", fill: palette.title)[#title]
    #v(0.26em)
    #set text(size: 7.25pt, fill: ink-soft)
    #set par(leading: 0.86em, spacing: 0.12em)
    #body
  ]

  #let detail-caveat(title, body, accent: palette.accent) = block(
    width: 100%,
    inset: (left: 7pt, right: 0pt, top: 0pt, bottom: 0pt),
    stroke: (left: 1.4pt + accent),
  )[
    #text(size: 7.55pt, weight: "bold", fill: accent)[#title]
    #v(0.10em)
    #text(size: 7.05pt, fill: ink-soft)[#body]
  ]

  #let pending-state-fill = rgb("#64748b")
  #let pending-state-soft = rgb("#f8fafc")

  #let detail-gap-label(body) = box(baseline: -1.4pt)[
    #block(
      radius: 999pt,
      inset: (x: 4.2pt, y: 1.5pt),
      fill: pending-state-soft,
      stroke: 0.24pt + panel-stroke,
    )[#text(size: 6.4pt, weight: "semibold", fill: pending-state-fill)[#body]]
  ]

  #let detail-invariant-box(title, body, accent: palette.title) = block(
    width: 100%,
    radius: 9pt,
    inset: (x: 8pt, y: 6.5pt),
    fill: panel-soft,
    stroke: 0.34pt + panel-stroke,
  )[
    #text(size: 8.7pt, weight: "bold", fill: accent)[#title]
    #v(0.24em)
    #set text(size: 7.25pt, fill: ink-soft)
    #set par(leading: 0.86em, spacing: 0.10em)
    #body
  ]

  #let detail-state(title, body, accent: palette.accent, fill: panel-fill) = block(
    width: 100%,
    radius: 10pt,
    inset: (left: 9pt, right: 7pt, top: 4.5pt, bottom: 4.5pt),
    fill: fill,
    stroke: (
      left: 2.6pt + accent,
      top: 0.32pt + panel-stroke,
      right: 0.32pt + panel-stroke,
      bottom: 0.32pt + panel-stroke,
    ),
  )[
    #text(size: 9.35pt, weight: "bold", fill: accent)[#title]
    #v(0.10em)
    #text(size: 7.25pt, fill: ink-soft)[#body]
  ]

  #slide[
    #slide-head[Send detail A: tonic encode decision points]
    #v(0.08em)
    #offload-legend
    #v(0.10em)

    #grid(
      columns: (2.55fr, 0.95fr),
      gutter: 14pt,
      [
        #detail-diagram-frame[
          #set text(size: 6.7pt)
          #align(center)[#scale(x: 88%, y: 88%, reflow: true)[#chronos.diagram({
            import chronos: *

            _par("Src", display-name: "request\nstream")
            _par("Enc", display-name: "EncodedBytes")
            _par("Buf", display-name: "BytesMut")
            _par("Prost", display-name: "prost")
            _par("Comp", display-name: "compression.rs")
            _par("Body", display-name: "h2 body\npoller")
            _col("Src", "Enc", margin: 10)
            _col("Enc", "Buf", margin: 12)
            _col("Buf", "Prost", margin: 14)
            _col("Prost", "Comp", margin: 12)
            _col("Comp", "Body", margin: 10)

            _seq(
              "Body",
              "Enc",
              comment: step-label(1, send-a-step-colors.at(0), "h2 asks for DATA"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq(
              "Enc",
              "Src",
              comment: step-label(2, send-a-step-colors.at(1), "pull next message"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Src", "Enc", comment: "typed T", dashed: true, disable-src: true)
            _seq(
              "Enc",
              "Prost",
              comment: step-label(3, send-a-step-colors.at(2), "prost encode fields"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq(
              "Prost",
              "Buf",
              comment: step-label(4, send-a-step-colors.at(3), "large bytes/string copy"),
              enable-dst: true,
              lifeline-style: dsa-style,
            )
            _seq("Buf", "Prost", comment: "payload slice appended", dashed: true, disable-src: true)
            _seq("Prost", "Enc", comment: "protobuf bytes ready", dashed: true, disable-src: true)

            _seq(
              "Enc",
              "Comp",
              comment: step-label(5, send-a-step-colors.at(4), "gRPC compression"),
              enable-dst: true,
              lifeline-style: iax-style,
            )
            _seq("Comp", "Enc", comment: "compressed protobuf bytes", dashed: true, disable-src: true)
            _seq(
              "Enc",
              "Buf",
              comment: step-label(6, send-a-step-colors.at(5), "write flag + length"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Buf", "Enc", comment: "gRPC prefix complete", dashed: true, disable-src: true)
            _seq("Enc", "Body", comment: "freeze Bytes for h2", dashed: true, disable-src: true)
          })]]
        ]
      ],
      [
        #detail-sequence(
          [Decision sequence],
          dsa-fill,
          [
            #set enum(numbering: step-number(send-a-step-colors))
            + *Downstream pull* — h2 polls tonic for DATA.
            + *Message pull* — Tonic pulls one Rust message.
            + *Protobuf* — `prost` walks schema and writes fields on CPU.
            + *Large field copy* — DSA only for big `bytes`/`string` appends.
            + *Compression* — gRPC compression runs after protobuf bytes exist; IAX/IAA candidate.
            + *Write prefix* — The flag/length prefix and h2 handoff stay CPU.
          ],
        )
        #v(0.24em)
        #detail-caveat(
          [Boundary rule],
          [Candidate colors stay inside software lifelines. DSA copy/CRC only matters if a large post-encode copy remains; hardware is not a participant.],
          accent: palette.accent,
        )
      ],
    )
  ]

  #slide[
    #slide-head[Async encode: two polls, one owned operation]
    #v(0.10em)
    #offload-legend
    #v(0.08em)

    #grid(
      columns: (1fr, auto, 1fr, auto, 1fr),
      gutter: 7pt,
      align: horizon,
      [#detail-state(
        [First poll],
        [Own buffers, submit IAX compression, then yield `Poll::Pending`.],
        accent: palette.title,
        fill: rgb("#f7fbff"),
      )],
      [#flow-arrow],
      [#detail-state(
        [In flight / Pending],
        [Executor is free; `OwnedPendingOp` + completion record own the operation.],
        accent: pending-state-fill,
        fill: pending-state-soft,
      )],
      [#flow-arrow],
      [#detail-state(
        [Second poll],
        [Observe status/length; emit DATA on success or surface the error.],
        accent: rgb("#047857"),
      )],
    )

    #v(0.10em)

    #grid(
      columns: (2.75fr, 0.85fr),
      gutter: 12pt,
      [
        #detail-diagram-frame[
          #set text(size: 6.65pt)
          #align(center)[#scale(x: 95%, y: 84%, reflow: true)[#chronos.diagram({
            import chronos: *

            _par("Tokio", display-name: "tokio/h2")
            _par("Tonic", display-name: "tonic\nBody")
            _par("Op", display-name: "OwnedPendingOp\n+ buffers")
            _par("Cpl", display-name: "completion\nrecord")
            _par("Body", display-name: "HTTP body")
            _col("Tokio", "Tonic", margin: 12)
            _col("Tonic", "Op", margin: 18)
            _col("Op", "Cpl", margin: 8)
            _col("Cpl", "Body", margin: 17)

            _seq(
              "Tokio",
              "Tonic",
              comment: step-label(1, send-b-step-colors.at(0), "h2 polls for DATA"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Tonic", "Tonic", comment: step-label(2, send-b-step-colors.at(1), "own buffers"))
            _seq(
              "Tonic",
              "Op",
              comment: step-label(3, send-b-step-colors.at(2), "submit IAX compression"),
              enable-dst: true,
              lifeline-style: iax-style,
            )
            _seq(
              "Tonic",
              "Tokio",
              comment: step-label(4, send-b-step-colors.at(3), "return Pending"),
              dashed: true,
              disable-src: true,
            )

            _seq(
              "Op",
              "Cpl",
              comment: detail-gap-label([async gap: executor free; buffers owned by state]),
              dashed: true,
              enable-dst: true,
              disable-src: true,
              lifeline-style: (fill: pending-state-fill),
            )
            _seq(
              "Cpl",
              "Tokio",
              comment: step-label(5, send-b-step-colors.at(4), "status/len; wake"),
              dashed: true,
              disable-src: true,
            )

            _seq(
              "Tokio",
              "Tonic",
              comment: "second poll after wake",
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq(
              "Tonic",
              "Cpl",
              comment: step-label(6, send-b-step-colors.at(5), "observe status"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Cpl", "Tonic", comment: "ok / error", dashed: true, disable-src: true)
            _seq(
              "Tonic",
              "Body",
              comment: step-label(7, send-b-step-colors.at(6), "emit DATA"),
              dashed: true,
              disable-src: true,
            )
          })]]
        ]
      ],
      [
        #detail-invariant-box(
          [Owned async state invariant],
          [
            - Own src/dst buffers across `Poll::Pending`.
            - Completion record stores status/length and wakes the body task.
            - Second poll observes success/error; success emits DATA.
          ],
          accent: pending-state-fill,
        )
        #v(0.18em)
        #detail-caveat(
          [DSA caveat],
          [DSA `copy_crc` uses this same two-poll shape for large copies; it is not another numbered flow or hardware participant.],
          accent: dsa-fill,
        )
      ],
    )
  ]

  #slide[
    #slide-head[Receive detail A: frame accumulation and deframe]
    #v(0.12em)
    #offload-legend
    #v(0.16em)

    #grid(
      columns: (2.55fr, 0.95fr),
      gutter: 14pt,
      [
        #detail-diagram-frame[
          #set text(size: 6.55pt)
          #align(center)[#scale(x: 95%, y: 90%, reflow: true)[#chronos.diagram({
            import chronos: *

            _par("App", display-name: "App")
            _par("Stream", display-name: "Streaming\nInner")
            _par("Body", display-name: "http Body")
            _par("Buf", display-name: "BytesMut\nstaging")
            _par("State", display-name: "deframe\nstate")
            _par("Trail", display-name: "trailers\nstatus")
            _col("App", "Stream", margin: 10)
            _col("Stream", "Body", margin: 12)
            _col("Body", "Buf", margin: 12)
            _col("Buf", "State", margin: 14)
            _col("State", "Trail", margin: 10)

            _seq(
              "App",
              "Stream",
              comment: step-label(1, recv-a-step-colors.at(0), "app demand"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Stream", "Stream", comment: "poll_decode_chunk loop")
            _seq(
              "Stream",
              "Body",
              comment: step-label(2, recv-a-step-colors.at(1), "body.poll_frame(cx)"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq(
              "Body",
              "Stream",
              comment: step-label(3, recv-a-step-colors.at(2), "DATA Bytes"),
              dashed: true,
              disable-src: true,
            )
            _seq(
              "Stream",
              "Buf",
              comment: step-label(4, recv-a-step-colors.at(3), "append DATA / DSA candidate"),
              enable-dst: true,
              lifeline-style: dsa-style,
            )
            _seq("Buf", "Stream", comment: "staged bytes", dashed: true, disable-src: true)
            _seq(
              "State",
              "Buf",
              comment: step-label(5, recv-a-step-colors.at(4), "read flag + len"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq(
              "Stream",
              "Buf",
              comment: step-label(6, recv-a-step-colors.at(5), "wait until len bytes"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Buf", "Stream", comment: "full message slice", dashed: true, disable-src: true)
            _seq("Stream", "App", comment: "message or Status", dashed: true, disable-src: true)
          })]]
        ]
      ],
      [
        #detail-sequence(
          [Deframe sequence],
          dsa-fill,
          [
            #set enum(numbering: step-number(recv-a-step-colors))
            + *Demand* — The app poll starts receive work.
            + *Poll body* — No frame means `Pending`, not a spin loop.
            + *DATA bytes* — HTTP/2 DATA arrives from the body.
            + *Append bytes* — DSA only if fragmented assembly becomes a large copy.
            + *Read prefix* — Tonic parses the 5-byte flag/length on CPU.
            + *Wait body* — Wait for `len` bytes; trailers/status stay metadata.
          ],
        )
        #v(0.20em)
        #detail-caveat(
          [Boundary rule],
          [Candidate colors stay inside software lifelines. DSA/IAX are not deframe participants, and trailers/status stay CPU metadata.],
          accent: palette.accent,
        )
      ],
    )
  ]

  #slide[
    #slide-head[Receive detail B: decompress and typed decode]
    #v(0.12em)
    #offload-legend
    #v(0.16em)

    #grid(
      columns: (2.55fr, 0.95fr),
      gutter: 14pt,
      [
        #detail-diagram-frame[
          #set text(size: 6.45pt)
          #align(center)[#scale(x: 96%, y: 92%, reflow: true)[#chronos.diagram({
            import chronos: *

            _par("App", display-name: "App")
            _par("Stream", display-name: "Streaming\nInner")
            _par("Buf", display-name: "msg\nbuffer")
            _par("Copy", display-name: "optional\ngather/copy")
            _par("Comp", display-name: "compression.rs\ncodec")
            _par("Out", display-name: "decoded\nbytes")
            _par("Prost", display-name: "prost\nMessage")
            _col("App", "Stream", margin: 8)
            _col("Stream", "Buf", margin: 10)
            _col("Buf", "Copy", margin: 12)
            _col("Copy", "Comp", margin: 12)
            _col("Comp", "Out", margin: 12)
            _col("Out", "Prost", margin: 10)

            _seq("App", "Stream", comment: "poll_next(cx)", enable-dst: true, lifeline-style: cpu-style)
            _seq(
              "Stream",
              "Buf",
              comment: step-label(1, recv-b-step-colors.at(0), "remaining() >= len?"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Buf", "Stream", comment: "full message body", dashed: true, disable-src: true)

            _seq(
              "Stream",
              "Copy",
              comment: step-label(2, recv-b-step-colors.at(1), "optional gather/copy"),
              enable-dst: true,
              lifeline-style: dsa-style,
            )
            _seq("Copy", "Stream", comment: "contiguous bytes", dashed: true, disable-src: true)

            _seq(
              "Stream",
              "Comp",
              comment: step-label(3, recv-b-step-colors.at(2), "decompress if needed"),
              enable-dst: true,
              lifeline-style: iax-style,
            )
            _seq("Comp", "Out", comment: "uncompressed payload", dashed: true, enable-dst: true, disable-src: true)
            _seq("Out", "Stream", comment: "payload ready", dashed: true, disable-src: true)

            _seq(
              "Stream",
              "Prost",
              comment: step-label(4, recv-b-step-colors.at(3), "decode protobuf into T"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Prost", "Stream", comment: "typed Rust value", dashed: true, disable-src: true)
            _seq(
              "Stream",
              "Buf",
              comment: step-label(5, recv-b-step-colors.at(4), "advance to next message"),
              enable-dst: true,
              lifeline-style: cpu-style,
            )
            _seq("Buf", "Stream", comment: "ReadHeader gate", dashed: true, disable-src: true)
            _seq(
              "Stream",
              "App",
              comment: step-label(6, recv-b-step-colors.at(5), "return T"),
              dashed: true,
              disable-src: true,
            )
          })]]
        ]
      ],
      [
        #detail-sequence(
          [Decode sequence],
          iax-fill,
          [
            #set enum(numbering: step-number(recv-b-step-colors))
            + *Full message* — A complete gRPC message body is buffered.
            + *Gather/copy* — DSA only for large fragmented payloads.
            + *Decompress* — Offloaded decompression can yield `Pending` and wake later.
            + *Decode* — `prost` rebuilds Rust `T` on CPU.
            + *Advance* — The buffer moves to the next message gate.
            + *Return* — The stream yields the typed message to the app.
          ],
        )
        #v(0.20em)
        #detail-caveat(
          [Scope note],
          [Colored spans stay inside software lifelines. Offloaded copy/decompress may yield `Pending` and wake later; benchmark `batch_n` separately from concurrency.],
          accent: palette.accent,
        )
      ],
    )
  ]
]
