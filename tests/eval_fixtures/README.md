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
      "expected_headings": [{ "text": "Introduction", "level": 1 }],
      "expected_tables": 1
    }
  ]
}
```

Field notes:

- `page`: 1-indexed.
- `expected_formula_count`: total formula or formula-review blocks expected on this page.
- `expected_headings`: exact text, case-insensitive and trimmed, plus heading level.
- `expected_tables`: `1` if at least one table is expected, otherwise `0`.

## Adding A Fixture

1. Place the PDF in this directory or use a relative path to a local corpus PDF.
2. Run `pdfp inspect <pdf>` to identify page content.
3. Create a `.json` file with expectations for the pages you want to measure.
4. Run `pdfp eval tests/eval_fixtures/`.

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
