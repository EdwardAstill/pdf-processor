---
title: "Technical Standards Documents (DNV, ISO, IEC, ASTM)"
kind: "reference"
category: "wiki"
summary: "Engineering and maritime standards have distinct PDF characteristics that break generic PDF-to-Markdown pipelines: symbol-heavy tables, decorative rule false positives, watermark contamination, and near-zero table detection rates. This page maps those failure modes and their fixes."
virtual_path: "wiki/topics/technical-standards-documents"
entities: [DNV, ISO, IEC, ASTM, camelot, pdfplumber, img2table, DocLayout-YOLO, StructEqTable]
---

# Technical Standards Documents (DNV, ISO, IEC, ASTM)

Engineering and maritime standards are a distinct document class. They are not
financial reports, academic papers, or scanned books. Their failure modes are
different enough to warrant dedicated treatment.

## Characteristics of Standards PDFs

### Visual structure

- Title page with organisation logo, standard number, large horizontal title bars
- Dense sections separated by horizontal rules (thin lines 1–3px high)
- Numbered clauses with deeply nested heading hierarchy (1.2.3.4)
- Mixed text and formulae inline (variable definitions, limit checks)
- Display equations for design formulae (often boxed or indented)
- Specification tables with units, Greek letter variables, limit values
- Multi-page tables that continue across page breaks

### Text layer

- Well-formed digital text (not scanned), but symbol encodings vary
- Greek letters (σ, ε, φ, α) embedded as Unicode or as Symbol font glyphs
- Superscripts and subscripts (e.g. `f_y`, `N_Ed`) as separate text spans
- Right-aligned numeric columns in specification tables

### Access control artifacts

Many standards carry redistribution watermarks embedded as text layers:
- "Downloaded by [organisation] on [date]"
- "No further distribution allowed"
- "CONFIDENTIAL — FOR AUTHORISED USE ONLY"

These appear at consistent positions (typically footer or header) and pollute
extracted text, table cell content, and Markdown output.

## Known Failure Modes in pdfp

Based on DNV-ST-N001 (699 pages, assessed 2026-05):

### Formula detection — false positives

| Page type | Symptom | Root cause |
|-----------|---------|------------|
| Page 1 (title / logo) | Logo/header bars cropped as formulas | Visual band detector sees long horizontal dark region |
| Any section header | Horizontal rule under heading flagged | Rule is 1–3px high but creates pixel band |
| Table rows with σ, ≥, MPa | Row classified as formula candidate | High non-alphanumeric symbol density without table context |

See `formula-detection-and-ocr.md` for suppression strategies.

### Table detection — near-zero recall

DNV result: only **10 table candidates** across 699 pages. Known problem pages:

- **Page 69**: 0 table candidates, 12 formula candidates (specification table
  with mixed Greek/numeric columns)
- **Page 389**: 0 table candidates, 5 formula candidates

Root causes:

1. **No border lines**: Many DNV tables use only horizontal rules (top and
   bottom of table, column headers), not a full grid. Lattice-mode detectors
   that require a closed grid fail completely.
2. **Symbol-heavy cells**: Rows like `σ_y ≥ 235 N/mm²` look like display math
   rather than table cells to both word-based and visual detectors.
3. **Wide first column**: A long text description in column 1 followed by
   several short numeric columns confuses whitespace-based stream detectors.
4. **Formula detection absorbs table pages**: Because table rows trigger the
   formula detector, the table detector sees a depleted candidate pool.

### Watermark contamination

Detected in: several DNV tables contained footer text including "Downloaded",
"No further distribution", "shall". These appear because:

- Watermark text is extracted as regular text spans at consistent Y positions
- Table cell extraction sweeps the full page area and picks them up
- They appear in the rendered Markdown table as spurious rows or cells

## Practical Approach for Standards

### Watermark/footer suppression

Pre-pass before any layout analysis:

1. Collect all text spans in the bottom 8% of each page
2. Hash normalised span content per page; spans that are identical (or fuzzy-
   match) on ≥50% of pages are furniture
3. Add to per-document furniture mask; suppress those bboxes from all
   subsequent extraction

This is the page-association method (see `formula-detection-and-ocr.md`
§ Running headers, footers, and watermarks). No model required.

### Table detection strategy for borderless tables

Prioritise these approaches in order:

1. **Explicit line detection** — extract drawing operators (lines and rects)
   from the PDF content stream. Even tables with no vertical grid lines usually
   have at least one horizontal rule at the header boundary. `pdfplumber`'s
   `"lines"` strategy and explicit edge specification work well here.

2. **Whitespace column inference** — `Camelot stream` mode and `pdfplumber`
   text-based strategies can infer column boundaries from the statistical
   distribution of x-coordinates across a suspected table region, but require
   the table region to be known first.

3. **img2table (OpenCV, CPU)** — morphological erosion/dilation can detect
   implicit table structure even without grid lines. Its `borderless_tables`
   mode is alpha-quality but worth evaluating on DNV-style specification tables.
   Requires ≥3 columns. License: MIT.

4. **DocLayout-YOLO** — YOLOv10-based layout detector trained on DocSynth-300K,
   85.5 FPS, CPU-capable, AGPL-3.0. Detects 10 categories including "Table",
   "Table Caption", "Isolated Formula". Domain match is uncertain (trained on
   general docs, not standards), but separating Table and Isolated Formula at
   the layout stage is the right architectural approach.

5. **StructEqTable** — converts a table crop (image) to LaTeX/HTML/Markdown.
   Handles symbol-heavy cells because it was trained on arXiv documents with
   math. GPU required (~1 sec/image on A100). License: MIT. Use as a sidecar
   for difficult cells once the table region is detected.

### No pre-trained model covers engineering standards

PubTables-1M (TATR, microsoft/table-transformer) is academic papers.
DocGenome (StructEqTable) is arXiv. Neither covers ISO/DNV/IEC. Custom fine-
tuning would be required to get high accuracy on standards-style tables. Until
that data exists, geometry-first approaches with explicit line detection are
more reliable than model-first approaches.

### Order of operations for standards pages

```
Page rendering at medium DPI
  ↓
Furniture mask (watermarks, running headers/footers)
  ↓
Drawing ops extraction (lines, rects from content stream)
  ↓
Table region detection (explicit lines first, whitespace second)
  ↓
Formula detection on non-table regions only
  ↓
Formula false-positive suppression (decorative rules, reference lists)
  ↓
Table cell extraction with watermark bbox exclusion
  ↓
Formula OCR sidecar for flagged crops (optional)
```

The critical gate is running table detection before formula detection, so that
table rows with symbols are excluded from the formula candidate pool.

## Tool Summary for Standards

| Tool | Approach | Standards fit | CPU | License |
|------|---------|--------------|-----|---------|
| pdfplumber | Text + lines + explicit | Good for ruled tables, tunable | Yes | MIT |
| Camelot lattice | Morphological lines | Good when borders exist | Yes | MIT |
| Camelot stream | Whitespace column inference | Weak on wide-label tables | Yes | MIT |
| img2table | OpenCV morphological | Borderless alpha; worth testing | Yes | MIT |
| PyMuPDF `find_tables()` | Native C++ | Simple bordered only | Yes | AGPL |
| DocLayout-YOLO | YOLOv10 layout detection | General docs; may help routing | Yes | AGPL |
| StructEqTable | Table image → LaTeX | Handles symbols; arXiv-trained | No (GPU) | MIT |
| TATR (Table Transformer) | DETR+ResNet | Academic bias; domain mismatch | No | MIT |

## What Makes Standards Hard vs. Financial Tables

Financial statement tables:
- Bordered or semi-bordered grid
- Numeric columns are right-aligned, consistent width
- Row labels are long text on the left
- Main difficulty: merged cells, multi-page, continuation

Engineering standard tables:
- Often borderless (only header rule)
- Cells contain Greek symbols, units, inequalities (σ ≥ 235 N/mm²)
- Column widths highly variable
- Main difficulty: table vs. formula confusion, watermark pollution, no grid

See `tables-forms-and-financials.md` for the financial statement class.

## Sources

- DNV-ST-N001 assessment results: target/quick-assess-dnv/ (2026-05-11)
- Camelot docs: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html
- Camelot vs. other tools: https://github.com/camelot-dev/camelot/wiki/Comparison-with-other-PDF-Table-Extraction-libraries-and-tools
- img2table: github.com/xavctn/img2table
- DocLayout-YOLO: github.com/opendatalab/DocLayout-YOLO (arXiv:2410.12628)
- StructEqTable-Deploy: github.com/InternScience/StructEqTable-Deploy
- TATR: github.com/microsoft/table-transformer (arXiv:2110.00061)
- Page-association for footer detection: https://www.researchgate.net/publication/221253782
- MDPI engineering standards table framework: https://www.mdpi.com/2076-3417/10/18/6182
