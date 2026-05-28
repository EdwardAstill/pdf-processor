# Evaluation Fixtures

Each `.json` file describes one PDF's expected content.

## Schema

```json
{
  "pdf": "relative/path/to/file.pdf",
  "pages": [
    {
      "page": 1,
      "expected_formula_count": 2,
      "expected_formula_detection_count": 2,
      "expected_formula_latex_snippets": ["E =", "\\sqrt"],
      "formula_false_positive_budget": 0,
      "expected_headings": [{ "text": "Introduction", "level": 1 }],
      "expected_tables": 1,
      "expected_table_regions": [
        { "x0": 40.0, "y0": 100.0, "x1": 500.0, "y1": 180.0 }
      ],
      "expected_decorative_images": 0,
      "expected_meaningful_figures": 1,
      "expected_figure_captions": 1,
      "expected_vector_only_regions": 0,
      "skip_text_metrics": false,
      "skip_table_metrics": false
    }
  ]
}
```

Field notes:

- `page`: 1-indexed.
- `expected_formula_count`: total emitted formula or formula-review blocks expected on this page.
- `expected_formula_detection_count`: optional candidate count checked against `debug/formulas/index.json`.
- `expected_formula_latex_snippets`: optional snippets that should appear in recovered/emitted candidate LaTeX or source text.
- `formula_false_positive_budget`: allowed extra detected candidates before detection precision is penalized.
- `expected_headings`: exact text, case-insensitive and trimmed, plus heading level.
- `expected_tables`: `1` if at least one table is expected, otherwise `0`.
- `expected_table_regions`: optional expected table bboxes in page coordinates. When present,
  eval reports IoU-based table-region recall and precision in addition to page-level table metrics.
- `expected_decorative_images`: decorative images that should be suppressed.
- `expected_meaningful_figures`: meaningful image or figure regions that should be retained.
- `expected_figure_captions`: expected retained figures with paired captions.
- `expected_vector_only_regions`: vector-only regions that should be acknowledged by a figure snapshot or equivalent marker.
- `skip_text_metrics`: use for image-only benchmark pages so heading/formula expectations do not pollute Stage 8 floors.
- `skip_table_metrics`: use for image-only benchmark pages so table expectations do not pollute Stage 8.5 floors.

`pdfp eval` reports both recall and precision. Formula precision is
`matched / emitted_formula_blocks`; formula detection precision is based on
`debug/formulas/index.json` when `expected_formula_detection_count` is present;
LaTeX snippet recall checks `expected_formula_latex_snippets`. Heading precision is
`matched / emitted_heading_blocks`, and table precision is page-based:
`expected_table_pages_found / emitted_table_pages`. Table-region precision is
`matched_expected_regions / emitted_table_regions` using the fixture bboxes.
Image metrics report decorative suppression, meaningful figure retention,
figure-caption pairing, and vector-only acknowledgement when the corresponding
expectation fields are non-zero.

## Adding A Fixture

1. Place the PDF in this directory or use a relative path to a local corpus PDF.
2. Run `pdfp inspect <pdf>` to identify page content.
3. Create a `.json` file with expectations for the pages you want to measure.
4. Run `pdfp eval tests/eval_fixtures/`.

## Formula Corpus

The tracked formula corpus under `tests/eval_fixtures/formula_corpus/` contains
Typst source and generated PDFs for simple equations, numbered equations, and
fraction/sum/root/matrix-heavy equations. Regenerate them with:

```sh
scripts/generate-eval-fixtures.sh formula-corpus
```

These fixtures keep formula expectations non-zero in a fresh clone without the
ignored external engineering corpus.

## Stage 7.5 Local Baseline

The tracked Stage 7.5 fixtures point at ignored local PDFs:

- `../../test-corpus/eval/engineering-report-example.pdf`
- `../../test-corpus/eval/engineering-calc-example.pdf`

On this machine those PDFs were copied from:

```sh
mkdir -p test-corpus/eval
cp /home/eastill/projects/typst-templates/engineering-report/example.pdf \
  test-corpus/eval/engineering-report-example.pdf
cp /home/eastill/projects/typst-templates/engineering-calc/example.pdf \
  test-corpus/eval/engineering-calc-example.pdf
```

`test-corpus/` is ignored. Missing PDFs are reported as skipped so the JSON
fixtures can stay tracked without committing binary corpus files.

Stage 8.5 currently measures 17/21 headings, 13/13 formulas, 4/4 table-page
recall, and 4/4 table-page precision across these two fixtures. Heading and
formula false positives are 0. Table-region precision is 4/5 across the
recorded expected boxes because the calc fixture still emits one extra
definition/layout region on a true-table page.

Stage 9 adds harder local fixtures pointing at the ignored `example/pdf/`
corpus. These JSON files are tracked, while the PDFs remain untracked local
fixtures and skip cleanly when absent. Git worktrees do not automatically carry
ignored corpora, so link or copy `example/pdf/` into the worktree when you want
these hard fixtures evaluated instead of skipped.

Stage 9 also has a reproducible generated hard-image pack:

```sh
scripts/generate-eval-fixtures.sh stage9-hard-images
cargo run --quiet -- eval tests/eval_fixtures/
```

The generated PDF is written to ignored
`test-corpus/eval/stage9-hard-images.pdf` from tracked Typst/SVG sources under
`tests/eval_fixtures/stage9_hard_images/`.

With the generated hard pack present, current Stage 9 image totals are:

- decorative suppression: `1/2` on deliberately labeled decorative pages;
- meaningful figure retention: `8/9` across hard image pages;
- figure-caption pairing: `5/6`;
- vector-only acknowledgement: `1/2`.

Those numbers are intentionally harder than the kickoff baseline. They expose
the next image/vector tuning work while preserving the Stage 8 and Stage 8.5
text, formula, and table floors.
