# PDF-to-Markdown Wiki

Working knowledge base for building and improving `pdfp`. Covers PDF internals, algorithms, tools, content structures, pipeline design, and the external project landscape.

Scoped to local, code-first PDF-to-Markdown conversion. Not a general PDF encyclopedia.

## Sections

### [algorithms/](algorithms/algorithms.md) — How it works

Core techniques used at each pipeline stage.

- [Reading Order and Layout](algorithms/reading-order.md)
- [Table Detection and Structure](algorithms/table-detection.md)
- [Formula Detection and False Positives](algorithms/formula-detection.md)
- [OCR](algorithms/ocr.md)
- [Heading Classification](algorithms/heading-classification.md)
- [Page Triage and Classification](algorithms/page-triage.md)

### [tools/](tools/tools.md) — What to use

Libraries, frameworks, models, and Rust crates.

- [PDF Engines](tools/pdf-engines.md) — MuPDF, pdfminer.six, pdfium, lopdf
- [Extraction Libraries](tools/extraction-libs.md) — pdfplumber, Camelot, tabula, img2table, OCR engines
- [Frameworks](tools/frameworks.md) — Docling, Marker, MinerU, PDF-Extract-Kit, OpenDataLoader
- [Layout Models](tools/layout-models.md) — DocLayout-YOLO, TATR, Surya
- [Equation OCR Models](tools/equation-ocr-models.md) — UniMERNet, RapidLaTeX-OCR, Texo, Pix2Tex
- [Table Models](tools/table-models.md) — StructEqTable, CascadeTabNet, LGPMA
- [Rust Crates](tools/rust-crates.md) — mupdf, ort, lopdf, quick-xml, resvg

### [structures/](structures/) — Content types

How each structural element is encoded in PDFs and rendered in Markdown.

- [Tables](structures/tables.md) — generic, financial, forms
- [Equations](structures/equations.md) — detection, false positive suppression, OCR sidecars
- [Headings](structures/headings.md) — font-size tiering, bold detection, struct-tree, TOC
- [Lists](structures/lists.md) — ordered, unordered, nested, bullet normalisation
- [Inline Formatting](structures/inline-formatting.md) — bold, italic, code spans, underline

### [topics/](topics/) — Pipeline concepts

Cross-cutting concerns and pipeline design.

- [Pipeline Overview](topics/pipeline-overview.md)
- [Text Extraction](topics/text-extraction.md)
- [Layout and Reading Order](topics/layout-and-reading-order.md)
- [Scans and OCR](topics/scans-and-ocr.md)
- [Markdown Rendering](topics/markdown-rendering.md)
- [Figure Snapshot Extraction](topics/figure-snapshot-extraction.md)
- [Information Extraction](topics/information-extraction.md)
- [Evaluation and Benchmarks](topics/evaluation-and-benchmarks.md)
- [Technical Standards Documents](topics/technical-standards-documents.md)

### [projects/](projects/) — External landscape

What other projects are doing and what to borrow.

- [Project Comparison Matrix](projects/project-comparison-matrix.md)
- [OpenDataLoader Ecosystem](projects/opendataloader-ecosystem.md)
- [Reference Implementations](projects/reference-implementations.md)
- [Improvement Opportunities](projects/improvement-opportunities.md)

---

## Current biggest gaps (2026-06)

1. Table detection for engineering standards — borderless tables, symbol-heavy rows; form-column clustering now handles borderless form tables but complex multi-row-header standards tables still need improvement
2. Formula LaTeX recovery quality — sidecar path exists (`rapid_latex_ocr`) but still produces garbled output on some symbols; needs better crop selection and post-processing
3. Watermark/footer suppression — "Downloaded by…" text contaminating table cells
4. Inline formatting — bold/italic only extracted with optional `pdfium-metadata` feature
5. Financial statement reconstruction — row structure loss in complex accounting tables

## Design position

`pdfp` is intentionally:

- PDF-first and local-first
- deterministic extraction before model-based recovery
- Markdown-first output
- code-first (no silent hosted-API dependencies)

## Related repo docs

- [README.md](../README.md) — install, CLI usage, architecture
- [docs/pdf-internals.md](../docs/pdf-internals.md) — PDF object model, content streams, fonts
- [docs/TESTING.md](../docs/TESTING.md) — verification matrix
