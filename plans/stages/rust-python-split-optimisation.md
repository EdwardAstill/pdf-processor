# Rust/Python Split Optimisation Plan

**Date:** 2026-05-26
**From:** `.pied/research/wiki-expansion/FINDINGS.md` and `wiki/topics/rust-python-split.md`

## Audit findings

Three of the five areas identified in the wiki analysis are already implemented:

| Area | Status | Detail |
|---|---|---|
| Furniture suppression | ✅ Active | `src/layout/furniture.rs` → `detect_furniture_bboxes()` called in pipeline. Excludes furniture from formula detection and suppresses text blocks inside furniture zones. |
| Structure tree | ✅ Active (feature-gated) | `pdfium-metadata` feature → `FPDF_StructTree_GetForPage()` → `FontInfoProvider` returns struct-tree roles. Used in `classifier.rs` for authoritative heading detection. Tests exist. Requires libpdfium installed. |
| ONNX preferred over Python | ✅ Active (feature-gated) | `onnx-ocr` feature → `OnnxFormulaSidecar` via `ort` crate. Formula sidecar architecture supports command (Python) or ONNX (Rust). ONNX is the preferred path when feature is enabled. |

Two areas remain to address:

| Area | Status | Detail |
|---|---|---|
| Hybrid routing granularity | ⚠️ Per-page only | `override_markdown` replaces entire page. One hard table → lose clean headings/paragraphs. |
| Python eval companion | ❌ Missing | No standard benchmark integration. Rust eval uses custom fixtures only. |

## Changes needed

### 1. Fine-grained hybrid routing (Stage 3 scope)

**Current**: `should_route()` decides per-page → entire page sent to Docling → `page.override_markdown` replaces all output.

**Target**: Per-region routing. The local pipeline extracts all blocks. Only hard regions (formulas, complex tables, layout-complex pages) get routed. Clean blocks stay local.

**Implementation approach**:

1. Keep existing per-page routing as the default (simple, works).
2. Add `--hybrid region` mode that:
   - Runs local pipeline fully first (all blocks extracted, classified, tables/formulas detected)
   - For formula blocks: routes crop-only to formula OCR sidecar (already supported via `--formula-sidecar`)
   - For table blocks: routes table region as single-page PDF to Docling, extracts only the table markdown from response, merges back into local blocks
   - For layout-complex pages: keeps current per-page routing as fallback

3. New architecture:

```
Local pipeline → blocks[] → for each block:
  ├─ Formula block → formula sidecar (ONNX or Python) → LaTeX → emit as $$ block
  ├─ Table block  → extract page region → Docling → extract table markdown → merge
  ├─ Everything else → local rendering
  └─ Layout-complex page → full page → Docling → override (current behavior)
```

**Files touched**:
- `src/hybrid/mod.rs` — new `apply_regions()` function
- `src/hybrid/triage.rs` — `RegionRouteDecision` enum
- `src/hybrid/client.rs` — `convert_region_to_markdown()` method
- `src/pipeline/mod.rs` — dispatch between per-page and per-region modes
- `src/cli.rs` — new `--hybrid-mode` flag (`page` | `region`)

**Estimated LOC**: ~300-400
**Stage**: 3 (medium features)
**New dependencies**: None

### 2. Python eval companion (Stage 4 scope)

**Current**: Rust eval framework (`src/eval/`) uses custom JSON fixtures. No integration with standard benchmarks.

**Target**: Python companion script that runs standard benchmarks on `pdfp` output.

**Implementation approach**:

1. `tools/eval_benchmarks/` directory with:
   - `requirements.txt` — pypdf, datasets (for OmniDocBench), table-metric-study
   - `run_omni_doc_bench.py` — run `pdfp convert` on OmniDocBench PDFs, score against ground truth
   - `run_table_bench.py` — run GriTS/T-LAG scoring on table output
   - `compare.py` — compare `pdfp` scores against baseline (Docling, Marker, etc.)

2. `pdfp` already emits structured output → Python reads the JSON/Markdown → scores against benchmark ground truth.

3. Output: JSON report with per-document-type scores, comparison table.

**Files created**:
- `tools/eval_benchmarks/run_omni_doc_bench.py`
- `tools/eval_benchmarks/run_table_bench.py`
- `tools/eval_benchmarks/compare.py`
- `tools/eval_benchmarks/requirements.txt`

**Estimated LOC**: ~400 Python
**Stage**: 4 (advanced — OmniDocBench alignment is already Stage 4.4)
**New dependencies**: `datasets`, `pypdf`, `table-metric-study` (Python packages)

## Implementation priority

| Change | Priority | Stage | Effort | Rationale |
|---|---|---|---|---|
| Fine-grained hybrid routing | Medium | 3 (after Stage 2 quick wins) | ~400 LOC Rust | Unlocks hybrid quality on mixed-content pages without losing local pipeline accuracy on clean regions |
| Python eval companion | Low | 4 (with OmniDocBench alignment) | ~400 LOC Python | Needed for competitive benchmarking; parallel to Stage 4.4 work |

## What NOT to change

The following are already implemented correctly and should NOT be changed:

- **Furniture suppression**: Works. `detect_furniture_bboxes` uses 8% page-edge zones, 50% repeat threshold, signature-based dedup. Used in formula exclusion and text suppression. Tests exist in `pipeline/merge.rs`.
- **Structure tree integration**: Works when `pdfium-metadata` feature is enabled. `FontInfoProvider` returns struct-tree roles. Classifier tests cover `struct_tree_role_h2_wins_over_size_ratio`, `struct_tree_title_maps_to_h1`, `struct_tree_artifact_suppresses_block`, `struct_tree_list_item`, `struct_tree_table_cell_roles`. The feature-gated design is correct — pdfium is not always available.
- **ONNX formula OCR**: Works. `onnx-ocr` feature → `OnnxFormulaSidecar` → `ort` crate. The formula sidecar architecture (trait `FormulaSidecar` with command and ONNX implementations) is correct. ONNX is already the preferred path when available.

## Performance gate

Both changes must pass the standard gate:
1. `cargo clippy -- -D warnings` — clean
2. `cargo test` — all passing
3. `cargo build --release` — succeeds
4. Conversion benchmark — time and output must not regress on standard test PDF
5. Eval fixtures — scores must not regress

For the Python eval companion, the gate is:
1. `python tools/eval_benchmarks/run_omni_doc_bench.py` — runs without error
2. Scores are reproducible (same pdfp binary + same benchmark data = same scores)
