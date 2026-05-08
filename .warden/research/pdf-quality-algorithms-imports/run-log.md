# Run Log

## 2026-05-08

- Created research workspace.
- Read existing local research on coordinate tables, formula tools, figure snapshots, and OCR viability.
- Scope: GitHub and external primary-source research for import/adaptation candidates.
- Checked local implementation entry points: `src/layout/table.rs`, `src/formula/detect.rs`, `src/figure/detect.rs`, `src/ocr/mod.rs`, `src/hybrid/client.rs`, `scripts/example-audit.sh`, `docs/TESTING.md`, and `docs/QUALITY_LOOP.md`.
- Researched coordinate table algorithms: pdfplumber, Camelot Stream/Lattice/Network/Hybrid, Tabula, Table Transformer, gmft, and img2table.
- Researched full-document and ML sidecar candidates: Docling, MinerU, Marker, Surya, PDF-Extract-Kit, PaddleOCR, OCRmyPDF, and PyMuPDF4LLM.
- Researched formula-specific options: UniMERNet, PDF-Extract-Kit formula detection/recognition, Surya LaTeX OCR, Marker, and Docling formula enrichment.
- Researched figure extraction approaches: PDFFigures2, DeepFigures, GROBID, Docling, and current local snapshot rendering.
- Recorded license constraints: MIT/Apache sources are candidates for adaptation; GPL/AGPL sources are benchmark or subprocess-only unless the project accepts copyleft obligations.
- Wrote source notes, contradiction register, evidence JSONL, and final report.
