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

## Interpretation

- Heading accuracy is currently the weakest measured signal on the local engineering fixtures.
- Formula recall is also zero on display/calc math, partly because broad table-region detection can suppress formula candidates.
- Table recall is 100%, but this is only recall. Debug table output shows whole-page table regions on these Typst fixtures, so Stage 8/9 should not treat this as table precision.
- The existing `sample.json` still points at a missing sample PDF and verifies the skip path.

## Stage Targets Anchored Here

The numeric targets for Stages 8 / 9 / 10 are anchored to the recall numbers
in this file. See `.warden/plans/2026-05-13-next-stage-goals.md` for the
per-stage acceptance bands. Headline:

- Stage 8: heading accuracy >=11/21 (min) / >=15/21 (stretch); formula recall
  >=4/13 (min) / >=7/13 (stretch); table recall 4/4 floor. Precision metric
  must be added to `pdfp eval` and baselined before Stage 8 tuning lands.
- Stage 9: introduce decorative-image suppression, meaningful-figure
  retention, and figure-caption pairing metrics; baseline first, then lock
  targets in the stage doc.
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
