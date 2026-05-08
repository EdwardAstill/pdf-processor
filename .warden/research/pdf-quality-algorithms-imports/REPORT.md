# Report: PDF Quality Algorithms And Import Candidates

Date: 2026-05-08

## Executive Recommendation

The repo should not try to solve every PDF issue with one backend. The best path is a layered extractor:

1. Keep the local Rust path as the default for born-digital PDFs.
2. Port targeted coordinate algorithms into the local path for tables and figures.
3. Add optional sidecar adapters for formulas, hard tables, and scanned/image tables.
4. Use the existing quality loop to compare sidecars before making them user-facing defaults.

The highest-value native work is table extraction. The strongest sources are pdfplumber's edge/intersection/cell model, Camelot's Stream and Network text-alignment models, and PDFFigures2's caption-driven figure proposal scoring. The strongest optional sidecars are Docling, gmft, UniMERNet, img2table, and the already-integrated OCRmyPDF path. Marker, Surya, and PDF-Extract-Kit are technically useful, but their GPL/AGPL licensing means they should not be imported into this MIT repo.

## Current Local Baseline

`pdfp` already has the right scaffolding:

- `src/layout/table.rs` keeps `RawWord` geometry and reconstructs coordinate tables from word rows, numeric density, row consistency, confidence, Markdown rendering, layout fallback, and overlap suppression.
- `src/formula/detect.rs` detects formula candidates and can write debug crops, which are the correct unit for formula recognition sidecars.
- `src/figure/detect.rs` detects snapshot candidates from significant embedded images and nearby captions, then `src/figure/render.rs` renders the page region.
- `src/ocr/mod.rs` uses OCRmyPDF as a local sidecar and records cache/provenance.
- `src/hybrid/client.rs` already sends documents to `docling-serve` with OCR, table structure, and formula enrichment enabled.
- `scripts/example-audit.sh` and `docs/QUALITY_LOOP.md` now provide a repeatable research/change/test/observe loop.

The local path therefore needs sharper algorithms and better sidecar routing, not a rewrite.

## Candidate Matrix

| Category | Candidate | Recommendation |
| --- | --- | --- |
| Native-port | pdfplumber | Port its table edge, snap/join, intersection, cell, and grouping ideas into `src/layout/table.rs`. Source: https://github.com/jsvine/pdfplumber |
| Native-port | Camelot Stream/Network | Use as a design reference for whitespace tables and text-alignment graph tables. Source: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html |
| Native-port | PDFFigures2 | Adapt caption proposal, body-text exclusion, proposal scoring, and overlap rejection for `src/figure/detect.rs`. Source: https://github.com/allenai/pdffigures2 |
| Optional sidecar | Docling | Keep and harden `--hybrid docling`; verify current `docling-serve` request/response shapes. Sources: https://github.com/docling-project/docling and https://docling-project.github.io/docling/reference/pipeline_options/ |
| Optional sidecar | gmft | Add an experiment adapter for hard tables; compare outputs against native tables and Docling. Source: https://github.com/conjuncts/gmft |
| Optional sidecar | UniMERNet | Add a crop-level formula recognizer adapter after formula candidates are stable. Sources: https://arxiv.org/abs/2404.15254 and https://github.com/opendatalab/UniMERNet |
| Optional sidecar | img2table | Test on scanned/image tables after OCRmyPDF preprocessing and directly on page images. Source: https://github.com/xavctn/img2table |
| Existing sidecar | OCRmyPDF | Keep as first scan preprocessing path. Source: https://ocrmypdf.readthedocs.io/en/latest/ |
| Benchmark/heavy sidecar | Table Transformer | Use for table detection/structure benchmarks and possible opt-in sidecar, not native Rust import. Source: https://github.com/microsoft/table-transformer |
| Benchmark metric | GriTS | Use as target table-structure metric once gold cell grids exist. Source: https://arxiv.org/abs/2203.12555 |
| Benchmark/opt-in only | PDF-Extract-Kit | Strong layout/formula/OCR/table stack, but AGPL. Source: https://github.com/opendatalab/PDF-Extract-Kit |
| Benchmark/opt-in only | Surya | Strong OCR/layout/table/LaTeX OCR, but GPL/commercial licensing constraints. Source: https://github.com/datalab-to/surya |
| Benchmark only | Marker | Useful PDF-to-Markdown/table oracle, but GPL code and separate weight terms. Source: https://github.com/datalab-to/marker |
| Benchmark first | MinerU | Promising full-document parser with formulas/tables/scans/cross-page tables; verify license/runtime before integration. Source: https://github.com/opendatalab/MinerU |
| Lower priority | GROBID | Better for scholarly metadata/references/TEI than current table/formula/image failures. Source: https://github.com/grobidOrg/grobid |

## Tables

### What The Sources Suggest

pdfplumber is the closest fit to a native Rust improvement. It works over detailed PDF objects and exposes table extraction strategies built from explicit or inferred lines, snapping, joining, intersections, cells, and final table grouping. That maps cleanly to `RawWord`, `Bbox`, and future line/rect extraction in this repo.

Camelot contributes a useful parser taxonomy:

- Stream: groups words into rows, guesses table areas from text edges, infers columns, and assigns words to cells.
- Lattice: rasterizes pages and uses image morphology to find ruled-line tables.
- Network: finds common coordinate alignments across text boxes, prunes unconnected text, grows a table body from a high-connectivity seed, then expands into plausible headers.
- Hybrid: combines Network and Lattice when both provide useful signals.

Table Transformer and gmft show what to expect from ML table structure recognizers: they can recover rows, columns, headers, spanning cells, and confidence from visual crops, but they are better as optional sidecars or benchmark tools than native Rust ports.

### Recommended Table Work

1. Add a pdfplumber-style `TableGraph` beside the current numeric row detector.
   - Input: `RawWord` rows, plus any available PDF line/rect primitives later.
   - Infer vertical and horizontal edges from aligned words.
   - Snap nearby edges, join fragmented edges, build intersections, derive rectangular cells.
   - Assign words into cells by bbox containment.
   - Score row/column consistency and cell coverage.

2. Add a Camelot Network-style text-alignment detector.
   - Build alignments for word/text-line left, center, right, top, middle, and bottom coordinates.
   - Remove elements without both horizontal and vertical connections.
   - Grow candidate table bboxes from high-connectivity seeds.
   - Search above the table body for header rows that fit the inferred column anchors.

3. Add sidecar comparison output for hard tables.
   - Native output: current table JSON.
   - Docling output: Markdown/JSON from `--hybrid docling`.
   - gmft output: Markdown/HTML/CSV/JSON.
   - Optional Table Transformer output: cells/HTML when available.

4. Add a table gold fixture format before adding more heuristics.
   - Start simple: expected rows as JSON arrays.
   - Later: expected bboxes and spanning-cell topology.
   - Long-term: use a GriTS-like metric for topology/location/content.

## Formulas

### What The Sources Suggest

Formula extraction should stay split into detection and recognition. The local detector can find likely display-equation regions, but semantic recovery requires a formula recognizer or a document backend. UniMERNet is the best candidate for a crop-level recognizer because it is formula-specific and Apache-2.0. Docling is the best immediate whole-page backend because `pdfp` already has a hybrid client. PDF-Extract-Kit and Surya are technically useful but licensing makes them unsuitable for direct import.

### Recommended Formula Work

1. Keep `src/formula/detect.rs` as an audit/routing layer.
2. Add table/formula conflict resolution.
   - If a formula candidate overlaps a high-confidence table bbox, downgrade or suppress it.
   - This should reduce false positives in numeric catalogues and financial tables.
3. Add a formula sidecar interface around rendered crops.
   - Input: crop PNG path, page number, bbox, source text, confidence.
   - Output: LaTeX string, confidence, backend name, raw backend payload path.
4. Implement adapters in this order:
   - Docling page backend already available through `--hybrid docling`.
   - UniMERNet crop backend as the first formula-specific adapter.
   - PDF-Extract-Kit only as opt-in benchmark/subprocess because of AGPL.

## Figures And Images

### What The Sources Suggest

PDFFigures2 is the strongest algorithm source for improving snapshots. It is caption-driven and explicitly proposes regions around captions, classifies body text away, scores proposals, rejects overlaps, and renders chosen regions. This fits the local snapshot model because `pdfp` already renders a detected bbox through MuPDF.

Embedded images and snapshots solve different problems. Embedded image extraction is fast and stable when the PDF stores the figure as a raster asset. Snapshot rendering is better for vector charts, axis labels, legends, and composite figures.

### Recommended Figure Work

1. Keep `--figures embedded` as compatibility default and `--figures snapshot` as the complete-visual mode.
2. Improve `src/figure/detect.rs` with PDFFigures2-style proposal generation:
   - identify caption blocks;
   - remove normal body text from the proposal area;
   - propose regions above, below, and near the caption;
   - include graphical/image/vector-heavy regions;
   - score by visual area, caption distance, body-text contamination, and overlap.
3. Add debug overlays for figure candidates so misses can be inspected quickly.
4. Use DeepFigures only as a lower-priority benchmark. The local implementation is closer to PDFFigures2's heuristic/geometry approach than to a full figure ML model.

## OCR And Scans

OCRmyPDF should remain the first scan path. It creates a searchable derivative PDF, which lets the rest of `pdfp` keep using normal word geometry, search, inspect, and Markdown conversion. Direct OCR engines are better reserved for special cases, especially image-only tables.

Recommended scan work:

1. Keep `--ocr auto` and `--ocr force` centered on OCRmyPDF.
2. Add a scanned-table experiment lane using img2table.
3. Compare three cases for each scan-heavy PDF:
   - local without OCR;
   - OCRmyPDF then native extraction;
   - image-table sidecar directly from rendered page/crop.
4. Keep Surya/PaddleOCR as indirect or optional tools unless scan OCR becomes the main bottleneck.

## Full-Document Backends

Docling is the best supported hybrid backend for this repo because it is MIT-licensed, active, and already has local wiring. It should be live-tested against the current `docling-serve` API before relying on it in docs or examples. The current request shape in `src/hybrid/client.rs` is plausible but should be verified with a running server and fixture PDFs.

MinerU and Marker are worth using as benchmark oracles. They can show what good Markdown/JSON might look like on hard examples, but they should not become default dependencies without license/runtime review. Marker is GPL. MinerU's advertised capabilities are strong, but integration should wait for a deliberate license and packaging check.

GROBID is useful when the goal is scholarly metadata, references, and TEI structure. It is not the highest-impact tool for the user's current pain points: tables, formulas, images, and scans.

## Import Plan

### Phase 1: Native Table Hardening

Files:

- `src/layout/table.rs`
- `tests/quality.rs`
- `docs/pdf-internals.md`
- `docs/TESTING.md`

Tasks:

1. Add a `TableGraph` data structure for inferred horizontal/vertical edges and cells.
2. Implement pdfplumber-style edge snapping/joining/intersections from `RawWord` geometry.
3. Add Camelot Network-style alignment counts as a second candidate detector.
4. Emit richer `--debug-tables` JSON: inferred edges, intersections, cells, render mode, confidence reasons.
5. Add gold fixtures for at least one numeric catalogue table and one non-numeric/simple table.

### Phase 2: Formula Crop Sidecars

Files:

- `src/formula/detect.rs`
- `src/pipeline.rs`
- `src/hybrid/client.rs`
- new `src/formula/sidecar.rs`

Tasks:

1. Add table/formula overlap suppression.
2. Define a stable JSON contract for formula crop requests and responses.
3. Implement a UniMERNet command adapter as experimental/opt-in.
4. Keep Docling page enrichment as the default whole-page hybrid route.

### Phase 3: Figure Proposal Scoring

Files:

- `src/figure/detect.rs`
- `src/figure/render.rs`
- `tests/figure_snapshots.rs`

Tasks:

1. Add caption-region proposal generation above/below/near captions.
2. Classify body text away from figure regions.
3. Score and dedupe candidates with reasons.
4. Add debug overlays or JSON enough to reproduce why a region was chosen.

### Phase 4: Sidecar Benchmark Harness

Files:

- `scripts/example-audit.sh`
- new `scripts/sidecar-audit.sh`
- `docs/QUALITY_LOOP.md`
- `docs/TESTING.md`

Tasks:

1. Run native, Docling, gmft, img2table, and optional formula sidecars into separate output dirs.
2. Save per-backend Markdown, debug JSON, crops, and summary metrics.
3. Add a per-case failure ledger with labels: `table`, `formula`, `figure`, `ocr`, `reading-order`.
4. Promote an algorithm only when it improves a labeled failure without regressing existing examples.

## What To Avoid

- Do not import GPL or AGPL code into this MIT repo. Marker, Surya, and PDF-Extract-Kit can be subprocess benchmarks only unless licensing is explicitly changed or approved.
- Do not replace the local pipeline with a monolithic converter. The local Rust path is valuable for fast deterministic operation, inspect/search/page commands, and debugability.
- Do not make formulas look "recovered" from local heuristics. Keep local formula rendering as audit/debug only unless a recognizer returns a real result.
- Do not make ML sidecars default until the example audit shows clear, repeatable improvement and acceptable runtime.

## Next Concrete Experiments

1. Run `scripts/example-audit.sh` and tag each failure in `target/example-audit/summary.md`.
2. Start a current `docling-serve` and run the documented live hybrid test from `docs/TESTING.md`.
3. Create one Python-only scratch experiment for gmft on the table-heavy examples.
4. Create one Python-only scratch experiment for UniMERNet on `debug/formulas/*.png` crops.
5. Port the pdfplumber edge/intersection/cell algorithm into a new internal table module and test it against the same examples.

## Bottom Line

The most pragmatic path is: pdfplumber/Camelot ideas inside the Rust table engine, PDFFigures2 ideas inside the figure snapshot detector, OCRmyPDF for scans, Docling as the immediate hybrid backend, gmft/img2table/UniMERNet as opt-in sidecar experiments, and GPL/AGPL tools as benchmark references only.
