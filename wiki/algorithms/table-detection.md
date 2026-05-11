---
title: "Table Detection and Structure Recognition Algorithms"
kind: "reference"
category: "wiki"
summary: "Algorithms for locating table regions and recovering row, column, and cell structure: morphological line extraction, whitespace column inference, drawing-ops extraction, model-based detection, and structure recognition methods."
virtual_path: "wiki/algorithms/table-detection"
entities: [Camelot, pdfplumber, TATR, LGPMA, StructEqTable, CascadeTabNet, morphological-operations, projection-profiling]
---

# Table Detection and Structure Recognition Algorithms

Table work splits into two subproblems: **detection** (finding the table region) and **structure recognition** (recovering rows, columns, and cells). They need separate algorithms and separate evaluation.

---

## Detection

### Morphological Line Extraction (Lattice)

**What it does**: Converts the page to a binary image. Applies `MORPH_OPEN` with a horizontal structuring element (e.g. 1×40 pixels) to extract horizontal rules, and with a vertical element (40×1) to extract vertical lines. Intersections of H and V lines define the cell grid.

**Used by**: Camelot lattice mode, pdfplumber `lines` strategy, img2table, OpenCV tutorials.

**Best for**: Tables with explicit border lines (invoices, many financial tables, fully-gridded standards tables).

**Fails on**: Borderless tables; tables with only a header rule (common in engineering standards).

### Whitespace Column Inference (Stream)

**What it does**: For each candidate row, sorts text fragments by X. Infers column boundaries at large horizontal gaps. Rows are identified by Y-position clustering.

**Used by**: Camelot stream mode, pdfplumber text strategy, Tabula.

**Best for**: Tables with consistent column spacing and no merged cells.

**Fails on**: Wide descriptive first columns (common in standards), merged cells, multi-line cells, symbol-heavy rows.

### Drawing-Ops Explicit Line Extraction

**What it does**: Reads PDF content stream drawing operators (`l`, `m`, `re`, etc.) directly rather than rasterising the page. Gets sub-pixel precise line coordinates without DPI-dependent rendering artefacts.

**Used by**: pdfplumber `edges` API, pdf-inspector.

**Best for**: Standards documents where tables have precise hairline rules that do not rasterise cleanly at low DPI. More reliable than morphological detection when lines are present.

### Model-Based Table Detection

**What it does**: Predicts table bounding boxes from page images.

**Table Transformer (TATR)**: DETR + ResNet-18, trained on PubTables-1M (academic papers). GPU required.

**DocLayout-YOLO Table class**: Part of the 10-category layout model. CPU-capable. Trained on general documents; may not transfer well to engineering standards.

**Limitation**: Pre-trained models are biased toward their training domain. Fine-tuning required for good recall on DNV/ISO standards. See [structures/tables.md](../structures/tables.md) for standards-specific issues.

---

## Structure Recognition

Once a table region is known, structure recognition recovers rows, columns, and cells.

### Projection Profiling

**What it does**: Within the table region, projects text fragment Y coordinates to find row boundaries (horizontal gaps) and X coordinates to find column boundaries.

**Best for**: Clean rectangular tables. Fast and deterministic.

### Ruled-Line Cell Intersection

**What it does**: From detected H and V line segments, finds intersections and uses them as cell corners. Walks the intersection grid to assign cell spans.

**Best for**: Fully bordered tables.

### LGPMA (Local + Global Pyramid Mask Alignment)

**What it does**: Mask R-CNN variant with two feature paths — local (text region masks) and global (cell relationship masks) — combined with pyramid re-scoring. Explicitly handles merged/spanning cells.

**Best for**: Complex tables with merged headers and irregular spans.

**Reference**: arXiv:2105.06224 (ICDAR2021 Best Industry Paper)

### StructEqTable

**What it does**: Converts a table crop image directly to LaTeX or HTML using a model trained on arXiv documents (DocGenome/TableX, 2M+ image-LaTeX pairs). Handles symbol-heavy cells better than general table models.

**CPU feasible**: No (GPU recommended).

**Domain note**: Trained on arXiv papers. Performance on engineering standards (DNV/ISO) is unknown.

**Reference**: arXiv:2406.11633; github.com/InternScience/StructEqTable-Deploy

---

## Failure Mode: Formula/Table Confusion

When table rows contain mathematical symbols (σ ≥ 235 N/mm²), the formula detector and table detector compete. The correct ordering is: **run table detection first**, then exclude table regions from formula candidacy. See [formula-detection.md](formula-detection.md) for the suppression strategy.
