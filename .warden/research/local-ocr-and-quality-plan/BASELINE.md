# Current Baseline

Date run: 2026-05-05

Command:

```bash
cargo build --bin cnv
CNV_QUALITY_CORPUS=example/pdf \
CNV_QUALITY_OUT=.warden/research/local-ocr-and-quality-plan/baseline/current-quality \
  bash scripts/quality-report.sh
```

Result:

- Recursive corpus: 44 PDFs, 44 passed, 0 failed.
- Top-level `example/pdf`: 22 PDFs, 22 passed, 0 failed.
- Full report: `baseline/current-quality/report.json`.
- Top-level summary: `baseline/top-level-summary.json`.

## Notable Cases

| PDF | Pages | Warnings | Images | Empty pages | Table markers | Headings | Baseline implication |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `golden__chinese_scan.pdf` | 1 | 1 | 1 | 0 | 0 | 0 | Image-only Markdown; primary OCR acceptance fixture. |
| `golden__pdfua-1-reference-suite-1-1__PDFUA-Ref-2-09_Scanned.pdf` | 82 | 0 | 165 | 0 | 18 | 245 | Has a text layer despite being scanned-style; use for OCR auto/skip-text behavior. |
| `golden__issue-336-conto-economico-bialetti.pdf` | 2 | 0 | 0 | 0 | 51 | 2 | Good partial tables, but glued numeric rows remain. |
| `golden__pdfua-1-reference-suite-1-1__PDFUA-Ref-2-01_Magazine-danish.pdf` | 32 | 0 | 387 | 1 | 12 | 89 | Image/decorative media pressure and document-subtype fixture. |
| `survey-llm.pdf` | 144 | 0 | 236 | 0 | 122 | 64 | Large academic/report stress case. |
| `resnet.pdf` | 12 | 0 | 0 | 0 | 103 | 219 | Heading/table false-positive pressure. |

## Sample Observations

`golden__chinese_scan.pdf` currently renders as:

```md
<!-- page:1 -->
![image](images/page1_img1.png)
```

The financial statement currently contains both useful tables and glued rows:

```md
1. d) Svalutation cred. ... disponibilità liquide745.000904.00006.218.000
```

This means OCR and numeric-table repair should be separate workstreams.

## Environment Probe

Installed on this machine during planning:

- `gs`: present.
- `ocrmypdf`: not found.
- `tesseract`: not found.
- `qpdf`: not found.

The implementation must therefore detect missing OCR dependencies and either skip OCR tests cleanly or emit a clear actionable CLI error when OCR is requested.

