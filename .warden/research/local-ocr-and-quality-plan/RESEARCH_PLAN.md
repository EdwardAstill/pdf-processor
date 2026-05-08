# Local OCR and PDF Quality Plan Research Plan

Date checked: 2026-05-05

## Question

Which next changes are viable for `cnv` after the pipeline refactor, especially local OCR, table quality, tagged PDF support, and reducing overgrown renderer logic?

## Deliverables

- A research-backed implementation plan.
- A baseline conversion run against PDFs already present in the repo.
- A viability report with external and local evidence.

## Sub-questions

- Should OCR be implemented as a local preprocessing sidecar or as a new converter path?
- Which OCR tool shape is viable from Rust without overcomplicating the app?
- Which current examples expose scan/table/layout weaknesses?
- Which changes should be sequenced first so every later improvement is measurable?
- Which existing code areas look more complicated than they need to be?

## Source Families

- Official/project docs for OCRmyPDF, Tesseract, Docling, PyMuPDF4LLM, and pdf-inspector.
- Repo-local wiki pages on scans/OCR, improvement opportunities, benchmarks, and pipeline shape.
- Current baseline run over `example/pdf`.

