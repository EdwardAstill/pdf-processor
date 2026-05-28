# Handoff — Formula Quality Follow-up

**Date:** 2026-05-28
**Branch:** `main` at `5800c48` before this handoff edit
**Status:** Formula OCR plumbing exists. The remaining work is validation and
quality improvement on real documents, especially DNV-style standards.

---

## Read this first

The old handoff overstated the remaining implementation work. Do **not** plan a
new sidecar integration pass before checking the current code.

Implemented already:

- CLI flag: `--formula-sidecar <SIDECAR>` in `src/cli.rs`
- Command sidecar: `SubprocessSidecar` in `src/formula/ocr.rs`
- Feature-gated ONNX sidecar: `OnnxFormulaSidecar` in `src/formula/ocr_onnx.rs`
- Pipeline wiring: `build_formula_sidecar()`, crop rendering, and
  `sidecar.recognize()` calls in `src/pipeline/mod.rs`
- Tests: `tests/formula_ocr.rs` and feature-gated `tests/formula_onnx.rs`
- Docling hybrid requests already send `do_formula_enrichment=true` in
  `src/hybrid/client.rs`

What remains is not "add OCR plumbing". It is: prove which OCR path produces
usable LaTeX on the project corpus, then reduce detection errors that make the
crops unreliable.

---

## Current formula behaviour

Local formula handling is an audit and recovery path, not a reliable default
LaTeX extractor.

- `--formulas auto` detects likely display equations and emits high-confidence
  candidates as display math.
- `--debug-formulas` writes JSON ledgers and crop PNGs under `debug/formulas/`.
  It also enables a visual scan for isolated rendered equation bands that are
  missing from the PDF text layer.
- Visual-only formula regions are emitted as `formula-review` comments unless a
  sidecar recovers LaTeX.
- `--formula-sidecar <CMD>` or `cmd:<CMD>` sends high-confidence crops to a
  local command. The command receives the crop PNG path and should print LaTeX
  to stdout.
- Builds with `--features onnx-ocr` also accept `--formula-sidecar
  onnx:<model-dir>` where the model directory contains `encoder.onnx`,
  `decoder.onnx`, and `vocab.txt`.
- If no sidecar succeeds, `build_formula_latex()` falls back to PDF-extracted
  source text plus small Unicode-to-LaTeX cleanup. That fallback is useful, but
  it cannot reconstruct fractions, roots, matrices, or layout-heavy equations.

Command-name caveat: the research notes cite `rapid_latex_ocr`; current README
and CLI examples cite `rapid-latex-ocr`. Verify the installed package's actual
entry point before standardising docs or scripts.

2026-05-28 update: entry point is `rapid_latex_ocr` (underscore). The PyPI
package is `rapid-latex-ocr` (hyphen). Docs now use `rapid_latex_ocr` for
command examples.

---

## Open problems

### 1. Formula OCR quality has not been measured on the target corpus

The sidecar path is implemented, but it has not been benchmarked on real formula
crops from the documents that matter. The next session should compare at least:

- command sidecar with RapidLaTeXOCR, after verifying the command name
- native ONNX sidecar if model files are available
- Docling hybrid output with formula enrichment already enabled

Record both accuracy and runtime. Do not rely on README claims or generic OCR
benchmarks as proof.

### 2. DNV-style standards still produce noisy formula candidates

See `next.md` for the concrete DNV audit. Main failures:

- alpha-factor and symbol-heavy tables are often flagged as formulas
- reference sections can still produce math-like false positives
- some important displayed equations are visible in page renders but absent from
  the text layer
- local text fallback does not reconstruct true LaTeX structure

The current crop generator is useful for review, but not yet enough for reliable
standards conversion.

### 3. Evaluation needs a repeatable fixture loop

Before tuning heuristics, create a small ignored local corpus or fixture set
with representative formula cases:

- clean text-backed equations
- visual-only equations
- formula-like tables that should remain tables
- reference pages that should not emit formula candidates
- DNV pages from `next.md` such as pages 69-71, 130, 389, 597, 670, and 675

Each candidate OCR backend should be run on the same crops and outputs should be
kept under `target/` or another ignored evaluation directory.

---

## Suggested next session

1. Generate fresh formula debug crops for a small target corpus.

   ```sh
   pdfp convert <pdf> -o target/formula-eval/<name> --debug-formulas --conservative
   ```

2. Verify the RapidLaTeXOCR executable name from the installed package.

   ```sh
   command -v rapid_latex_ocr || true
   command -v rapid-latex-ocr || true
   rapid_latex_ocr --help || rapid-latex-ocr --help
   ```

3. Run the implemented sidecar path on the same PDFs.

   ```sh
   pdfp convert <pdf> -o target/formula-eval/<name>-sidecar \
     --debug-formulas --formula-sidecar <verified-command>
   ```

4. Compare debug JSON, Markdown output, and crop-level LaTeX manually for the
   representative pages. Record findings in a new evaluation note, not in this
   handoff.

5. Only after that, choose the next implementation change. Likely candidates:
   table/formula suppression improvements, reference-section suppression, or a
   small benchmark script for sidecar comparisons.

---

## Files to know

- `next.md` — current DNV formula extraction audit and failure examples
- `src/pipeline/mod.rs` — formula candidate flow, crop rendering, sidecar calls,
  and local fallback LaTeX construction
- `src/formula/detect.rs` — text-backed formula candidate detection
- `src/formula/visual.rs` — visual-only formula candidate detection
- `src/formula/ocr.rs` — sidecar trait and subprocess implementation
- `src/formula/ocr_onnx.rs` — feature-gated native ONNX implementation
- `src/hybrid/client.rs` — Docling request options, including formula enrichment
- `tests/formula_ocr.rs` — command sidecar regression coverage
- `tests/formula_onnx.rs` — feature-gated ONNX/parser coverage
- `.pied/research/formula-latex-ocr/FINDINGS.md` — background research only;
  treat implementation recommendations there as stale unless source confirms
  them
