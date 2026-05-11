---
title: "Algorithms for PDF Content Interpretation"
id: "algorithms-index"
kind: "index"
category: "wiki"
summary: "Index of algorithm pages covering the core techniques used at each stage of PDF-to-Markdown conversion."
virtual_path: "wiki/algorithms/index"
---

# Algorithms for PDF Content Interpretation

Each stage of PDF conversion has a family of known algorithms. Pages in this folder cover the most useful ones — core idea, where they fit, and how they are used in `pdfp`.

## Pages

- [Reading Order and Layout](reading-order.md) — XY-Cut++, projection, deep learning layout detection, text line assembly
- [Table Detection and Structure](table-detection.md) — morphological, whitespace stream, drawing-ops, model-based detection; structure recognition
- [Formula Detection and False Positives](formula-detection.md) — symbol density, ScanSSD, visual band, model-based; suppression heuristics
- [OCR](ocr.md) — Tesseract, PaddleOCR, TrOCR, formula-specific OCR
- [Heading Classification](heading-classification.md) — font-size tiering, bold-at-body-size, struct-tree roles, TOC-driven
- [Page Triage and Classification](page-triage.md) — scan density, text density, encoding health, math density

## Design principle

Do not conflate these stages. Detection (where is a table?), structure recognition (what are its rows and columns?), and rendering (how does it become Markdown?) are separate problems that fail in different ways and require different evaluation fixtures.
