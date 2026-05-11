---
title: "PDF Engines"
kind: "reference"
category: "wiki"
summary: "Low-level PDF rendering and parsing engines: MuPDF, pdfminer.six, pdfium, lopdf, pypdf, pikepdf."
virtual_path: "wiki/tools/pdf-engines"
entities: [MuPDF, pdfminer.six, pdfium, lopdf, pypdf, pikepdf]
---

# PDF Engines

Low-level libraries that read the PDF file format and expose its content. Everything above this layer is built on top of one of these.

---

## MuPDF (Artifex)

- **Language**: C (bindings: Python, Rust, Java)
- **What it does**: Full PDF renderer and parser. Extracts text spans with coordinates, renders pages to images, manipulates pages (split, merge, crop).
- **Used in**: `pdfp` as the primary extraction engine (`mupdf` Rust crate 0.6)
- **Gotchas**: Not thread-safe. Font names not exposed in Rust wrapper 0.6. `TextPageFlags::PRESERVE_IMAGES` required or image blocks are silently dropped.
- **Licence**: AGPL-3.0 (open source); commercial licence available
- **Links**: https://mupdf.com; https://crates.io/crates/mupdf

---

## pdfminer.six

- **Language**: Python
- **What it does**: Low-level PDF parser. Exposes PDFPage, LTPage, LTTextBox, LTChar, LTLine, LTRect. Foundation for pdfplumber.
- **Best for**: Fine-grained control over extraction; inspecting raw PDF primitives
- **Licence**: MIT
- **Links**: https://github.com/pdfminer/pdfminer.six

---

## pdfium (Google)

- **Language**: C++ (part of Chromium)
- **What it does**: PDF rendering engine. Exposes font names, weights, italic flags, struct-tree roles — things MuPDF 0.6 does not surface.
- **Used in**: `pdfp` optionally via `pdfium-render` Rust crate under `--features pdfium-metadata`
- **Note**: Requires libpdfium installed separately (apt/pacman/brew or pdfium-binaries). When missing, `pdfp` silently falls back to size-only classification.
- **Licence**: BSD 3-clause
- **Links**: https://pdfium.googlesource.com/pdfium; https://github.com/ajrcarey/pdfium-render; https://github.com/bblanchon/pdfium-binaries (pre-built)

---

## lopdf

- **Language**: Rust (pure)
- **What it does**: PDF object graph reader and writer. Good for reading content streams, walking object trees, and simple surgery. Not a renderer.
- **Used by**: firecrawl/pdf-inspector
- **Licence**: MIT
- **Links**: https://crates.io/crates/lopdf

---

## pypdf / PyPDF2

- **Language**: Python (pure)
- **What it does**: Basic text extraction, metadata, page manipulation. Weak at layout. Good for metadata checks and page splits/merges.
- **Licence**: BSD 3-clause
- **Links**: https://github.com/py-pdf/pypdf

---

## pikepdf (QPDF wrapper)

- **Language**: Python (wraps libqpdf C++)
- **What it does**: High-level PDF manipulation — read/write content streams, repair broken PDFs, compress, redact.
- **Licence**: MIT
- **Links**: https://github.com/pikepdf/pikepdf
