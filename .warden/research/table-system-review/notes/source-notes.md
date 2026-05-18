# Source Notes

## pdfplumber

The official README exposes independent vertical and horizontal strategies:
`lines`, `lines_strict`, `text`, and `explicit`. It also exposes tolerances for
snapping, joining, line length, text grouping, and intersections. The important
design point for this repo is not the exact values; it is that table extraction
has explicit strategy provenance and debug-tunable settings instead of one
undifferentiated candidate stream.

## Camelot

Camelot separates four parsing modes:

- Stream: text-row and whitespace based, derived from PDFMiner word grouping and
  Nurminen-style text edges.
- Lattice: deterministic ruling-line extraction using rendered-page morphology,
  line intersections, and scaled PDF coordinates.
- Network: text-box alignment graph with pruning of unconnected edges.
- Hybrid: combines network cell detection with lattice boundaries when both
  sources exist.

Camelot also exposes a parsing report with accuracy/whitespace, visual debug
plots for line/grid/contour/textedge evidence, and explicit `table_areas` /
`table_regions` controls.

## Tabula

Tabula keeps the user-facing distinction between lattice/spreadsheet mode for
ruled tables and stream mode for unruled tables. That matches the same broad
split used by Camelot and pdfplumber.

## Microsoft Table Transformer / PubTables-1M / GriTS

The TATR project separates table detection, structure recognition, and
functional analysis. PubTables-1M includes table, row, column, and cell boxes,
including blank cells, and the GriTS metric evaluates table structure in matrix
form rather than only page presence.

The useful lesson for this repo is eval design: page-level "table exists" is a
floor, not enough to measure region precision or structure quality.

## Docling

Docling documents separate layout detection, table structure recognition, OCR,
picture classification, VLM conversion, and code/formula stages. Table
structure can use TableFormer fast/accurate modes with cell matching. This makes
Docling a plausible optional sidecar/benchmark for hard table pages, but not a
reason to remove the local deterministic fast path.
