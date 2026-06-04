# Formula Detection (`src/formula/`)

Detects display-equation candidate regions from word geometry, recovers LaTeX from word positions, and optionally sends crops to an external OCR sidecar for recognition.

## Source files

| File | Purpose |
|---|---|
| `mod.rs` | Module root. Re-exports `detect_formula_candidates`, `FormulaCandidate`, `detect_visual_formula_candidates` |
| `detect.rs` | Word-geometry formula candidate detection |
| `geometric.rs` | Geometric LaTeX recovery from word positions (superscript, subscript, fraction stacking) |
| `visual.rs` | Visual formula band scanning via page rendering |
| `ocr.rs` | Subprocess formula OCR sidecar dispatch (spawns external command) |
| `ocr_onnx.rs` | Native ONNX Runtime formula OCR (feature `onnx-ocr`, optional) |

## Key types

| Type | Module | Purpose |
|---|---|---|
| `FormulaCandidate` | `detect.rs` | A candidate formula region: `bbox`, `confidence`, `status`, `source_text`, `latex`, `equation_number`, `crop_path`, `sidecar` attempt, `sanity`, `emission_reason` |
| `FormulaStatus` | `detect.rs` | `LocalCandidate`, `NeedsReview`, `VisualBand`, etc. |
| `FormulaSidecarAttempt` | `ocr.rs` | Results of an OCR sidecar call: `status` (recovered/empty/timeout/failed), `latex`, `timing_ms`, `stderr_summary` |

## Key functions

### Detection (`detect.rs`)

| Function | Description |
|---|---|
| `detect_formula_candidates(raw_page, excluded_regions) -> Vec<FormulaCandidate>` | Main entry. Scans words for display-equation patterns: centering, tall characters, math operators, equation numbers |
| `suppress_formula_candidates_overlapping_tables(candidates, tables)` | Filters candidates that overlap high-confidence table regions |

The detector uses heuristics: centered lines, significant character height variance (superscripts/subscripts), math Unicode ranges, equation-number patterns like `(3)` or `(1.2)`.

### Geometric LaTeX (`geometric.rs`)

| Function | Description |
|---|---|
| `recover_geometric_latex(words, bbox) -> String` | Reconstructs a LaTeX approximation from word positions. Detects superscripts, subscripts, fractions (stacked rows), large operators |

This is a fast fallback, not reliable for production use. The quality varies significantly by PDF.

### Visual band scanning (`visual.rs`)

| Function | Description |
|---|---|
| `detect_visual_formula_candidates(pdf_path, raw_page, existing_candidates, excluded) -> Vec<FormulaCandidate>` | Renders the page at low DPI, scans for isolated dark horizontal bands near formula cues (Hence:, where:), adds review-only candidates |

Active only when `--debug-formulas` is enabled. Visual-only candidates are emitted as `formula-review` comments, not LaTeX.

### OCR sidecar (`ocr.rs`)

| Function | Description |
|---|---|
| `spawn_subprocess_sidecar(crop_path, command, timeout) -> FormulaSidecarAttempt` | Spawns an external command, passes crop PNG as first arg, reads LaTeX from stdout |
| `should_send_to_formula_sidecar(candidate) -> bool` | Policy gate: rejects prose-like, table-like, or broad ambiguous candidates |

The sidecar path supports:
- Bare command: `--formula-sidecar rapid_latex_ocr`
- Explicit command: `--formula-sidecar cmd:/path/to/ocr.sh`
- ONNX native: `--formula-sidecar onnx:<model-dir>` (requires `--features onnx-ocr`)

`rapid_latex_ocr` uses a persistent Python worker so model startup is paid once per conversion. Generic commands receive the crop path as their first argument and should print LaTeX to stdout.

## CLI flags

| Flag | Effect |
|---|---|
| `--formulas auto\|local\|hybrid\|off` | Controls `FormulaMode`. Auto: emit high-confidence candidates as display math. Local: emit all text-backed candidates. Hybrid: route to Docling. Off: disable |
| `--debug-formulas` | Enable all debug output: crop PNGs, `index.json` ledger, visual band scanning |
| `--formula-sidecar <CMD>` | External OCR command for LaTeX recovery on selected crops |
| `--formula-sidecar-timeout-secs <N>` | Per-crop timeout (default 30) |
| `--formula-emit conservative\|auto\|all\|none` | Controls emission policy within auto/local modes |
| `--conservative` | Disables heuristic formula rendering (audit-only) |

## Cross-references

- `wiki/algorithms/formula-detection.md` — detection algorithms and false-positive suppression
- `wiki/structures/equations.md` — equation content types and sidecar benchmarks
- `wiki/tools/equation-ocr-models.md` — RapidLaTeX-OCR, UniMERNet, pix2tex
- `wiki/topics/technical-standards-documents.md` — DNV failure modes
- [layout-analysis.md](layout-analysis.md) — how table regions block formula candidates
- [pipeline.md](pipeline.md) — formula candidate routing and emission
