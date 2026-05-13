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
