---
title: "Reading Order and Layout Algorithms"
kind: "reference"
category: "wiki"
summary: "Algorithms for recovering reading order and page layout from PDF primitives: XY-Cut++, projection-based column detection, deep learning layout detectors, and text line assembly."
virtual_path: "wiki/algorithms/reading-order"
entities: [XY-Cut++, DocLayout-YOLO, DETR, projection-profiling]
---

# Reading Order and Layout Algorithms

## XY-Cut and XY-Cut++

**What it does**: Recursively partitions the page bounding box with horizontal and vertical cuts at whitespace gaps, building a binary tree whose leaves are text regions. The tree is flattened in reading order.

**Core idea**: Find the widest horizontal gap → cut to isolate top/bottom regions. Find the widest vertical gap → cut to isolate columns. Recurse until no more cuts improve the result.

**XY-Cut++** (used in `pdfp`, ported from OpenDataLoader Java) adds four improvements:
1. Pre-mask cross-layout elements (wide headers, titles, footers) before cutting — prevents them acting as false column boundaries
2. Compute density ratio across candidate cuts (reserved for future tiebreaker)
3. Narrow-outlier retry on X axis — if a vertical cut produces very narrow outlier strips, retry with a relaxed threshold
4. Re-merge cross-layout elements back by Y into the final stream

**Coordinate note**: The Java reference uses PDF y-up; `pdfp`'s port flips all Y comparisons because MuPDF uses top-left origin (y increases downward). See `src/layout/xycut.rs` module doc for the full translation table.

**Best for**: Multi-column academic papers, reports, standards.

**Weak on**: Magazines and brochures with non-rectangular text flows; pages with heavy overlap between floats and body text.

**Reference**: arXiv:2504.10258; `src/layout/xycut.rs`

---

## Projection-Based Column Detection

**What it does**: Projects all text fragment x-coordinates onto a 1D histogram. Column boundaries appear as valleys (low-density x ranges). Works as a pre-pass to identify plausible column split points before XY-Cut recurses.

**Best for**: Confirming two-column layouts; detecting irregular column widths.

---

## Deep Learning Layout Detection

**What it does**: Treats layout detection as object detection. A model trained on annotated document images predicts bounding boxes and class labels for document regions (text block, heading, table, figure, formula, footnote, header, footer).

**DocLayout-YOLO** (arXiv:2410.12628): YOLOv10 backbone with Global-to-Local Controllable Receptive Module. 10 output categories, 85.5 FPS, CPU-capable. Categories include Isolated Formula and Table as separate classes — the joint training helps reduce formula/table confusion.

**Docling RT-DETR**: Detection Transformer variant; 10 categories including Formula, Table, Page-header, Page-footer, Picture, Caption.

**Best for**: Routing pages to specialised handlers before extraction. Does not replace text extraction but tells you what kind of content is where.

See [tools/layout-models.md](../tools/layout-models.md) for tool details.

---

## Text Line Assembly

### Baseline clustering

**What it does**: Groups text fragments by their Y-baseline coordinate. Fragments whose baselines differ by less than a threshold (usually ~50% of font height) belong to the same line.

**Subtlety**: Superscripts and subscripts have different baselines than their parent line. Good implementations track the dominant baseline and attach off-baseline fragments to the nearest plausible parent.

### Gap-based fragment merging

**What it does**: Within a candidate line, sorts fragments by X and merges adjacent ones if the gap between them is less than a character-width threshold. Fragments wider than ~2× the average word gap indicate a column boundary rather than a space.

**Best for**: Reconstructing words and sentences from the glyph-level drawing instructions PDFs actually contain.
