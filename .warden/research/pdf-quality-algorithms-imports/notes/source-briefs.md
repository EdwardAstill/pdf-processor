# Source Briefs

## Tables

The best table path is not one algorithm. It is a selector:

1. Coordinate text path for born-digital PDFs.
2. Ruled-line/image morphology path for scanned or visually ruled tables.
3. ML table structure path for merged cells, multi-row headers, and dense financial/engineering tables.

pdfplumber is the cleanest native-port reference because it exposes the same primitives this repo already keeps: words, chars, lines, rectangles, and bounding boxes. Its strategy model is also directly useful: explicit lines, strict lines, text-inferred lines, and user-supplied lines. Camelot adds a practical parser taxonomy. Stream matches the current word-row path. Network adds graph-based text alignment and pruning. Lattice is better kept as a sidecar because it rasterizes the page and uses image morphology. Table Transformer and gmft are better as optional evaluators or sidecars.

Immediate local gap: `src/layout/table.rs` currently finds table-like runs mostly from numeric density and matching word counts. That is a good start for catalogue tables, but it does not yet infer edges, intersections, spanning cells, explicit ruling lines, or text alignment networks.

## Formulas

Current formula detection is a candidate/audit path, not a recognition path. That is the right split. The PDF text layer usually does not preserve semantic math. The crop is the correct unit of work.

Docling is the best short-term backend because `src/hybrid/client.rs` already enables formula enrichment. UniMERNet is the best crop-level sidecar candidate because it is formula-specific and Apache-2.0. PDF-Extract-Kit is technically attractive because it combines formula detection and recognition, but AGPL makes it unsuitable for direct import into this MIT repo.

Immediate local gap: formula candidates can collide with dense numeric tables. Add a resolver so formulas that overlap high-confidence table regions are suppressed or downgraded before debug/sidecar routing.

## Figures And Images

Current snapshot rendering is the right direction. Embedded image extraction alone misses vector drawings, text labels, axes, and figures made from multiple objects. PDFFigures2 gives the best next algorithm: detect captions, classify normal body text away, propose nearby visual regions, score candidates, reject overlaps, and render page regions.

Immediate local gap: `src/figure/detect.rs` mostly uses significant embedded images and caption proximity. Caption-only estimates are intentionally rough. It should grow toward proposal scoring and body-text exclusion.

## OCR And Scans

OCRmyPDF remains the simplest first pass because it produces a searchable derivative PDF that the rest of the local pipeline can process normally. Direct image OCR should be reserved for cases where a searchable derivative is insufficient, especially scanned tables.

img2table is worth testing on scanned/image tables because it combines image table detection with pluggable OCR. Surya and PaddleOCR are strong image/OCR stacks, but licensing/runtime complexity means they should be optional or indirect first.

## Evaluation

Tables need structure-aware metrics. GriTS is the strongest target because it evaluates table topology, location, and content as matrix similarity. For near-term work, add a simpler gold-table fixture format first, then graduate to GriTS-like scoring once table candidates serialize row/column/cell geometry.

The current `scripts/example-audit.sh` is a useful smoke loop. It should grow a per-case expected-failure ledger with labels such as `table`, `formula`, `figure`, `ocr`, and `reading-order`, plus sidecar comparison outputs under `target/example-audit/<backend>/`.
