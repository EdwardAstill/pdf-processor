# Handoff — RapidLaTeXOCR Benchmark & ONNX Path Polish

**Date:** 2026-05-28
**Branch:** `main` at `451c49f`
**Status:** RapidLaTeXOCR installed, benchmarked, sidecar routing fixed, ONNX
native path compiles and unit-tests pass but inference too slow on CPU without
GPU execution provider.

---

## This session (2026-05-28)

Three commits:

| Commit | What |
|---|---|
| `d64d872` | Sidecar routing, benchmark, docs |
| `b90c8de` | ONNX decode fixes (vocab, BPE merge, mask input) |
| `451c49f` | ONNX decode loop optimization |

---

## RapidLaTeXOCR integration

### Installation

```sh
uv tool install rapid-latex-ocr --with requests
```

The executable is `rapid_latex_ocr` (underscore). The PyPI package is
`rapid-latex-ocr` (hyphen).

### Sidecar routing fix

`should_send_to_formula_sidecar` was changed from:
- `confidence >= 70 && status == LocalCandidate` (56 candidates)
- to `confidence >= 65` (any status, 75 candidates including visual-only)

This means higher-confidence NeedsReview candidates, including visual-page-
render crops (conf 68), now reach the sidecar for LaTeX recovery.

### Quality benchmark (math-number-theory.pdf)

| Candidate | Geometric LaTeX | RapidLaTeXOCR |
|---|---|---|
| Conv equation | `\label {eq:conv ...` missing braces | Structured `\begin{array}`, correct subscripts ✅ |
| Matmul equation | `\labe l eq:matmul }...` garbled | `\hat{Y}=\hat{X}\hat{W}` correct ✅ |
| RUNTIME equation | `\labe l ^{{eq:r ...` garbled | Recognisable, minor errors |
| CEIL_smooth | `^{m} ^{o} h c eilin...` broken | Nested `\sum`, partial recovery |
| Lambda params | Duplicates | Clean `\underline{\lambda}` ✅ |

**Conclusion**: RapidLaTeXOCR produces significantly better LaTeX than local
geometric recovery on real formulas. Geometric is a fast fallback;
RapidLaTeXOCR should be preferred when quality matters.

### Per-crop timing (Python onnxruntime 1.26.0, CPU)

- 1 token: 26ms
- 21-token formula (blank image): 398ms total (19ms/token avg)
- Estimated 56 crops × 20 tokens: ~22s total

---

## ONNX native path

### What works

- `cargo build --features onnx-ocr` compiles and produces `pdfp`
- `--formula-sidecar onnx:<model-dir>` parses correctly
- 11 ONNX unit tests pass: vocab loading, BPE merge, preprocessing shape,
  module structure, model path validation
- Vocal.txt generated from installed `tokenizer.json` (1175 tokens)
- BPE backslash merge fixed: `\ mathrm` → `\mathrm`
- Decoder mask input added (3rd input `mask: tensor(bool)`)
- Context tensor pre-allocated once per crop (was cloning 517KB per step)

### What doesn't work

- Full inference is too slow on this CPU when using the `ort` crate (ONNX
  Runtime 1.24.2). The `ort` crate bundles a custom build from
  `cdn.pyke.io` which appears slower than Python's onnxruntime 1.26.0 on
  x86_64 Linux.
- Each crop takes 10+ seconds in Rust vs ~400ms in Python.
- The pipeline times out before completing a full document.

### Known gap

Compare the ort crate's ONNX Runtime build with the pip onnxruntime 1.26.0.
Likely causes:
- Missing CPU feature detection (AVX, SSE)
- Missing OpenMP threading
- ort 2.0.0-rc.12 uses ORT 1.24.2 vs Python's 1.26.0
- Model may need KV-cache support for efficient autoregressive decode

### Recommended next work for ONNX

1. Investigate why ort crate is slower — try `ORT_CACHE_DIR` / custom ORT
   library path with a pip-installed onnxruntime .so
2. Or: wait for ort >= 2.0.0 stable with newer ORT version
3. Or: use the subprocess sidecar as the production path (proven, acceptable
   speed for batch processing)

---

## Table output

Checked: Markdown pipe tables are already produced in `TableMode::Auto` when
`row_consistency >= 0.80` and `rows >= 3`. Layout fallback for complex
tables is correct. The real weakness is table *detection recall* on
borderless DNV/standards tables, not formatting.

---

## Files changed this session

| File | Change |
|---|---|
| `README.md` | Command name: `rapid_latex_ocr` (not `rapid-latex-ocr`) |
| `docs/CLI.md` | Same command name fix |
| `src/formula/geometric.rs` | Clippy fix (redundant closure) |
| `src/pipeline/mod.rs` | Sidecar routing: remove `latex.is_none()` gate; lower conf threshold to 65 |
| `src/formula/ocr_onnx.rs` | Vocab from `tokenizer.json`, BPE merge, mask input, context reuse |
| `docs/handoff.md` | Updated handoff |
| `wiki/structures/equations.md` | Benchmark results, production timing estimates |

---

## Files to know

- `wiki/structures/equations.md` — updated with benchmarks
- `wiki/algorithms/formula-detection.md` — detection algorithms
- `wiki/topics/technical-standards-documents.md` — DNV failure modes
- `src/formula/ocr.rs` — sidecar trait and subprocess implementation
- `src/formula/ocr_onnx.rs` — native ONNX implementation
- `src/pipeline/mod.rs` — candidate flow and sidecar calls
- `tests/formula_ocr.rs` — subprocess sidecar tests
- `tests/formula_onnx.rs` — ONNX unit tests

---

## Next session (suggested)

1. **Install** `rapid_latex_ocr` and run a full evaluation on attention.pdf
   and DNV standards pages:
   ```sh
   pdfp convert paper.pdf --no-images --debug-formulas \
     --formula-sidecar rapid_latex_ocr -o target/eval/
   ```
2. **Compare** geometric vs sidecar LaTeX in `debug/formulas/index.json`.
3. **Measure** false positive suppression — does lowering the sidecar threshold
   help or hurt on real documents?
4. **Fix** ONNX native path by testing an updated ort crate or custom ORT lib.
5. **Evaluate** whether to invest in ONNX performance or stay with subprocess
   path.
