# Eval Framework (`src/eval/`)

Quality evaluation system that measures `pdfp`'s conversion accuracy against labelled JSON fixtures. Reports precision and recall for formulas, headings, tables, figures, and decorative images.

## Source files

| File | Purpose |
|---|---|
| `mod.rs` | Module root, re-exports |
| `fixtures.rs` | JSON fixture loading and schema |
| `metrics.rs` | Precision/recall computation for each signal type |
| `runner.rs` | Eval runner: iterates fixtures, invokes pipeline, compares output |

## Key types

| Type | Purpose |
|---|---|
| `FixtureCollection` | Collection of `FixtureEntry`s loaded from a directory |
| `FixtureEntry` | One fixture file: `pdf` path, vector of `FixturePage` expectations |
| `FixturePage` | Per-page expectations: `expected_formula_count`, `expected_headings`, `expected_tables`, `expected_table_regions`, `expected_decorative_images`, `expected_meaningful_figures`, `expected_figure_captions`, `expected_vector_only_regions`, `skip_text_metrics`, `skip_table_metrics` |
| `EvalReport` | Per-fixture evaluation results |
| `EvalSummary` | Aggregate report across all fixtures |

## Key functions

### Fixtures (`fixtures.rs`)

| Function | Description |
|---|---|
| `load_fixtures(fixtures_dir) -> Result<FixtureCollection>` | Loads all `.json` fixture files from a directory |
| `resolve_pdf_path(fixture_path) -> PathBuf` | Resolves the `pdf` field (relative path) against the fixture file location |

Fixture schema (see [docs/TESTING.md](../TESTING.md#evaluation-pdfp-eval) for full reference):
```json
{
  "pdf": "relative/path/to/pdf.pdf",
  "pages": [{
    "page": 1,
    "expected_formula_count": 2,
    "expected_formula_latex_snippets": ["E =", "\\sqrt"],
    "expected_headings": [{ "text": "Introduction", "level": 1 }],
    "expected_tables": 1,
    "expected_table_regions": [{ "x0": 40, "y0": 100, "x1": 500, "y1": 180 }],
    "expected_meaningful_figures": 1,
    "expected_decorative_images": 0
  }]
}
```

### Metrics (`metrics.rs`)

| Function | Description |
|---|---|
| `evaluate_page(page, fixture) -> PageMetrics` | Compare one page's blocks against fixture expectations |
| `evaluate_tables(page, fixture_pages) -> TableMetrics` | Page-level table recall/precision and optional region IoU |
| `evaluate_formulas(page, fixture_pages) -> FormulaMetrics` | Formula block recall/precision plus LaTeX snippet matching |
| `evaluate_heads(page, fixture_pages) -> HeadingMetrics` | Heading text and level accuracy |
| `evaluate_images(doc, fixture) -> ImageMetrics` | Figure retention, decorative suppression, caption pairing |

### Runner (`runner.rs`)

| Function | Description |
|---|---|
| `run_eval(fixtures_dir) -> PdfpResult<()>` | CLI entry point. Loads fixtures, converts each PDF, runs metrics, prints report |
| `run_eval_with_args(fixtures_dir, output) -> EvalSummary` | Internal version returning structured data |

## CLI flags

| Flag | Effect |
|---|---|
| `pdfp eval <FIXTURES_DIR>` | Run evaluation against labelled fixtures |

## Fixture files

Tracked in `tests/eval_fixtures/`:
- `engineering-calc.json` / `engineering-report.json` — heading, formula, and table floors
- `formula-*.json` — formula detection and recall fixtures (simple, heavy, numbered, fractions, roots, subscripts, sums)
- `hard-*.json` — hard document fixtures (attention, brochure, resnet-vector)
- `stage9-hard-images.json` — image-only benchmark pages
- `formula_corpus/` — Typst-sourced formula PDFs with known expectations

Regenerate: `scripts/generate-eval-fixtures.sh formula-corpus`

## Cross-references

- [docs/TESTING.md](../TESTING.md#evaluation-pdfp-eval) — fixture schema documentation
- [docs/TESTING.md](../TESTING.md#quality-improvement-loop) — quality improvement workflow
