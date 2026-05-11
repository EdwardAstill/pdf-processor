---
title: "Rust Crates for PDF Processing"
kind: "reference"
category: "wiki"
summary: "Rust crates used in pdfp and useful for PDF processing, ML inference, XML parsing, HTML scraping, and SVG rendering."
virtual_path: "wiki/tools/rust-crates"
entities: [mupdf, pdfium-render, lopdf, ort, quick-xml, scraper, resvg, zip]
---

# Rust Crates for PDF Processing

---

## mupdf (0.6.0)

- **What it does**: Rust bindings for MuPDF. Used in `pdfp` for all PDF extraction and rendering.
- **Gotchas in `pdfp`**:
  - Font names/weights not exposed — use `pdfium-render` feature for those
  - Not thread-safe — never use `par_iter`/rayon for PDF processing
  - `TextPageFlags::PRESERVE_IMAGES` required or image blocks are silently dropped
  - `Rect` is `(x0, y0, x1, y1)` corner coords, not `(x, y, w, h)`
- **Links**: https://crates.io/crates/mupdf

---

## pdfium-render

- **What it does**: Rust bindings for libpdfium. Used in `pdfp` under `--features pdfium-metadata` to add font family, weight, italic flags, and struct-tree roles.
- **Runtime requirement**: libpdfium shared library must be installed separately. When missing, `pdfp` logs a warning and falls back to size-only classification.
- **Links**: https://github.com/ajrcarey/pdfium-render

---

## lopdf

- **What it does**: Pure-Rust PDF object graph reader and writer. No rendering. Good for content stream inspection and low-level manipulation.
- **Links**: https://crates.io/crates/lopdf

---

## ort

- **What it does**: Rust wrapper for ONNX Runtime. Supports CPU and CUDA backends via feature flags. The standard path for running ONNX-exported ML models (formula OCR, layout detection) natively in Rust without Python.
- **Usage**: Load an ONNX model file → create a session → run inference with ndarray inputs → get ndarray outputs.
- **Licence**: MIT / Apache 2.0
- **Links**: https://github.com/pykeio/ort; https://crates.io/crates/ort

---

## quick-xml (0.39)

- **What it does**: Fast Rust XML parser (streaming SAX-style). Used in `pdfp` for DOCX, EPUB, and PPTX extraction.
- **API notes (0.39)**:
  - Use `BytesText::decode()` not `unescape()`
  - Use `Attribute::unescape_value()`
- **Links**: https://crates.io/crates/quick-xml

---

## scraper

- **What it does**: HTML5-compliant selector-based scraper built on html5ever. Used in `pdfp` for HTML and EPUB chapter content extraction.
- **Links**: https://crates.io/crates/scraper

---

## resvg

- **What it does**: Pure-Rust SVG renderer. Used in `pdfp` for SVG → PNG conversion (Pipeline C).
- **Links**: https://crates.io/crates/resvg

---

## zip

- **What it does**: Rust ZIP archive reader/writer. Used in `pdfp` for reading DOCX, EPUB, and PPTX container files (all are ZIP archives).
- **Links**: https://crates.io/crates/zip

---

## oar-ocr

- **What it does**: OCR utilities built on top of `ort`. Supports formula model inference. Useful for wiring up ONNX equation OCR models in a Rust pipeline without writing raw ONNX session code.
- **Links**: https://crates.io/crates/oar-ocr; https://github.com/GreatV/oar-ocr
