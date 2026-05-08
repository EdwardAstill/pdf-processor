# External Tool Notes

## OpenDataLoader PDF

OpenDataLoader presents a two-path design: default deterministic local conversion for standard digital PDFs, and hybrid backends for complex tables, scans, non-English OCR, formulas, and chart/image descriptions. It also emphasizes bounding boxes, XY-Cut++ reading order, AI-safety filtering, and tagged-PDF support.

Implication for `cnv`: the current deterministic default and Docling hybrid route match the right broad direction, but the implementation should make the pipeline boundary explicit so extraction, page assembly, routing, and rendering are not fused in `main.rs`.

Source: https://github.com/opendataloader-project/opendataloader-pdf

## Docling

Docling advertises advanced PDF understanding, including layout, reading order, table structure, code, formulas, image classification, and a unified `DoclingDocument` representation. Its OCR example configures OCR and table structure as pipeline options before Markdown export.

Implication for `cnv`: a clearer internal document/pipeline layer is the right place to hold page-level routing and later confidence signals. Markdown should remain an export, not the place where uncertain structure is discovered.

Sources:

- https://github.com/docling-project/docling
- https://docling-project.github.io/docling/examples/full_page_ocr/

## Marker

Marker separates table-only conversion and OCR-only conversion from full PDF conversion, and exposes JSON table output with cell bounding boxes. It also recommends force-OCR when text is garbled.

Implication for `cnv`: table and OCR handling should remain separable pipeline concerns. A structural refactor should make future specialized passes easier without moving behavior into the renderer.

Source: https://github.com/datalab-to/marker

## firecrawl/pdf-inspector

`pdf-inspector` is especially relevant because it is Rust and deterministic. It highlights document classification, confidence scores, per-page OCR routing, rectangle-based plus text-alignment table detection, encoding issue detection, and single document load.

Implication for `cnv`: the near-term code shape should preserve a single local extraction pass and expose page assembly/routing as named pipeline steps.

Source: https://github.com/firecrawl/pdf-inspector

## PyMuPDF4LLM

PyMuPDF4LLM exposes Markdown conversion options for OCR, image writing, page chunks, table strategy, header/footer handling, and multi-column order. It represents a MuPDF-backed path where export behavior is parameterized separately from extraction.

Implication for `cnv`: keep output flags and image handling visible at the pipeline boundary rather than scattering output concerns through `main.rs`.

Source: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/api.html

## Recent Paper Evidence

The 2026 PDF-to-RAG evaluation compared Docling, MinerU, Marker, and DeepSeek OCR across 19 configurations. It found that document preparation quality strongly affects downstream QA, and that hierarchy-aware chunking and metadata enrichment can matter more than converter choice alone.

The Docling technical report frames Docling as a document conversion package powered by layout analysis and table-structure models, with an extensible interface.

Implication for `cnv`: investing in a clean pipeline structure is justified because later improvements need hierarchy, metadata, confidence, and table signals before Markdown rendering.

Sources:

- https://arxiv.org/abs/2604.04948
- https://arxiv.org/abs/2408.09869
