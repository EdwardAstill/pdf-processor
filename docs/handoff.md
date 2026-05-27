# Handoff — Renderer Refactor + Formula Research

**Date:** 2026-05-27
**Branch:** `main` (committed at `5b5f861`, version 0.4.1)
**Status:** Refactoring complete. Formula OCR research Phase 1 done; Phase 2 ready to execute.

---

## Completed: Renderer refactor (Steps 1-4)

All four steps from the 2026-05-26 handoff are done and committed (`456092e`).

### ✓ Step 1: Commit formatting
Already committed at `b88975d`.

### ✓ Step 2: Table serialization
Extracted 9 functions into `src/render/table.rs` (220 lines). Three entry points
are `pub(crate)`: `render_table`, `render_coordinate_table`, `render_structured_region`.
Six helper functions are private.

### ✓ Step 3: Move tests
Tests extracted to `src/render/markdown/tests.rs` (807 lines) via Rust 2018
directory module pattern (`mod.rs` + `tests.rs`).

### ✓ Step 4: Block constructors
Three of four hand-rolled `Block { ... }` sites in `src/pipeline/mod.rs` now use
`Block::special()`. The Formula site keeps hand-rolled because `text` carries
`source_text` (asserted in tests). Test helper `make_block_at` uses struct update
with `Block::text()`.

Module layout after refactor:

```
src/render/
├── mod.rs
├── markdown/
│   ├── mod.rs       (451 lines)
│   └── tests.rs     (807 lines)
├── media.rs
├── scholarly.rs
├── table.rs         (220 lines)
└── text.rs
```

---

## Completed: Formula LaTeX OCR Research (Phase 1)

Full research artifacts in `.pied/research/formula-latex-ocr/`:
- `PLAN.md` — research plan with phases and lanes
- `FINDINGS.md` — synthesized findings
- `notes/` — per-lane analysis (formula-ocr-tools.md, pdf-pipelines.md, benchmarks.md)
- `raw/` — fetched source content with URLs

### Core finding

**The current formula pipeline has no OCR step.** `build_formula_latex()` in
`src/pipeline/mod.rs` just wraps raw extracted text in `$$...$$`. Every
production pipeline (Docling, Marker, MinerU) uses **visual crop+OCR** because
PDF text layers destroy math semantics (fractions → `/`, Greek letters → random
glyphs, layout destroyed).

### Top tool candidates for SubprocessSidecar

The project already has a `SubprocessSidecar` contract (`src/formula/ocr.rs`)
that invokes an external command: `tool crop.png` → LaTeX to stdout. No tool
is currently wired in.

| Tool | License | CLI | Speed | Integration effort |
|------|---------|:---:|-------|-------------------|
| **RapidLaTeXOCR** | MIT | `rapid_latex_ocr crop.png` | ~480ms CPU | `pip install` |
| pix2tex | MIT | `pix2tex crop.png` | ~500ms GPU | `pip install` (heavier PyTorch) |
| TexTeller | Apache 2.0 | `texteller inference crop.png` | ONNX avail | `uv pip install` |
| Texo | AGPL | Python API only | 311ms GPU | Wrap in CLI script |
| PP-FormulaNet_plus-M | Apache 2.0 | PaddleOCR pipeline | 1040ms GPU | Heavy integration |

**Best immediate fit**: RapidLaTeXOCR — MIT license, ONNX-based (lightweight),
clean stdout output, simple install.

### How production pipelines handle formulas

Every pipeline uses two-stage: **detection** (layout model finds formula regions)
→ **crop+OCR** (recognition model converts image to LaTeX). No pipeline attempts
to extract LaTeX from the PDF text layer.

- **Docling**: RT-DETR layout → CodeFormulaV2 VLM (optional, off by default)
- **Marker/Surya**: Surya layout → RecognitionPredictor
- **MinerU**: YOLOv8 dedicated formula detector → UniMERNet
- **Nougat**: End-to-end (no detection step; Swin+mBART on full page image)

Key insight: Docling's formula enrichment is **off by default**. Even when using
the hybrid backend (`--hybrid docling`), formulas may not get OCR unless
`do_formula_enrichment=True` is set in the pipeline options.

### Benchmark landscape

- **CDM has replaced BLEU** as the standard evaluation metric (aligns 96% with human judgment)
- **PP-FormulaNet_plus-L** is the current open-source accuracy SOTA (92.22 En-BLEU, self-reported)
- **UniMERNet-T** has the best third-party CDM evaluation (0.991 SPE CDM)
- **Texo** is the best lightweight option (20M params, runs in-browser)
- **No end-to-end pipeline benchmarks** exist — all tools evaluated on pre-cropped images

---

## Next: Formula OCR Integration Plan

### Phase 2: Evaluate top candidates (next session)

**Stop when:** the top 2-3 candidates have been tested on real formula crops
from the pdf-processor corpus, with quality and speed measured.

#### Step 2a: Generate test formula crops

```bash
pdfp convert example/pdf/attention.pdf -o /tmp/attn-test --debug-formulas
# This produces debug crop PNGs in the output directory
```

#### Step 2b: Test RapidLaTeXOCR

```bash
pip install rapid_latex_ocr
# For each formula crop:
rapid_latex_ocr debug/formulas/pageN_formulaM.png
# Record: LaTeX quality, inference time, failures
```

#### Step 2c: Test TexTeller (if RapidLaTeXOCR quality insufficient)

```bash
uv pip install texteller
texteller inference debug/formulas/pageN_formulaM.png
```

#### Step 2d: Test Docling hybrid with formula enrichment

Determine whether the existing hybrid backend path already solves this:
```bash
pdfp convert example/pdf/attention.pdf -o /tmp/attn-hybrid \
  --hybrid docling --formulas hybrid
```
Check if formulas in the output are proper LaTeX (with fractions, subscripts,
etc.) or still raw text.

### Phase 3: Integration (after Phase 2 confirms a tool)

The integration path is already designed — wire the chosen tool into the
`SubprocessSidecar` contract in `src/formula/ocr.rs`:

1. **Add CLI flag** — `--formula-sidecar rapid-latex-ocr` (or similar) to
   `src/cli.rs`
2. **Instantiate sidecar** — in `src/pipeline/mod.rs`, create
   `SubprocessSidecar::new("rapid_latex_ocr")` when flag is set
3. **Wire into formula pipeline** — in `build_page()`, call
   `sidecar.recognize(&crop_path)` for each formula candidate and store the
   returned LaTeX in `candidate.latex`
4. **Fall back to raw text** — if sidecar fails, use existing `build_formula_latex()`
   behavior (raw text wrapped in `$$...$$`)
5. **Add tests** — unit test with a mock sidecar, integration test with a real
   formula PDF

### Effort estimate

- RapidLaTeXOCR integration: **1-2 sessions** (CLI flag + sidecar wiring + tests)
- TexTeller or PP-FormulaNet integration: **2-3 sessions** (heavier dependency management)
- Full pipeline with detection quality improvements: **separate effort** (not scoped here)

---

## Files to know

- `plans/stages/renderer-refactor.md` — full refactoring plan
- `plans/stages/rust-python-split-optimisation.md` — hybrid routing plan
- `ARCHITECTURE.md` — architecture overview
- `src/formula/ocr.rs` — SubprocessSidecar contract (already exists, no tool wired)
- `src/formula/detect.rs` — formula detection and LaTeX building
- `src/pipeline/mod.rs` — `build_page()`, `formula_candidates_to_blocks()`, `build_formula_latex()`
- `.pied/research/formula-latex-ocr/FINDINGS.md` — full research synthesis
- `.pied/research/formula-latex-ocr/PLAN.md` — research plan with Phase 2/3 stubs
