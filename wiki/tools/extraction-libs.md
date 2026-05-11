---
title: "Extraction Libraries and OCR Engines"
kind: "reference"
category: "wiki"
summary: "High-level Python extraction libraries for text and table extraction (PyMuPDF, pdfplumber, Camelot, tabula, img2table) and OCR engines (Tesseract, PaddleOCR, EasyOCR, OCRmyPDF)."
virtual_path: "wiki/tools/extraction-libs"
entities: [PyMuPDF, pdfplumber, Camelot, tabula-py, img2table, Tesseract, PaddleOCR, EasyOCR, OCRmyPDF]
---

# Extraction Libraries and OCR Engines

---

## PDF Extraction Libraries

### PyMuPDF (fitz)

- **Language**: Python wrapper for MuPDF
- **What it does**: `page.get_text("dict")` returns spans with coordinates, fonts, colours. `page.find_tables()` for simple bordered tables. Can render pages to PIL/numpy arrays.
- **Licence**: AGPL-3.0
- **Links**: https://github.com/pymupdf/PyMuPDF

### pdfplumber

- **Language**: Python (built on pdfminer.six)
- **What it does**: Exposes text, words, chars, lines, rects per page. Fine-grained table extraction with configurable snap tolerances and edge strategies (`lines`, `text`, `explicit`). Visual debugging to PNG.
- **Best for**: Understanding why a table is failing; tuning with known line geometry. Best debuggability in the Python ecosystem.
- **For engineering standards**: Use `lines` or `explicit` strategy with drawing-ops edges; adjust `snap_x_tolerance` and `snap_y_tolerance` for hairline rules.
- **Licence**: MIT
- **Links**: https://github.com/jsvine/pdfplumber

### Camelot

- **Language**: Python (requires Ghostscript for lattice mode)
- **What it does**: Two modes — Lattice (morphological line detection via OpenCV) and Stream (whitespace column inference). Returns tables as pandas DataFrames.
- **Best for**: Lattice mode on fully-bordered tables; Stream on spaced-column tables.
- **Fails on**: Borderless standards tables, symbol-heavy cells, merged cells.
- **Licence**: MIT
- **Links**: https://github.com/camelot-dev/camelot; https://camelot-py.readthedocs.io

### tabula-py

- **Language**: Python wrapper for tabula-java (JVM)
- **What it does**: Lattice and Stream modes. Stream better on some borderless cases but output parsing weaker. Requires Java runtime.
- **Fails on**: Merged cells, multi-page tables, multiple tables per page.
- **Licence**: MIT
- **Links**: https://github.com/chezou/tabula-py

### img2table

- **Language**: Python (OpenCV, no ML)
- **What it does**: Morphological erosion/dilation and contour detection. `borderless_tables` parameter for heuristic borderless detection (alpha quality; requires ≥3 columns). Handles merged cells and multi-page tables.
- **Best for**: CPU-only borderless table detection; worth evaluating on DNV-style specification tables.
- **Licence**: MIT
- **Links**: https://github.com/xavctn/img2table

---

## OCR Engines

### Tesseract

- **Language**: C++ (Python: pytesseract; Rust: leptess)
- **What it does**: Classic open-source OCR. LSTM mode for modern accuracy. Separate equation-detector module uses symbol density to locate formula regions.
- **Best for**: Scan-heavy documents; broad language support. Used by OCRmyPDF as recognition backend.
- **Licence**: Apache 2.0
- **Links**: https://github.com/tesseract-ocr/tesseract

### PaddleOCR (Baidu)

- **Language**: Python (PaddlePaddle)
- **What it does**: Detection + recognition neural OCR stack. 80+ languages. Default engine in PDF-Extract-Kit and MinerU.
- **Best for**: High-accuracy production OCR.
- **Licence**: Apache 2.0
- **Links**: https://github.com/PaddlePaddle/PaddleOCR

### EasyOCR

- **Language**: Python (PyTorch)
- **What it does**: Simple API, 80+ languages. Less accurate than PaddleOCR on complex layouts but easier to set up.
- **Licence**: Apache 2.0
- **Links**: https://github.com/JaidedAI/EasyOCR

### OCRmyPDF

- **Language**: Python (wraps Tesseract + Ghostscript/MuPDF)
- **What it does**: Adds a searchable text layer to a scanned PDF, producing a standard PDF. The original file is untouched; the output feeds back into the normal extraction pipeline.
- **Best for**: Preprocessing scan-heavy PDFs before `pdfp` converts them. The cleanest OCR integration pattern.
- **Licence**: MPL-2.0
- **Links**: https://github.com/ocrmypdf/OCRmyPDF

### pdf2image

- **Language**: Python (wraps pdftoppm via poppler)
- **What it does**: Converts PDF pages to PIL Image objects at configurable DPI.
- **Best for**: Getting page images as input for model-based detectors.
- **Licence**: MIT
- **Links**: https://github.com/Belval/pdf2image
