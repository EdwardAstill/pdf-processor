---
title: "Page Furniture and Watermark Suppression"
kind: "knowledge"
category: "wiki"
summary: "Approaches to detecting and suppressing repeated page furniture (headers, footers, watermarks, logos, page numbers) during PDF extraction — heuristic methods, content-based analysis, and model-based detection."
entities: [watermark, header, footer, page-furniture, MinerU, refinedoc, DocLayout-YOLO]
---

# Page Furniture and Watermark Suppression

Page furniture — headers, footers, watermarks, logos, page numbers — is one of the most persistent sources of noise in PDF-to-Markdown conversion. Unlike tables and formulas, furniture is perfectly extractable but *wrong* — it contaminates body text, breaks reading order, and creates false positives in formula and table detectors.

---

## What Counts as Furniture

- **Headers/footers**: Repeated text blocks near page top/bottom (chapter titles, section names, dates, page numbers)
- **Watermarks**: Semi-transparent text or images repeated across pages ("DRAFT", "CONFIDENTIAL", "Downloaded by…")
- **Logos**: Repeated company/institutional logos
- **Slide furniture**: Repeated decorative elements in presentation PDFs
- **Decorative rules**: Horizontal/vertical lines, borders

---

## Detection Approaches

### Heuristic: Position + Repetition

The simplest approach and the one most appropriate for `pdfp`:

1. Extract all text blocks with coordinates
2. Group blocks that appear at similar positions across multiple pages
3. Classify groups as furniture if they meet thresholds for repetition (>50% of pages) and position (within 5% of page edge)

**Used by**: MinerU, refinedoc, most practical extraction pipelines

**Implementation sketch**:
```
For each page:
  For each block:
    If block is within 10% of top/bottom edge:
      Hash (text content, relative position)
  For hashes appearing on >50% of pages:
    Mark as furniture → exclude from body text
```

### Content-Based: PDF Operator Analysis

Rather than rasterizing, inspect the PDF content stream directly:

- **Text operators**: Detect text drawn at repeated positions
- **Transparency groups**: Watermarks often have transparency (`/ca` or `/CA` < 1.0)
- **Form XObjects**: Watermarks may be repeated via Form XObjects referenced from each page

**Used by**: oPDF (github.com/Charltsing/oPDF) — analyzes annotation, text, curve, path, form, image, and pattern watermarks directly from PDF primitives

### Image-Based: Rasterize + Morphological Processing

For image/pattern watermarks:

1. Rasterize pages at moderate DPI
2. Apply morphological operations to detect repeated visual patterns
3. Use inpainting to remove watermark regions from the rendered image
4. Re-extract text from the cleaned image

**Used by**: pdf-watermark-remover (PyPI), pdf-watermark-removal-otsu-inpaint (PyPI)

**Limitation**: This is a destructive approach — it loses the digital text layer and requires OCR. Appropriate for image-heavy PDFs, not born-digital ones.

### Model-Based: Layout Detection

Layout detection models (DocLayout-YOLO, YOLO-GFD) can classify page regions including headers, footers, and watermarks. The limitation is training data — most layout models are trained on academic papers, not the full diversity of document types.

Recent work (arxiv 2604.23276) on lightweight PDF visual element parsing explicitly filters watermarks and logos as "non-informative artifacts."

---

## Where Furniture Suppression Fits

Furniture suppression should run **before** semantic classification, **after** text extraction:

```
1. Open PDF → extract text blocks + coordinates + images
2. Furniture detection → flag repeated blocks near page edges
3. Suppress flagged blocks from further processing
4. Run table/formula/heading detection on remaining blocks
5. Render Markdown
```

Running furniture suppression first prevents:
- False formula detections on watermarks containing math symbols
- Table cells contaminated by "Downloaded by…" watermarks
- Heading detection confused by repeated chapter headers

---

## Relevance to pdfp

### Current state
`pdfp` has no furniture suppression. The `--conservative` mode avoids injecting heuristic formula Markdown but doesn't suppress watermarks from output. Known failures:
- "Downloaded by…" text contaminating table cells in standards documents
- Decorative rules triggering formula false positives
- Page headers duplicating in body text

### Recommended approach for pdfp
1. **Position-based heuristic first**: Detect repeated text at page top/bottom edges. Simple, no new dependencies, works for 80% of cases.
2. **Transparency check**: Inspect content stream for `/ca` or `/CA` entries below 1.0 on repeated blocks.
3. **Page-edge zone suppression**: `--suppress-header-height 50 --suppress-footer-height 50` flags for manual tuning.

### Integration with existing features
- Furniture suppression → fewer formula false positives (directly addresses wiki gap #4)
- Fewer false positives → cleaner `--debug-formulas` audit output
- Position-based approach reuses existing block coordinate data from `layout/`

---

## Related Pages

- [Formula Detection and False Positives](../algorithms/formula-detection.md) — furniture triggers formula false positives
- [Table Detection](../algorithms/table-detection.md) — watermarks contaminate table cells
- [Technical Standards Documents](../topics/technical-standards-documents.md) — watermark contamination is a known failure mode
- [Layout and Reading Order](layout-and-reading-order.md) — furniture suppression before reading order recovery
