---
title: "Heading Classification Algorithms"
kind: "reference"
category: "wiki"
summary: "Algorithms for detecting and classifying headings in PDFs: font-size tiering, bold-at-body-size detection, struct-tree role extraction, and TOC-driven classification."
virtual_path: "wiki/algorithms/heading-classification"
entities: [font-size-tiering, struct-tree, TOC, pdfium-metadata]
---

# Heading Classification Algorithms

## Font-Size Tiering

**What it does**: Computes the modal (body) font size across the page. Classifies blocks by ratio to body size.

**Thresholds used in `pdfp`** (`src/layout/classifier.rs`):

| Ratio to body | Level |
|---------------|-------|
| ≥ 2.0 | H1 |
| ≥ 1.6 | H2 |
| ≥ 1.35 | H3 |
| ≥ 1.15 | H4 |
| < 1.15 | H5 (or body) |

**Fails on**: Documents where headings use the same size as body text but differ in weight, colour, or style — e.g. many engineering standards and corporate documents.

---

## Bold at Body Size (`pdfium-metadata` feature)

**What it does**: If font weight ≥ 700 and the line is short and not sentence-terminated, classify as H4 even at body size. Rescues documents with minimal or absent size hierarchy.

**Used in**: `pdfp` when built with `--features pdfium-metadata` (requires libpdfium at runtime).

**Why it is opt-in**: mupdf 0.6's Rust wrapper does not expose font weight. pdfium-render is a second-opinion reader that adds this signal. The feature flag dynamically loads libpdfium; if missing, the classifier silently falls back to size-only.

---

## Struct-Tree Role Override

**What it does**: Tagged PDFs carry a `/StructTreeRoot` with role attributes (H1–H6, Title, P, Table, etc.). These are authoritative and override all size-ratio or weight heuristics.

**Priority**: Struct-tree roles are the highest-confidence signal. Any role assignment here is exact.

**Best for**: Any properly tagged PDF. Common in government, standards, and accessibility-compliant documents.

**Implementation note**: Struct-tree support requires reading the PDF's logical structure tree, which is separate from the content stream. The `pdfium-metadata` feature surfaces this via pdfium-render.

---

## TOC-Driven Headers

**What it does**: Reads the PDF's outline/bookmark tree (`/Outlines`) to get known heading texts and page numbers. Matches extracted text to outline entries to assign heading levels.

**Used by**: PyMuPDF4LLM.

**Best for**: Documents with a well-maintained table of contents.

**Fails on**: PDFs with corrupt or missing outlines; documents where bookmark text does not exactly match page text.

---

## Priority Order

When multiple signals are available:

1. Struct-tree role (authoritative)
2. TOC match (high confidence)
3. Bold at body size (medium confidence, feature-gated)
4. Font-size ratio (default fallback)
