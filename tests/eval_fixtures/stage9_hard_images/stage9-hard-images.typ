#set page(width: 210mm, height: 297mm, margin: 18mm)
#set text(font: "Arial", size: 10pt)
#set par(justify: true)

= Decorative Raster Stress

#v(8mm)

#align(center)[
  #image("../../../test-corpus/eval/stage9-assets/decorative-banner.png", width: 170mm)
]

#v(8mm)

This page intentionally contains a large decorative raster graphic with no
caption. It is useful because a converter should not treat this kind of page
background or masthead art as a retained meaningful figure.

#pagebreak()

= Captioned Figure Retention Stress

The next graphic is meaningful. It has a direct caption and should be retained
as a figure.

#v(8mm)

#align(center)[
  #image("../../../test-corpus/eval/stage9-assets/meaningful-chart.png", width: 150mm)
]

#align(center)[
  Figure 1: Parser stage throughput from extraction to Markdown rendering.
]

#pagebreak()

= Mixed Decorative And Meaningful Stress

#place(top + right, dx: -8mm, dy: 3mm)[
  #image("../../../test-corpus/eval/stage9-assets/decorative-banner.png", width: 42mm)
]

This page includes a small decorative page mark and a real figure. The desired
behavior is to retain the captioned figure while suppressing the decorative
page mark.

#v(8mm)

#align(center)[
  #image("../../../test-corpus/eval/stage9-assets/meaningful-chart.png", width: 135mm)
]

#align(center)[
  Figure 2: Coordinate-table arbitration keeps one best candidate per region.
]

#pagebreak()

= Vector-Only Region Stress

The diagram below is drawn from PDF vector primitives rather than an embedded
raster image. It should still be acknowledged when a figure caption identifies
the region.

#v(22mm)

#align(center)[
  #rect(width: 132mm, height: 52mm, stroke: 1.2pt + black, fill: rgb("#f8fafc"))[
    #pad(x: 8mm, y: 8mm)[
      #grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 10mm,
        align: center + horizon,
        [#rect(width: 30mm, height: 15mm, stroke: 1pt + black)[Extract]],
        [#rect(width: 30mm, height: 15mm, stroke: 1pt + black)[Classify]],
        [#rect(width: 30mm, height: 15mm, stroke: 1pt + black)[Render]],
      )
    ]
  ]
]

#align(center)[
  Figure 3: Vector-only pipeline diagram with no embedded image object.
]
