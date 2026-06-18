#import "../../../template.typ": *
#import "../support.typ": *

#let slide_05() = slide[
  #slide-head("05", [Driver Lifecycle: Reserve, Store, Poll, Finish])

  #v(0.45em)
  #card(
    [One message through `EncodedBytes`],
    [
      #grid(columns: (0.95fr, auto, 1.05fr, auto, 1.08fr, auto, 0.95fr, auto, 1fr), gutter: 5pt, align: horizon)[
        #step([1. reserve 5-byte header gap], fill: panel-soft, accent: gray, size: 8.9pt)
      ][#arrow][
        #step([2. move active `BytesMut` into `EncodeBuffer`], fill: blue-soft, accent: blue, size: 8.55pt)
      ][#arrow][
        #step([3. store `in_flight = Some(T::Encode)`], fill: blue-soft, accent: blue, size: 8.55pt)
      ][#arrow][
        #step([4. poll `Pending` / `Ready`], fill: orange-soft, accent: orange, size: 8.8pt)
      ][#arrow][
        #step([5. finish compression + header], fill: green-soft, accent: green, size: 8.8pt)
      ]
    ],
    fill: white,
    accent: blue,
    title-size: 13pt,
    body-size: 10pt,
    inset: (x: 15pt, y: 12pt),
  )

  #v(0.65em)
  #grid(columns: (1fr, 1fr, 1fr, 1fr), gutter: 8pt)[
    #card([reserve], [`buf.reserve(HEADER_SIZE)`], fill: panel-soft, accent: gray, body-size: 8.6pt, title-size: 9.5pt)
  ][
    #card([start], [`encoder.encode(item, dst)`], fill: blue-soft, accent: blue, body-size: 8.6pt, title-size: 9.5pt)
  ][
    #card([poll], [`ready!(encode.poll(cx))`], fill: orange-soft, accent: orange, body-size: 8.6pt, title-size: 9.5pt)
  ][
    #card([finish], [`finish_encode_buffer(...)`], fill: green-soft, accent: green, body-size: 8.6pt, title-size: 9.5pt)
  ]

  #v(0.55em)
  #core-point(
    [Frame finalization stays in Tonic: compression, max-size check, flags, and gRPC length are written after `Ready`.],
    accent: green,
    fill: green-soft,
  )

  #v(1fr)
]
