# Progress

## Status
In Progress

## Tasks
- [x] Research: Formula OCR — benchmarks research complete (2026-05-27)
- [x] Research: PDF pipelines — formula handling architecture complete (2026-05-27)
- [x] Research: Formula OCR — tool survey complete (2026-05-27)

## Files Changed
- `.pied/research/formula-latex-ocr/raw/benchmarks.md` — raw source data from 9 sources
- `.pied/research/formula-latex-ocr/notes/benchmarks.md` — analyzed findings with comparison table
- `.pied/research/formula-latex-ocr/raw/pdf-pipelines.md` — raw source data (Docling, Marker, Nougat, pix2tex, MinerU, PaperStructure)
- `.pied/research/formula-latex-ocr/notes/pdf-pipelines.md` — analyzed pipeline architecture findings
- `.pied/research/formula-latex-ocr/raw/formula-ocr-tools.md` — raw source data for 8+ formula OCR tools
- `.pied/research/formula-latex-ocr/notes/formula-ocr-tools.md` — analyzed findings, comparison table, sidecar assessment

## Key Finding: Top Sidecar Tools
For the use case (receive crop PNG → LaTeX to stdout):
1. **RapidLaTeXOCR** (MIT) — best fit: ONNX, clean stdout CLI, fast
2. **pix2tex** (MIT) — most popular: simple CLI, heavier (PyTorch)
3. **TexTeller** (Apache 2.0) — best accuracy/generalization: 80M training pairs, strong CLI
