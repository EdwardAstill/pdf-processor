---
title: "Formula Detection and False Positive Suppression Algorithms"
kind: "reference"
category: "wiki"
summary: "Algorithms for detecting display math regions in PDFs and suppressing false positives from decorative rules, table rows, headers/footers, and reference lists."
virtual_path: "wiki/algorithms/formula-detection"
entities: [ScanSSD, DocLayout-YOLO, PDF-Extract-Kit, visual-band-detector, morphological-operations, page-association]
---

# Formula Detection and False Positive Suppression Algorithms

## Detection

### Symbol Density (Tesseract-style)

**What it does**: Counts non-alphanumeric characters in a text region. High density (>30% operators, Greek letters, delimiters) is a formula signal.

**Precision**: ~74% on Google Books corpus (Liu & Smith 2013, ICDAR).

**Fails on**: Table rows with unit symbols (σ, ≥, MPa) — high symbol density but not display math.

### Aspect Ratio + Sliding Window (ScanSSD)

**What it does**: Renders page at 600 DPI. Slides windows at aspect ratios {5:1, 7:1, 10:1} — wide and short, typical of inline or display math. Detects formula character bounding regions using visual features only (no text layer required).

**F-score**: 0.926 on character detection.

**Reference**: arXiv:2003.08005

### Visual Band Scanning (pdfp implementation)

**What it does**: Renders page at low DPI (60–100). Scans horizontal pixel rows for bands of high dark-pixel density. Filters by: minimum band height, isolation gap above and below, proximity to cue words ("where", "=", "defined as"). Returns `FormulaCandidate` with `source: visual-page-render`.

**Implemented in**: `src/formula/visual.rs` (2026-05). Opt-in via `--debug-formulas`.

**Catches**: Visible fraction/display equations the word-based detector misses (e.g. DNV page 670).

**Current false positives**: Horizontal decorative rules (too thin but create a pixel band); logo bars at page top.

### Model-Based Formula Detection (PDF-Extract-Kit YOLOv8_ft)

**What it does**: Dedicated formula detection model (separate from layout detection) trained to distinguish isolated formulas from tables, figures, and text.

**DocLayout-YOLO** has an "Isolated Formula" class trained jointly with "Table" and "Abandoned Text" — joint training reduces formula/table confusion.

---

## False Positive Suppression

### Decorative-Rule Filter (morphological)

**What it does**: Applies `MORPH_OPEN` with a 1×N horizontal kernel to find thin continuous horizontal lines. Lines whose rendered pixel height falls below a threshold relative to body glyph height are decorative (page dividers, logo bars, title underlines) and excluded from formula/table candidates.

**Fixes**: DNV logo-bar false positives in the visual band detector.

**Implementation note**: Also check that the crop contains actual Unicode text spans — a pure-pixel band with no text is never display math.

### Page-Association Furniture Detection

**What it does**: For each Y-band position (top 8%, bottom 8% of page), collects all text spans across consecutive pages and computes text similarity. Spans appearing at the same Y position with >50% identical (or fuzzy-matched) content across ≥3 pages are classified as furniture (running headers, footers, watermarks).

**Fixes**: "Downloaded by…" watermarks contaminating table cells; running section headers triggering formula detector.

**Reference**: https://www.researchgate.net/publication/221253782

### Table-First Ordering

**What it does**: Run table detection before formula detection. Any formula candidate whose bounding box overlaps a detected table region is suppressed (it is a table cell, not display math).

**Fixes**: DNV pages 69 and 389 — specification table rows with Greek symbols generating formula candidates while the table itself goes undetected.

### Reference-List Filter

**What it does**: Contextual text check — if a candidate band contains a leading `[N]` or `(N)` pattern followed by alphabetic runs longer than 20 chars, and contains no operator or Greek characters, classify as bibliography reference.

### Weighted Box Fusion (ensemble)

**What it does**: When multiple detectors each produce candidate boxes, WBF consolidates overlapping predictions by weighting confidence scores. Reduces single-model false positives without discarding true positives that only one model caught.

**Used by**: ICDAR 2021 formula detection winner (arXiv:2107.05534).

**Best for**: Ensemble setups where both word-based and visual-band candidates are merged.

---

## See Also

- [structures/equations.md](../structures/equations.md) — full equations pipeline including OCR sidecars
- [topics/technical-standards-documents.md](../topics/technical-standards-documents.md) — DNV-specific false positive patterns
