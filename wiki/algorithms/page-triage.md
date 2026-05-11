---
title: "Page Triage and Classification Algorithms"
kind: "reference"
category: "wiki"
summary: "Algorithms for classifying pages before extraction runs: scan density, text density, encoding health checks, math density, and document subtype signals."
virtual_path: "wiki/algorithms/page-triage"
entities: [scan-density, text-density, math-density, hybrid-triage]
---

# Page Triage and Classification Algorithms

Triage determines which pipeline path a page or document takes. Running the wrong path wastes time at best and produces garbage at worst. These signals should be computed early, before expensive extraction or model inference.

---

## Scan Density

**What it does**: Computes the ratio of image area to total page area. High image area with low text content suggests a scanned page.

**Threshold (typical)**: Image area > 80% of page AND < 50 extracted characters → likely scan.

**Used in**: `pdfp` hybrid triage (`src/hybrid/triage.rs`).

**Action**: Route to OCR preprocessing (OCRmyPDF) or hybrid/Docling backend.

---

## Text Density

**What it does**: Counts extracted characters per unit page area. Very low density indicates a visual-only page (image, chart, decorative) or a failed extraction (broken encoding, missing ToUnicode map).

**Distinguishes from scan density**: A page with many images and very little text could be a scan (needs OCR) or a figure-heavy page (needs image extraction, not OCR). Cross-referencing image area with character count separates the two.

---

## Encoding Health Check

**What it does**: Checks for high proportion of replacement characters (`\u{FFFD}`), empty Unicode mappings, or non-printable code points in extracted text. Signals that the text layer is unreliable even though characters were extracted.

**Action**: Flag page as low-confidence; optionally route to OCR fallback for the affected spans.

**Used by**: pdf-inspector (explicit encoding classification); recommended for `pdfp`.

---

## Math Density (Hybrid Routing Trigger)

**What it does**: Counts formula candidates relative to page area. Pages above the threshold are routed to Docling for formula enrichment rather than staying on the local path.

**Used in**: `src/hybrid/triage.rs`.

---

## Table Density

**What it does**: Counts detected table candidates (or high-confidence numeric-alignment signals) relative to page area. Pages above threshold are flagged for the table recovery path.

---

## Document Subtype Signals

Page-level signals that imply document class:

| Signal | Likely class |
|--------|-------------|
| Many formula candidates, numbered sections, dense text | Academic paper or standard |
| Short lines, right-aligned numbers, indented rows | Financial statement |
| Repeated logo, large images, variable columns | Brochure or magazine |
| Consistent label+value pairs | Form or invoice |
| Single large image, near-zero text | Figure or scan |

Document subtype should be inferred before applying class-specific recovery passes (financial table engine, form parser, etc.).

---

## See Also

- [topics/scans-and-ocr.md](../topics/scans-and-ocr.md) — full OCR preprocessing strategy
- [topics/pipeline-overview.md](../topics/pipeline-overview.md) — where triage fits in the pipeline
