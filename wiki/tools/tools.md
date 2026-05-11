---
title: "PDF Tooling Landscape"
id: "tools-index"
kind: "index"
category: "wiki"
summary: "Index of tool pages covering libraries, frameworks, models, and Rust crates for PDF parsing, layout analysis, table extraction, formula OCR, and general OCR."
virtual_path: "wiki/tools/index"
---

# PDF Tooling Landscape

Organised by function. For algorithm details see [Algorithms](../algorithms/algorithms.md). For project-level comparisons see [projects/](../projects/project-comparison-matrix.md).

## Pages

- [PDF Engines](pdf-engines.md) — low-level renderers and parsers: MuPDF, pdfminer.six, pdfium, lopdf, pypdf, pikepdf
- [Extraction Libraries](extraction-libs.md) — high-level Python extraction: PyMuPDF, pdfplumber, Camelot, tabula, img2table; OCR engines
- [Frameworks](frameworks.md) — full conversion pipelines: Docling, Marker, MinerU, PyMuPDF4LLM, OpenDataLoader, pdf-inspector, PDF-Extract-Kit
- [Layout Models](layout-models.md) — DocLayout-YOLO, Table Transformer, Surya, LayoutLMv3
- [Equation OCR Models](equation-ocr-models.md) — UniMERNet, RapidLaTeX-OCR, Texo, Pix2Tex; comparison table; Rust integration via `ort`
- [Table Models](table-models.md) — StructEqTable, CascadeTabNet, LGPMA, TableStructureFormer
- [Rust Crates](rust-crates.md) — mupdf, pdfium-render, lopdf, ort, quick-xml, scraper, resvg, zip
