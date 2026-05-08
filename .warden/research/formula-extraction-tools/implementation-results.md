# Formula Extraction Implementation Results

Checked: 2026-05-05

## What Landed

- `pdfp convert` now accepts `--formulas auto|local|hybrid|off`.
- `pdfp convert` now accepts `--debug-formulas`.
- Local formula candidate detection uses MuPDF word geometry and equation-line heuristics.
- `--debug-formulas` writes page JSON and rendered equation crops under `debug/formulas/`.
- Formula candidates are represented as Markdown display formulas when locally usable and as hybrid-routing signals when `--hybrid docling` is enabled.
- Warden standard processing now requires formula coverage ledgers and keeps unresolved formula gaps in `draft` / `pdf_review_status: partial`.

## Fixture Results

| Fixture | Pages with JSON | Formula candidates | Crops written | Notes |
| --- | ---: | ---: | ---: | --- |
| `example/pdf/math-number-theory.pdf` | 24 | 171 | 171 | Local audit works across the math fixture; 67 high-confidence local candidates, 104 review candidates. |
| `DNV-ST-N001_2018 - Marine operations and marine warranty.pdf` | 699 | 3623 | 3623 | Full standard audit completed under `/tmp/pdfp-dnv-n001-formula-benchmark`; 1090 high-confidence local candidates, 2533 review candidates. |

## Backend Results

- Mock Docling hybrid test preserves display math returned by the backend.
- Live Docling was not running on `http://localhost:5001` during this pass, so live formula enrichment quality was not measured.

## Recommendation

Do not add UniMERNet/PDF-Extract-Kit yet. The new ledger shows the scale of DNV formula review and gives the standard-processing workflow enough visibility to avoid silent gaps. The next evidence step should be a live Docling run on selected DNV pages or chapter extracts. Add a local UniMERNet/PDF-Extract-Kit sidecar only if Docling fails to recover the high-value formulas or is too operationally heavy for the standards workflow.

## Commands Run

```bash
cargo run --quiet -- convert example/pdf/math-number-theory.pdf -o /tmp/pdfp-formula-benchmark --no-images --debug-formulas --formulas auto
timeout 180 cargo run --quiet -- convert "/home/eastill/projects/literature/standards/pdfs/marine-operations-lifting-transport/DNV-ST-N001_2018 - Marine operations and marine warranty.pdf" -o /tmp/pdfp-dnv-n001-formula-benchmark --no-images --debug-formulas --formulas auto
curl -fsS --max-time 2 http://localhost:5001/
```
