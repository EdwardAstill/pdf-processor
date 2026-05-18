# Stage 7.5 Baseline

Updated: 2026-05-14

## Corpus

Ignored local PDFs:

- `test-corpus/eval/engineering-report-example.pdf`
  - source: `/home/eastill/projects/typst-templates/engineering-report/example.pdf`
  - profile: six-page tagged Typst engineering report with numbered headings, three tables, and one display equation.
- `test-corpus/eval/engineering-calc-example.pdf`
  - source: `/home/eastill/projects/typst-templates/engineering-calc/example.pdf`
  - profile: two-page tagged Typst calculation sheet with numbered/check headings, one input table, and calculation equations.

Tracked fixture JSON:

- `tests/eval_fixtures/engineering-report.json`
- `tests/eval_fixtures/engineering-calc.json`

## Runtime Gaps

- `pdfp doctor --json` reports OCR unavailable. This does not affect the current born-digital baseline PDFs.
- `ldconfig -p | rg -i pdfium` found no system `libpdfium`, so `pdfium-metadata` runtime extraction is expected to fall back.
- `/home/eastill/.local/share/pdfp` is absent, so RapidLaTeX-OCR ONNX model files are not installed.

## Baseline Results

Run:

```sh
target/debug/pdfp eval tests/eval_fixtures/
```

Results recorded after fixture creation. The missing `sample.pdf` warning is
emitted on stderr and may interleave with stdout because it exercises the skip
path.

```text
warning: detected 1 formula candidate(s) across 1 page(s); use `--debug-formulas` to inspect crops or `--hybrid docling --formulas hybrid` for formula enrichment.
eval: skipped sample.pdf: Failed to extract tests/eval_fixtures/sample.pdf: Failed to open PDF 'tests/eval_fixtures/sample.pdf': MuPDF error, code: 2, message: cannot open tests/eval_fixtures/sample.pdf: No such file or directory

engineering-calc-example.pdf
  pages evaluated:   2
  formula recall:    0.0% (0/12)
  heading accuracy:  0.0% (0/8)
  table recall:      100.0% (1/1)

engineering-report-example.pdf
  pages evaluated:   4
  formula recall:    0.0% (0/1)
  heading accuracy:  0.0% (0/13)
  table recall:      100.0% (3/3)

evaluated 2 document(s), skipped 1
```

## Precision Baseline Added In Stage 8

Stage 8 first extended `pdfp eval` to report precision and false-positive
counts. Immediately after that metric extension, before heading/formula tuning,
the same Stage 7.5 extraction behavior measured:

```text
engineering-calc-example.pdf
  pages evaluated:    2
  formula recall:     0.0% (0/12)
  formula precision:  100.0% (0/0, fp 0)
  heading accuracy:   0.0% (0/8)
  heading precision:  100.0% (0/0, fp 0)
  table recall:       100.0% (1/1)
  table precision:    50.0% (1/2, fp 1)

engineering-report-example.pdf
  pages evaluated:    4
  formula recall:     0.0% (0/1)
  formula precision:  100.0% (0/0, fp 0)
  heading accuracy:   0.0% (0/13)
  heading precision:  100.0% (0/0, fp 0)
  table recall:       100.0% (3/3)
  table precision:    75.0% (3/4, fp 1)
```

Precision floors for Stage 8:

- heading false positives: `0`
- formula false positives: `<= 1`
- table precision: calc `>=50.0%`, report `>=75.0%`

## Stage 8 Result

Run:

```sh
target/debug/pdfp eval tests/eval_fixtures/
```

Measured on 2026-05-15 after numbered-heading recovery, broad-layout formula
unblocking, and compact display-formula promotion:

```text
engineering-calc-example.pdf
  pages evaluated:    2
  formula recall:     100.0% (12/12)
  formula precision:  100.0% (12/12, fp 0)
  heading accuracy:   100.0% (8/8)
  heading precision:  100.0% (8/8, fp 0)
  table recall:       100.0% (1/1)
  table precision:    50.0% (1/2, fp 1)

engineering-report-example.pdf
  pages evaluated:    4
  formula recall:     100.0% (1/1)
  formula precision:  100.0% (1/1, fp 0)
  heading accuracy:   69.2% (9/13)
  heading precision:  100.0% (9/9, fp 0)
  table recall:       100.0% (3/3)
  table precision:    75.0% (3/4, fp 1)
```

Combined Stage 8 result:

- heading accuracy: `17/21` (81.0%), above minimum `11/21` and stretch `15/21`
- formula recall: `13/13` (100.0%), above minimum `4/13` and stretch `7/13`
- table recall: `4/4` (100.0%), floor preserved
- heading false positives: `0`, floor preserved
- formula false positives: `0`, floor preserved
- table precision: unchanged from precision baseline

## Stage 8.5 Result

Run:

```sh
cargo run --quiet -- eval tests/eval_fixtures/
```

Measured on 2026-05-15 after table evidence scoring, caption-anchored table
runs, best-candidate overlap arbitration, and weak layout-table quarantine:

```text
engineering-calc-example.pdf
  pages evaluated:    2
  formula recall:     100.0% (12/12)
  formula precision:  100.0% (12/12, fp 0)
  heading accuracy:   100.0% (8/8)
  heading precision:  100.0% (8/8, fp 0)
  table recall:       100.0% (1/1)
  table precision:    100.0% (1/1, fp 0)

  table region recall:    100.0% (1/1)
  table region precision: 50.0% (1/2, fp 1, fn 0)

engineering-report-example.pdf
  pages evaluated:    4
  formula recall:     100.0% (1/1)
  formula precision:  100.0% (1/1, fp 0)
  heading accuracy:   69.2% (9/13)
  heading precision:  100.0% (9/9, fp 0)
  table recall:       100.0% (3/3)
  table precision:    100.0% (3/3, fp 0)

  table region recall:    100.0% (3/3)
  table region precision: 100.0% (3/3, fp 0, fn 0)
```

Combined Stage 8.5 result:

- heading accuracy: `17/21` (81.0%), Stage 8 floor preserved
- formula recall: `13/13` (100.0%), Stage 8 floor preserved
- table page recall: `4/4` (100.0%), floor preserved
- table page precision: `4/4` (100.0%), improved from Stage 8 `4/6`
- table region recall: `4/4` (100.0%) on recorded expected boxes
- table region precision: `4/5` (80.0%) on recorded expected boxes
- calc page 2 and report page 4 emit no normal table blocks

## Stage 9 Image Benchmark Kickoff

Run:

```sh
cargo run --quiet -- eval tests/eval_fixtures/
```

Measured on 2026-05-15 after enabling snapshot figure extraction in eval and
adding harder local image fixtures for `attention.pdf`, the PDF/UA brochure,
and the vector-heavy `resnet.pdf`:

```text
attention.pdf
  pages evaluated:    2
  meaningful figure retention: 100.0% (2/2)
  figure-caption pairing: 100.0% (2/2)

golden__pdfua-1-reference-suite-1-1__PDFUA-Ref-2-06_Brochure.pdf
  pages evaluated:    2
  meaningful figure retention: 100.0% (3/3)
  figure-caption pairing: 100.0% (0/0)

resnet.pdf
  pages evaluated:    1
  meaningful figure retention: 100.0% (1/1)
  figure-caption pairing: 100.0% (1/1)
  vector-only acknowledgement: 100.0% (1/1)

engineering-calc-example.pdf
  pages evaluated:    2
  formula recall:     100.0% (12/12)
  heading accuracy:   100.0% (8/8)
  table recall:       100.0% (1/1)
  table precision:    100.0% (1/1, fp 0)

engineering-report-example.pdf
  pages evaluated:    4
  formula recall:     100.0% (1/1)
  heading accuracy:   69.2% (9/13)
  table recall:       100.0% (3/3)
  table precision:    100.0% (3/3, fp 0)

evaluated 5 document(s), skipped 1
```

Combined Stage 9 kickoff result:

- existing text/table floors still hold: headings `17/21`, formulas `13/13`,
  table pages `4/4`, table page precision `4/4`, table region precision `4/5`.
- meaningful figure retention is `6/6` across the three harder image fixtures.
- figure-caption pairing is `3/3` where captions are expected.
- vector-only acknowledgement is `1/1` on the first vector-heavy fixture page.
- decorative suppression is wired into eval, but current tracked hard fixtures
  have `0/0` for that category. It needs deliberately labeled non-zero pages
  before the proposed Stage 9 decorative target is meaningful.

## Interpretation

- Heading accuracy is now above the Stage 8 stretch floor on the local engineering fixtures, though the report fixture still misses four expected headings.
- Formula recall is now 13/13 on the tracked engineering fixtures; real ONNX formula OCR remains untested because local model files are absent.
- Table page precision is now 4/4. Region precision is separately tracked and currently 4/5 because calc page 1 still emits one extra definition/layout region on a page that also contains the expected table.
- Image/figure extraction is now part of `pdfp eval`; the current non-zero
  image baseline covers meaningful figure retention and caption pairing, not
  decorative or vector-only behavior yet.
- The existing `sample.json` still points at a missing sample PDF and verifies the skip path.

## Stage Targets Anchored Here

The numeric targets for Stages 8 / 8.5 / 9 / 10 are anchored to the numbers
in this file. See `.warden/plans/2026-05-13-next-stage-goals.md` for the
per-stage acceptance bands. Headline:

- Stage 8: heading accuracy >=11/21 (min) / >=15/21 (stretch); formula recall
  >=4/13 (min) / >=7/13 (stretch); table recall 4/4 floor. Precision metric
  must be added to `pdfp eval` and baselined before Stage 8 tuning lands.
- Stage 8.5: table page recall and precision 4/4; table region precision
  baseline 4/5 on recorded expected boxes.
- Stage 9: image metrics now exist in `pdfp eval`; current floors are 6/6
  meaningful figure retention, 3/3 caption pairing, and 1/1 vector-only
  acknowledgement on the hard image pages, while decorative suppression needs
  non-zero labeled fixtures before a hard floor.
- Stage 10: no new numeric targets; Stage 9 numbers become the floor and
  `pdfp eval` exits non-zero on regression.

## Feature Smoke

Run:

```sh
cargo run --features pdfium-metadata -- convert \
  test-corpus/eval/engineering-report-example.pdf \
  -o target/stage7-5-pdfium-smoke --no-images --verbose
```

Result:

- command exited 0 and wrote markdown.
- each page reported `pdfium-metadata: page N metadata unavailable`.
- root cause was `libpdfium.so: cannot open shared object file: No such file or directory`.

This confirms the feature-enabled path degrades explicitly on this machine
instead of silently claiming tagged-PDF metadata quality.
