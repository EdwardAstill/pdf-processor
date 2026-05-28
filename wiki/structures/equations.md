---
title: "Formula Detection, False Positive Suppression, and OCR Sidecars"
kind: "reference"
category: "wiki"
summary: "Covers the three-stage formula pipeline: visual/geometric detection, false positive filtering, and optional OCR sidecar for LaTeX reconstruction — including tool options, heuristics, and Rust integration paths."
virtual_path: "wiki/topics/formula-detection-and-ocr"
entities: [UniMERNet, RapidLaTeX-OCR, Texo, Pix2Tex, ScanSSD, DocLayout-YOLO, PDF-Extract-Kit, ort-crate, ONNX]
---

# Formula Detection, False Positive Suppression, and OCR Sidecars

Formula handling is a three-stage problem. Detection, filtering, and OCR are
separate concerns that need separate tools and separate evaluation. Conflating
them leads to pipelines that are hard to improve.

## Stage 1 — Detection

The goal is to identify bounding regions that *might* contain display math.
`pdfp` uses two complementary detectors:

**Word-based detector** — scans extracted text spans for non-alphanumeric
symbols, isolated short lines, Greek letters, and operator clusters.
Produces candidates with source `word`.

**Visual-band detector** (implemented 2026-05) — renders the page at low DPI
via MuPDF, scans horizontal bands for dense dark pixels, filters by isolation
and cue-word proximity. Catches visible equations that the word-based detector
misses (e.g. DNV page 670, fraction-style display math). Produces candidates
with source `visual-page-render`. Only runs when `--debug-formulas` is enabled.

### Signal taxonomy

Geometry signals that distinguish display math from other content:

| Signal | Formula value | Table-row value | Decorative-rule value |
|--------|--------------|-----------------|----------------------|
| Symbol density (non-alphanum / total chars) | High (>30%) | Low–medium | N/A (no chars) |
| Aspect ratio (width:height) | Wide (5:1–10:1) typical | Narrow–medium | Extremely wide (>50:1) |
| Pixel height of dark band | Matches glyph heights | Matches glyph heights | 1–3px thin line |
| Vertical isolation (gap above/below) | Large | Small (in-row) | Large but at margin |
| Cue words nearby ("where", "=", "defined as") | Often present | Rarely | Never |

ScanSSD (arXiv:2003.08005) validates aspect ratio {5, 7, 10} at 600 dpi as
strong formula signals. The key differentiator from table rows is that formula
characters cluster tightly with variable inter-character spacing, whereas table
rows have uniform column gaps.

## Stage 2 — False Positive Suppression

The current visual detector has known false positive classes. Each requires a
different filter.

### Horizontal rules and logo bars

**Problem**: Long thin horizontal rules (page dividers, logo bars) create high
dark-pixel bands at low DPI.

**Fix**: Morphological filter — compute the ratio of band height to band width.
A structuring element of 1×N pixels extracts horizontal line segments. Bands
narrower than 3px in rendered height relative to normal glyph heights are
decorative. This is the approach used by OpenCV morphological line extraction
(`MORPH_OPEN` with horizontal kernel).

DNV page 1 crops the logo/header bar because the visual band is wide and dark
but has height close to zero in pixel terms. Adding a minimum-band-height
threshold (or checking that the crop contains actual Unicode text spans) would
suppress it.

### Table rows with symbols

**Problem**: Rows like `σ_y ≥ 235 MPa` have non-alphanumeric symbols but are
table data, not display math. These generate formula candidates and suppress
table candidates.

**Fix**: The correct long-term fix is to detect the table first and then exclude
cells from formula candidacy. Short-term: if a band is within N pixels of
adjacent same-width bands with consistent vertical rhythm, it is more likely a
table row than isolated display math.

DocLayout-YOLO addresses this at the layout model level by separating "Isolated
Formula" from "Table" as distinct class labels, training on 10 categories
simultaneously. PDF-Extract-Kit keeps formula detection separate from layout
detection as a dedicated YOLOv8_ft model with its own thresholds.

### Running headers, footers, and watermarks

**Problem**: Page-top and page-bottom text bands (running headers, "Downloaded
by…" watermarks) appear at consistent Y positions.

**Fix**: Page-association method — text bands appearing at the same Y coordinate
with high text similarity across ≥3 consecutive pages are classified as
furniture. This is a pre-pass that builds a per-document furniture mask before
formula detection runs.

### Reference list entries

**Problem**: Numbered bibliography entries `[1] Author, Title…` get flagged
because they start with bracket+number patterns.

**Fix**: Contextual text check — if a candidate band contains a leading
`[N]` or `(N)` pattern followed by alphabetic runs longer than 20 chars, and
has no operator or Greek characters, classify as reference.

## Stage 3 — OCR Sidecar (LaTeX Reconstruction)

Once a crop is validated, local LaTeX reconstruction is weak — pdfp currently
emits `<!-- formula-review: ... -->` markers rather than `$$...$$` for
visual-only candidates, which is correct given the current quality bar.

The right answer is a sidecar service that accepts an image crop and returns
LaTeX. Options:

### Tool comparison

| Tool | Model size | CPU feasible | Inference (CPU) | Output | License | Notes |
|------|-----------|-------------|-----------------|--------|---------|-------|
| **RapidLaTeX-OCR** | ~100–150 MB | Yes | ~50–100 ms | LaTeX string | Apache/MIT | ONNX-first; drop-in for Rust via `ort` |
| **Texo** | 20 MB | Yes (fast) | 21–62 ms | LaTeX string | AGPL-3.0 | 7× faster than UniMERNet-T; ONNX exportable |
| **UniMERNet-Tiny** | 441 MB | Yes (slow) | ~100–300 ms | LaTeX string | Apache 2.0 | Best accuracy; exportable to ONNX |
| **Pix2Tex/LaTeX-OCR** | ~100–150 MB | Yes | ~150–400 ms | LaTeX string | MIT | Baseline; BLEU 0.88, token acc. 60% |
| **Nougat** | 1.2 GB | No | ~2–5 s | Markdown+math | Code: MIT; Weights: CC-BY-NC | Page-centric, not crop-friendly; non-commercial only |
| **GOT-OCR 2.0** | 580 MB | No | ~1–2 s | LaTeX/Markdown | Unclear | General OCR; formula is one mode |

**Recommendation** for `pdfp`:
- **Near-term sidecar** (Python subprocess): RapidLaTeX-OCR or UniMERNet-Tiny.
  Call from `src/hybrid/client.rs`-style HTTP client or subprocess.
- **Longer-term native** (Rust, no Python): export RapidLaTeX-OCR or Texo to
  ONNX → call via the `ort` Rust crate (Apache 2.0). No Python interpreter at
  runtime. `oar-ocr` crate wraps `ort` with OCR-specific utilities.
- **Avoid** Nougat (page-centric, non-commercial weights) and GOT-OCR
  (580 MB, GPU-oriented general OCR).

### Rust integration path

```
ONNX model (RapidLaTeX-OCR or Texo)
  ↓ load via ort crate
src/formula/ocr.rs — FormulaOcr::run(image_crop) → Option<String>
  ↓ on success
FormulaCandidate.latex = Some("...") + source updated
  ↓
MarkdownRenderer: emit $$ block instead of formula-review comment
```

The `ort` crate (github.com/pykeio/ort, Apache 2.0 / MIT) is the standard Rust
ONNX Runtime wrapper. It supports CPU and CUDA backends via feature flags, so
GPU can be enabled later without changing the API.

## Current State in pdfp

- Word-based detection: implemented, enabled in non-conservative mode
- Visual-band detection: implemented, opt-in via `--debug-formulas`
- Debug crops: written to `debug/formulas/` with JSON manifest
- Geometric LaTeX recovery: implemented (2026-05), uses baseline shifts for
  subscripts/superscripts, vertical stacking for fractions, operator limits.
  Quality is partial — correct on simple equations, garbled on multi-line or
  structured formulas (see benchmark below).
- Formula OCR sidecar: RapidLaTeXOCR integrated via subprocess, tested
  2026-05-28. Subprocess-per-crop is slow (~1.5-16s/crop, model reload per
  invocation). Native ONNX path exists behind `--features onnx-ocr`.
- False positive suppression: partial (table suppression, reference filters in
  word path); visual path still gets decorative-rule FPs

### Geometric vs RapidLaTeXOCR quality benchmark (2026-05-28)

Tested on `math-number-theory.pdf` formula crops. RapidLaTeXOCR installed via
`uv tool install rapid-latex-ocr --with requests`; inference on CPU.

| Candidate | Geometric LaTeX | RapidLaTeXOCR |
|---|---|---|
| Conv equation | `\label {eq:conv Y_{h \times W_{k_1\times`... (missing braces) | Structured `\begin{array}`, correct subscripts ✅ |
| Matmul equation | `\labe l eq:matmul }...\hat \times f} {W}` (garbled!) | `\hat{Y}_{h w b\times f}=\hat{X}_{h w b\times k_{1}k_{2}c}\hat{W}` ✅ clean |
| RUNTIME equation | `\labe l ^{{eq:r} ^{m}...` (garbled) | Recognisable, minor errors |
| CEIL_smooth | `^{m} ^{o} h c eilin g...` (broken) | Nested `\sum`, partially recovered |
| Lambda params | Duplicated text | Clean `\underline{\lambda}`, proper spacing ✅ |

**Conclusion**: RapidLaTeXOCR produces significantly better LaTeX than local
geometric recovery on real formulas. Geometric is useful as a fast fallback;
RapidLaTeXOCR should be preferred when quality matters.

**Production path**: native ONNX (cargo build --features onnx-ocr) keeps models
in memory. Estimated ~0.37s inference per crop (CPU), ~2-3s model load at
startup. 56 crops ≈ 21 seconds total. Subprocess path is 10-40× slower because
each invocation reloads ONNX models.

## What to Build Next

1. **Decorative-rule filter** — min band height in pixels / glyph height ratio;
   suppress when ratio < 0.5. Fixes DNV logo/header FPs.
2. **Furniture mask pre-pass** — detect running headers/footers and watermarks
   by page-association before formula detection runs.
3. **Table-first suppression** — run geometry table detector before formula
   detector; any candidate whose bbox overlaps a table region is suppressed.
4. **Formula OCR sidecar** — RapidLaTeX-OCR via Python sidecar first, then
   ONNX native via `ort` crate.

## Sources

- ScanSSD: arXiv:2003.08005 — formula detection with aspect ratio signals
- DocLayout-YOLO: arXiv:2410.12628 — separate formula/table/abandoned-text labels
- PDF-Extract-Kit formula detection: https://pdf-extract-kit.readthedocs.io/en/latest/algorithm/formula_detection.html
- UniMERNet: github.com/opendatalab/UniMERNet (Apache 2.0, arXiv:2404.15254)
- RapidLaTeX-OCR: github.com/RapidAI/RapidLaTeXOCR
- Texo: github.com/alephpi/Texo (arXiv:2602.17189, AGPL-3.0)
- Pix2Tex: github.com/lukas-blecher/LaTeX-OCR (MIT)
- ort Rust crate: github.com/pykeio/ort
- ICDAR 2021 formula detection winner: arXiv:2107.05534 (Weighted Box Fusion)
- Header/footer page-association: https://www.researchgate.net/publication/221253782
