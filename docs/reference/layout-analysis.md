# Layout Analysis (`src/layout/`)

Reconstructs reading order, classifies content blocks, detects tables, and suppresses page furniture. This is the core structural intelligence layer.

## Source files

| File | Purpose |
|---|---|
| `mod.rs` | Module root |
| `xycut.rs` | XY-Cut++ recursive reading-order algorithm |
| `classifier.rs` | Block classification (heading, paragraph, list, table cell, caption, code) |
| `table.rs` | Coordinate-based table detection: numeric rows, alignment tables, form-column clustering |
| `table_detector.rs` | Geometry-based table detection: rule-line grids and bands |
| `table_inference.rs` | Post-classification inference: implicit numeric tables and form-field lists from paragraphs |
| `furniture.rs` | Page furniture detection (running headers, footers, page numbers, watermarks) |
| `drawing_ops.rs` | Drawing operation analysis (horizontal/vertical rule lines from MuPDF page display list) |

## Key types

| Type | Definition | Module | Purpose |
|---|---|---|---|
| `BlockKind` | enum | `document/types.rs` | Tag for every block: `Heading`, `Paragraph`, `ListItem`, `TableCell`, `CoordinateTable`, `Caption`, `CodeBlock`, `PageNumber`, `RunningHeader`, `RunningFooter`, `Artifact`, `Image`, `Formula`, `FormulaReview`, `Figure` |
| `WordRow<'a>` | struct (private) | `table.rs` | A temporary row of words grouped by baseline for table detection |
| `TableCandidate` | struct (pub(crate)) | `table.rs` | A detected table with `DetectedTable`, `source_block_ids`, and `TableEvidence` |
| `TableEvidence` | struct | `table.rs` | Scoring struct: `source`, `row_consistency`, `column_alignment`, `numeric_density`, `row_count`, `caption_score`, `broad_page_penalty`, `prose_penalty`, `debug_reasons` |
| `TableEvidenceSource` | enum | `table.rs` | How a table was found: `RulingGrid`, `RulingBand`, `TextAlignment`, `NumericRows`, `ExplicitRegion`, `ExternalModel` |
| `GeometryTableRegion` | struct (pub(crate)) | `table_detector.rs` | Region detected from rule-line geometry, with rows and confidence |
| `StructuredRegion` / `StructuredRegionKind` | enums | `table_inference.rs` | Post-classification inference results: `TableCells`, `NumericTable` (with headers/rows/total), `FormFields` |
| `HLine` / `VLine` | structs | `drawing_ops.rs` | Extracted horizontal/vertical rule lines from MuPDF display list |

## Key functions

### Reading order (`xycut.rs`)

| Function | Description |
|---|---|
| `build_xycut_order(blocks, config) -> Vec<Vec<Bbox>>` | Recursive XY-Cut++: project blocks onto axes, find largest gap, recurse. Returns ordered groups of bounding boxes |
| `assign_reading_order(order, blocks)` | Apply the XY-Cut output order to blocks in place |

The algorithm is a Rust port of OpenDataLoader's `XYCutPlusPlusSorter.java` (arXiv:2504.10258), Apache-2.0. Coordinate system is top-left origin (Y down), flipped relative to the Java reference.

### Classification (`classifier.rs`)

| Function | Description |
|---|---|
| `PageClassifier::classify_page(blocks, ...)` | Assign `BlockKind` to each text block based on font size, position, content patterns |
| `classify_page_with_metadata(blocks, ..., metadata)` | Version that uses pdfium struct-tree roles when available |
| `detect_table_cells(blocks) -> HashMap<block_id, BlockKind>` | Legacy cell detection by font-size alignment |

Heading levels are derived from font-size ratio against body text mode:
- ≥2.0× body → H1, ≥1.6× → H2, ≥1.35× → H3, ≥1.15× → H4, else → H5

With struct-tree metadata, tagged roles (`H1`..`H6`, `Title`) override size-based heuristics, and bold-at-body-size gets promoted to H4.

### Table detection (`table.rs`)

| Function | Description |
|---|---|
| `detect_coordinate_tables(words, page_width, mode) -> Vec<TableCandidate>` | Main entry: tries numeric-row detection, then alignment, then form-column clustering |
| `detect_form_column_tables(rows, page_width, mode)` | **MarkItDown-style** global X-column clustering. Groups words into rows, finds stable column positions, assigns words into column slots with blank-cell preservation |
| `detect_alignment_tables(rows, page_width, mode)` | Text-alignment detection: finds runs of rows with consistent column alignment |
| `detect_captioned_table_runs(rows, page_width, mode)` | Runs following "Table N:" captions |

### Geometry table detection (`table_detector.rs`)

| Function | Description |
|---|---|
| `detect_table_region_candidates(hlines, vlines, words, ...) -> Vec<GeometryTableRegion>` | Detects regions from horizontal rule-line pairs, rule-line grids, and whitespace alignment |
| `geometry_region_to_table_candidate(region, mode) -> Option<TableCandidate>` | Converts a geometry region into the standard table candidate format |

### Table inference (`table_inference.rs`)

| Function | Description |
|---|---|
| `detect_structured_region(blocks, start) -> Option<StructuredRegion>` | Scans consecutive paragraph/list blocks for implicit numeric tables (invoice rows) or form-field lists (label: value pairs) |
| `strip_leading_list_marker(text) -> String` | Removes "1.", "a)", "• " etc. from the start of text |
| `looks_like_field_label(text) -> bool` | Heuristic for label:value patterns |

### Furniture (`furniture.rs`)

| Function | Description |
|---|---|
| `build_furniture_mask(pages, ...) -> HashMap<page_num, Vec<Bbox>>` | Identifies repeated headers, footers, watermarks, and page numbers across the document |
| `collect_unique_page_text(pages) -> ...` | Builds frequency map of text fragments across pages; repeats ≥ threshold become furniture |

## CLI flags

| Flag | Affects |
|---|---|
| `--tables auto\|native\|layout\|off` | Controls `TableMode` passed to all table detection functions |
| `--debug-tables` | Writes `debug/tables/pageN.json` with table candidates and evidence |
| `--conservative` | Forces `TableMode::Layout`, disables speculative Markdown tables |
| `--min-h-gap <pts>` | XY-Cut horizontal split threshold (default 8.0) |
| `--min-v-gap <pts>` | XY-Cut vertical split threshold (default 12.0) |

## Cross-references

- `wiki/algorithms/reading-order.md` — XY-Cut++ algorithm details
- `wiki/algorithms/table-detection.md` — all table detection strategies
- `wiki/structures/tables.md` — table content types and rendering
- `wiki/topics/layout-and-reading-order.md` — layout analysis pipeline overview
- [pdf-extraction.md](pdf-extraction.md) — where words come from
- [pipeline.md](pipeline.md) — how table and formula detection interact
