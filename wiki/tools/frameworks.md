---
title: "PDF Conversion Frameworks"
kind: "reference"
category: "wiki"
summary: "Full PDF-to-Markdown/JSON conversion frameworks and modular toolkits: Docling, Marker, MinerU, PyMuPDF4LLM, MarkItDown, OpenDataLoader PDF, pdf-inspector, PDF-Extract-Kit."
virtual_path: "wiki/tools/frameworks"
entities: [Docling, Marker, MinerU, PyMuPDF4LLM, MarkItDown, OpenDataLoader-PDF, pdf-inspector, PDF-Extract-Kit]
---

# PDF Conversion Frameworks

These take a PDF (or other document) as input and return Markdown or structured JSON. Contrast with extraction libraries (lower-level, task-specific) and models (single-task inference).

---

## Docling (IBM)

- **Language**: Python
- **What it does**: Unified `DoclingDocument` IR, RT-DETR layout model, table structure recognition, formula detection, OCR, export adapters (Markdown, JSON, HTML). Supports hybrid mode via docling-serve HTTP API.
- **Used in**: `pdfp` hybrid backend (`--hybrid docling`)
- **Best for**: Hard pages needing model-based layout recovery, formula enrichment, table reconstruction.
- **Licence**: MIT
- **Links**: https://github.com/docling-project/docling; arXiv:2501.17887

---

## Marker (Datalab)

- **Language**: Python (PyTorch)
- **What it does**: Surya OCR + layout model, separate `TableConverter`, optional LLM post-processing. Debug artifacts available.
- **Best for**: Academic papers, research documents; good formula handling.
- **Licence**: CC-BY-NC-SA 4.0 (non-commercial)
- **Links**: https://github.com/datalab-to/marker

---

## MinerU (OpenDataLab)

- **Language**: Python
- **What it does**: VLM + OCR dual engine. 109-language OCR, multi-column support, header/footer removal, cross-page table merging, formula → LaTeX, table → HTML.
- **Best for**: Broad-coverage conversion including scans; formula and table structured output.
- **Licence**: AGPL-3.0
- **Links**: https://github.com/opendatalab/MinerU

---

## PyMuPDF4LLM

- **Language**: Python (PyMuPDF wrapper)
- **What it does**: Converts PDFs to LLM-ready Markdown. Built-in table detection, multi-column support, TOC-driven headers, selective OCR.
- **Best for**: Quick conversion of clean digital PDFs; lighter than Docling/Marker.
- **Licence**: AGPL-3.0
- **Links**: https://github.com/pymupdf/pymupdf4llm

---

## MarkItDown (Microsoft)

- **Language**: Python
- **What it does**: Broad file-format converter (PDF, DOCX, PPTX, images, audio). PDF extraction is lightweight; plugin-based extension for LLM/OCR.
- **Best for**: Broad format support; not PDF-depth quality.
- **Licence**: MIT
- **Links**: https://github.com/microsoft/markitdown

---

## OpenDataLoader PDF

- **Language**: Java core, SDKs for Python/Node.js/Java
- **What it does**: Deterministic local parser with bounding boxes, XY-Cut++ reading order, tagged PDF support, optional hybrid mode for hard pages (OCR, complex tables, formulas, charts). Invisible-text and off-page filtering for prompt-injection safety.
- **Best for**: Architecture reference; tagged PDF handling; explicit per-page routing design.
- **Links**: https://github.com/opendataloader-project/opendataloader-pdf

---

## pdf-inspector (Firecrawl)

- **Language**: Rust
- **What it does**: lopdf + direct content-stream walking. Position-aware extraction, per-page classification, rectangle-based table detection from drawing ops.
- **Best for**: Rust reference implementation; table pipeline and triage design.
- **Links**: https://github.com/firecrawl/pdf-inspector

---

## PDF-Extract-Kit (OpenDataLab)

- **Language**: Python
- **What it does**: Modular model toolkit — not a top-level converter. Separable components:
  - Layout: DocLayout-YOLO, YOLO-v10_ft, LayoutLMv3_ft
  - OCR: PaddleOCR
  - Formula detection: YOLOv8_ft (dedicated model, separate from layout)
  - Formula recognition: UniMERNet
  - Table recognition: PaddleOCR+TableMaster, StructEqTable, StructTable-InternVL2-1B
- **Best for**: Wiring individual model components; understanding the state of the art for each subproblem.
- **Licence**: AGPL-3.0
- **Links**: https://github.com/opendatalab/PDF-Extract-Kit; https://pdf-extract-kit.readthedocs.io
