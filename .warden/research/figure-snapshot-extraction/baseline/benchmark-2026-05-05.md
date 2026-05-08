# Figure Snapshot Benchmark

date: 2026-05-05
command shape: `cargo run --quiet -- convert <pdf> -o target/figure-benchmark/<mode>/<stem> --figures <mode> --figure-dpi 96`

## Results

| mode | pdf | status | ms | png count | png bytes | fig refs | img refs |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| embedded | attention.pdf | ok | 472 | 3 | 183950 | 0 | 3 |
| embedded | clip.pdf | ok | 2702 | 79 | 22089003 | 0 | 21 |
| embedded | resnet.pdf | ok | 407 | 0 | 0 | 0 | 0 |
| embedded | PDFUA-Ref-2-01_Magazine-danish.pdf | ok | 4845 | 387 | 59593998 | 0 | 52 |
| snapshot | attention.pdf | ok | 808 | 6 | 227875 | 6 | 0 |
| snapshot | clip.pdf | ok | 4604 | 43 | 2336165 | 41 | 0 |
| snapshot | resnet.pdf | ok | 865 | 18 | 781774 | 18 | 0 |
| snapshot | PDFUA-Ref-2-01_Magazine-danish.pdf | ok | 8873 | 50 | 25241663 | 31 | 0 |
| both | attention.pdf | ok | 814 | 9 | 411825 | 6 | 3 |
| both | clip.pdf | ok | 4621 | 122 | 24425168 | 39 | 21 |
| both | resnet.pdf | ok | 863 | 18 | 781774 | 18 | 0 |
| both | PDFUA-Ref-2-01_Magazine-danish.pdf | ok | 8904 | 437 | 84835661 | 22 | 32 |
| none | attention.pdf | ok | 476 | 0 | 0 | 0 | 0 |
| none | clip.pdf | ok | 2726 | 0 | 0 | 0 | 0 |
| none | resnet.pdf | ok | 392 | 0 | 0 | 0 | 0 |
| none | PDFUA-Ref-2-01_Magazine-danish.pdf | ok | 4856 | 0 | 0 | 0 | 0 |

## Findings

- Snapshot mode fixes a real known gap: `resnet.pdf` has 0 embedded images but snapshot mode emits 18 rendered figure regions.
- Snapshot mode reduces image spam on `clip.pdf` and the PDF/UA magazine fixture compared with embedded extraction, because blank candidates are skipped and decorative image objects are not dumped directly.
- Runtime roughly increases by 1.6-2.1x on this small fixture set at 96 DPI. That supports keeping `embedded` as the default and making snapshot mode explicit.
- `both` mode is useful for inspection, but it has the highest output size on image-heavy documents and should not be the default.
- `none` mode confirms media suppression is working and gives a text-only baseline.

## Next Tuning

- Add candidate-level quality review for `clip.pdf` and the magazine fixture; their lower Markdown ref counts compared with PNG counts indicate renderer assets can still be suppressed by the Markdown media planner.
- Consider adding snapshot-aware media-planner scoring so `BlockKind::Figure` snapshots are less likely to be capped on figure-dense scholarly pages.
- Add a permanent small fixture for vector-only figures so the `resnet.pdf` improvement is represented by a faster focused test.
