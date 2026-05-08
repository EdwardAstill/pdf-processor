# PDF Quality Improvement Loop

This repo should improve PDF conversion by running the same loop each time:

1. Baseline the current examples.
2. Observe where output quality breaks down.
3. Research one failure class.
4. Change one algorithm or threshold.
5. Re-run the same audit and cargo checks.
6. Record the result before starting the next change.

The aim is not to make every PDF perfect in one pass. The aim is to stop guessing and make tables, formulas, images, scans, and reading order improve with evidence.

## Baseline Command

Run the example audit from the repo root:

```bash
bash scripts/example-audit.sh
```

The script writes:

- `target/example-audit/summary.md`
- `target/example-audit/quality/report.json`
- `target/example-audit/debug/<fixture>/...`

If you run from a worktree that does not have the ignored PDF fixtures, point it at the main checkout's corpus:

```bash
PDFP_AUDIT_CORPUS=/home/eastill/projects/pdf-processor/example/pdf \
  bash scripts/example-audit.sh
```

Use a custom fixture set when focusing on one failure class:

```bash
PDFP_AUDIT_FIXTURES="golden__issue-336-conto-economico-bialetti.pdf math-number-theory.pdf" \
  bash scripts/example-audit.sh
```

## Sidecar Comparison Command

Use the sidecar audit when comparing the local path against external backends:

```bash
bash scripts/sidecar-audit.sh
```

The script always runs the native `pdfp` conversion. Optional backends are skipped unless available:

- `docling` checks `PDFP_SIDECAR_DOCLING_URL` (default `http://localhost:5001`).
- `gmft` uses `PDFP_GMFT_COMMAND` or a `gmft` command on `PATH`.
- `img2table` uses `PDFP_IMG2TABLE_COMMAND` or an `img2table` command on `PATH`.
- `unimernet` uses `PDFP_UNIMERNET_COMMAND` or a `unimernet` command on `PATH`.

Outputs are written under `target/sidecar-audit/` with one directory per backend and a summary at `target/sidecar-audit/summary.md`. External command wrappers receive two arguments: the input PDF path and the output directory. This keeps GPL/AGPL or large-model tools outside the default build while still making experiments repeatable.

## Current Baseline

Last local run on 2026-05-08 used the top-level `example/pdf` corpus:

| Signal | Result |
| --- | ---: |
| PDFs converted | 22 |
| Hard conversion failures | 0 |
| Quality warnings | 7 |
| Main warning classes | glued numeric rows, high heading density, high image density |

Representative debug cases:

| PDF | Formula candidates | Table candidates | Figure candidates | Image files | Read |
| --- | ---: | ---: | ---: | ---: | --- |
| `attention.pdf` | 90 | 1 | 10 | 9 | Good scholarly baseline; figure extraction works, formulas need enrichment if exact LaTeX is required. |
| `math-number-theory.pdf` | 171 | 3 | 16 | 19 | Formula-heavy stress case; many candidates are expected, but output still needs visual formula review. |
| `golden__issue-336-conto-economico-bialetti.pdf` | 9 | 3 | 0 | 0 | Biggest table problem: financial rows partly align, then some long labels and numeric columns glue together. |
| `golden__chinese_scan.pdf` | 0 | 0 | 1 | 2 | Correctly detected as scan-heavy; local output is image-only and should route to OCR or hybrid. |

## Failure Classes

### Tables

Current table reconstruction is strongest on simple invoices and short tables. It still struggles on financial statements and long numeric tables where:

- row labels wrap across multiple text blocks;
- year columns are visually aligned but not reliably separated;
- subtotal/category rows look like equations because of `+`, `-`, parentheses, and dense numbers;
- later sections fall back to mixed list, table, and fixed-width text.

Research lanes:

- Camelot uses separate Stream, Lattice, Network, and Hybrid parsers, which is a useful model for choosing a table strategy per page: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html
- pdfplumber combines explicit PDF lines with implied word-alignment lines and exposes table debug objects, which maps well to this repo's debug JSON direction: https://github.com/jsvine/pdfplumber/blob/stable/README.md#extracting-tables
- Docling exposes table structure options, including table structure extraction mode and cell matching, as a reference backend for hard cases: https://docling-project.github.io/docling/reference/pipeline_options/

Next table experiments:

1. Add a financial-table suppression rule so formula detection ignores rows already inside strong table regions.
2. Score table candidates with separate `line-grid`, `text-alignment`, and `layout-fallback` strategies.
3. Add a snapshot fixture for the Bialetti page section that currently glues numeric rows.

### Formulas

Formula detection is useful as an audit lane, but it is not yet formula recognition. Current local mode detects candidate regions and crops them for review. Exact LaTeX should come from optional enrichment.

Current weakness:

- financial rows can be false positives;
- multi-line equations need grouping and crop review;
- local Markdown cannot guarantee semantic math output.

Research lanes:

- Docling formula enrichment can extract LaTeX representations for detected formulas: https://docling-project.github.io/docling/usage/enrichments/
- UniMERNet targets real-world mathematical expression recognition and provides a benchmark/model direction for formula crops: https://arxiv.org/abs/2404.15254
- Nougat treats academic-page OCR as markup generation, useful for comparing whole-page scientific conversion against local extraction: https://arxiv.org/abs/2308.13418

Next formula experiments:

1. Suppress candidates that overlap high-confidence table regions.
2. Add a small visual review set of formula crops and expected action: keep, merge, suppress, or hybrid.
3. Compare Docling formula-enriched output on the same crop/page set before adding any new local recognizer.

### Images, Figures, And Scans

Born-digital academic figures are mostly detectable with `--figures both`; scan-only pages are not. A scan-heavy page should not pretend to be text extraction.

Current weakness:

- scanned pages produce image links but no useful text without OCR;
- high image-density layouts such as magazines and brochures still need layout grouping;
- vector-drawn diagrams and captions need more review than embedded raster extraction.

Research lanes:

- Docling picture classification and description can classify picture items and optionally describe them through local or remote vision models: https://docling-project.github.io/docling/usage/enrichments/
- Docling OCR and pipeline options document the cost and setup of enabling OCR, table structure, and formulas only where needed: https://docling-project.github.io/docling/reference/pipeline_options/

Next image/OCR experiments:

1. Keep local default conservative: warn on scan-heavy PDFs and suggest `--ocr auto` or `--hybrid docling`.
2. Add a scan audit fixture that asserts image-only output remains flagged.
3. Benchmark OCR/hybrid output on scan fixtures before changing local rendering.

## Change Gate

Before accepting a quality change:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo check --features pdfium-metadata
bash scripts/example-audit.sh
bash scripts/sidecar-audit.sh
```

If a change is intended to improve only one class, the other classes must not regress in `target/example-audit/summary.md` or `target/example-audit/quality/report.json`.

## Record Template

Append short notes to the relevant plan or assessment after each loop:

```markdown
## YYYY-MM-DD - <change>

- Hypothesis:
- Files changed:
- Corpus:
- Before:
- After:
- Regressions:
- Next:
```
