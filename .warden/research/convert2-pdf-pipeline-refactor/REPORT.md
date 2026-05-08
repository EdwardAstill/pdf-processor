# Convert2 PDF Pipeline Refactor Report

Date checked: 2026-05-05

## Executive Conclusion

The strongest shared finding from current PDF-to-Markdown tools and the repo wiki is pipeline separation: deterministic local extraction should remain the default, hard pages should route through explicit hybrid/OCR/table paths, and Markdown rendering should format already-known structure instead of discovering structure late.

For this turn, the right refactor is therefore not a new feature. It is to move PDF processing out of `main.rs` into a dedicated pipeline module while preserving behavior. That gives future work a clear home for tagged-PDF signals, OCR preprocessing, confidence reports, table-specific passes, and page routing.

## Evidence Summary

- OpenDataLoader PDF separates standard digital PDFs from hybrid modes for complex tables, scans, non-English OCR, formulas, and chart/image descriptions. Source: https://github.com/opendataloader-project/opendataloader-pdf
- Docling exposes PDF layout, reading order, table structure, formulas, image classification, and a unified document representation, with OCR and table structure configured as pipeline options before Markdown export. Sources: https://github.com/docling-project/docling and https://docling-project.github.io/docling/examples/full_page_ocr/
- Marker separates table-only and OCR-only converters from full PDF conversion, which supports keeping table/OCR paths modular. Source: https://github.com/datalab-to/marker
- `pdf-inspector` is a Rust reference point for deterministic classification, confidence, OCR routing, geometry/text table detection, encoding checks, and single document load. Source: https://github.com/firecrawl/pdf-inspector
- PyMuPDF4LLM keeps Markdown export parameterized with OCR, image, table, page, and header/footer options. Source: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/api.html
- A 2026 PDF-to-RAG study found document preparation quality is a dominant factor for downstream QA, and hierarchy/metadata choices matter materially. Source: https://arxiv.org/abs/2604.04948
- The local wiki says rendering should remain small and deterministic; extraction, layout recovery, table reconstruction, and scan handling belong earlier in the pipeline. Source: `wiki/markdown-rendering.md`
- The local wiki prioritizes tagged-PDF support, OCR preprocessing, numeric-heavy tables, geometry-aware tables, cross-page table continuity, and debug artifacts. Source: `wiki/improvement-opportunities.md`
- Warden's refactoring wiki calls this a structural refactor and requires impact mapping plus fresh verification. Source: `/home/eastill/projects/warden/wiki/knowledge/refactoring-code-intelligence.md`

## Refactor Recommendation

Extract a `pipeline` module from `main.rs` with ownership of:

- resolving one PDF into a `Document`
- assigning XY-Cut reading order
- classifying text blocks with PDF metadata
- saving and merging image blocks
- warning on empty/scan-heavy pages
- applying hybrid routing
- rendering and writing the document

Keep `main.rs` responsible for:

- module declarations
- CLI parsing
- batch input resolution
- per-file success/error reporting
- process exit behavior

## Behavior Contract

The refactor must not change:

- CLI flags or defaults
- output directory and file naming
- image paths
- scan-heavy warnings
- hybrid routing and fallback behavior
- Markdown rendering output
- current test expectations

## Verification Plan

- Baseline: `cargo test`
- After refactor: `cargo fmt --check`
- After refactor: `cargo test`

## Follow-up Work Not Included

- Tagged-PDF structure support.
- Local OCR preprocessing.
- Table-specific pipeline pass.
- Geometry/debug artifact mode.
- Independent benchmarking of external project accuracy claims.
