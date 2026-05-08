# Contradictions and Gaps

## True Contradictions

None found. External sources and the local wiki agree that OCR should be selective and should not replace clean native text extraction.

## Different Use Cases

- OCRmyPDF is best for creating searchable derivative PDFs from scanned/image PDFs.
- Tesseract is the underlying OCR engine and can produce searchable PDF/hOCR directly from images, but it does not provide OCRmyPDF's PDF-level workflow.
- Docling is better for heavier document understanding, especially tables/formulas/layout, but it is slower and operationally heavier.
- pdf-inspector demonstrates a Rust-first classification/routing model but is a separate parser, not an OCR implementation.

## Gaps

- OCR tools are not installed on this machine, so the plan cannot include a real OCR-vs-baseline comparison yet.
- The current quality script recurses through `example/pdf`, which includes duplicate nested PDFs under `papers/`; the plan should add a non-duplicate/top-level mode.
- The current baseline metrics are proxies. They prove conversion stability and surface signals, but they are not semantic quality scores.

