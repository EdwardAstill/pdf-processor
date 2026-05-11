---
title: "Equation OCR Models"
kind: "reference"
category: "wiki"
summary: "Models that convert equation image crops to LaTeX: UniMERNet, RapidLaTeX-OCR, Texo, Pix2Tex. Includes comparison table and Rust integration path via ort + ONNX."
virtual_path: "wiki/tools/equation-ocr-models"
entities: [UniMERNet, RapidLaTeX-OCR, Texo, Pix2Tex, Nougat, ort-crate, ONNX]
---

# Equation OCR Models

These models take an image crop of a mathematical equation and return a LaTeX string. They are the right tool for the formula OCR sidecar stage — after a formula region has been detected and cropped.

---

## Comparison Table

| Tool | Model size | CPU feasible | Inference (CPU) | Output | Licence | Notes |
|------|-----------|-------------|-----------------|--------|---------|-------|
| **RapidLaTeX-OCR** | ~100–150 MB | Yes | ~50–100 ms | LaTeX string | Apache/MIT | ONNX-first; drop-in for Rust via `ort` |
| **Texo** | 20 MB | Yes (fast) | 21–62 ms | LaTeX string | AGPL-3.0 | 7× faster than UniMERNet-T; ONNX-exportable |
| **UniMERNet-Tiny** | 441 MB | Yes (slow) | ~100–300 ms | LaTeX string | Apache 2.0 | Best accuracy; ONNX-exportable |
| **Pix2Tex/LaTeX-OCR** | ~100–150 MB | Yes | ~150–400 ms | LaTeX string | MIT | Baseline; BLEU 0.88, token acc. 60% |
| **Nougat** | 1.2 GB | No | ~2–5 s | Markdown+math | Code: MIT; Weights: CC-BY-NC | Page-centric, not crop-friendly; non-commercial |

**Recommended for `pdfp`**:
- Near-term sidecar (Python subprocess): RapidLaTeX-OCR or UniMERNet-Tiny
- Longer-term native Rust: export to ONNX → call via `ort` crate

**Avoid**: Nougat (page-centric, CC-BY-NC weights), GOT-OCR (580 MB, general-purpose OCR).

---

## UniMERNet (OpenDataLab)

- **Architecture**: Three model sizes (base 1.3 GB, small 773 MB, tiny 441 MB)
- **Training data**: UniMER-1M (1M equation images)
- **Accuracy**: Best in class; outperforms Pix2Tex on complex notation and multi-line equations
- **ONNX**: Exportable via standard PyTorch export
- **Licence**: Apache 2.0
- **Links**: https://github.com/opendatalab/UniMERNet; arXiv:2404.15254

---

## RapidLaTeX-OCR (RapidAI)

- **Architecture**: Pix2Tex converted to ONNX + ONNXRuntime
- **Design**: ONNX-first — removes Python interpreter overhead at inference time
- **CPU**: Optimised for CPU via ONNXRuntime; suitable as a low-latency sidecar
- **Rust integration**: Load ONNX model via `ort` crate; no Python at runtime
- **Links**: https://github.com/RapidAI/RapidLaTeXOCR

---

## Texo

- **Architecture**: 20M parameters; 80% smaller than UniMERNet-Tiny; ONNX-exportable; also available via Transformers.js for browser use
- **Speed**: 21–62 ms per crop on CPU (7× faster than UniMERNet-T)
- **Accuracy**: BLEU 0.9014 on UniMER-Test; competitive with much larger models
- **Licence**: AGPL-3.0 — verify terms for integration
- **Links**: https://github.com/alephpi/Texo; arXiv:2602.17189

---

## Pix2Tex / LaTeX-OCR (Lukas Blecher)

- **Architecture**: ViT + ResNet encoder, Transformer decoder; ~25M parameters
- **Accuracy**: BLEU 0.88, token accuracy ~60%
- **CPU**: Feasible; slower than RapidLaTeX-OCR
- **Best for**: Baseline comparisons; simple equations
- **Licence**: MIT
- **Links**: https://github.com/lukas-blecher/LaTeX-OCR

---

## Rust Integration Path

All models above can be called from Rust via ONNX export + the `ort` crate:

```
ONNX model (RapidLaTeX-OCR or Texo)
  ↓ loaded via ort crate (github.com/pykeio/ort, MIT/Apache 2.0)
src/formula/ocr.rs — FormulaOcr::run(image_crop) → Option<String>
  ↓ on success
FormulaCandidate.latex = Some("...") + source updated
  ↓
MarkdownRenderer: emit $$ block instead of formula-review comment
```

The `ort` crate supports CPU and CUDA backends via feature flags. GPU can be enabled later without changing the API.

The `oar-ocr` crate (https://crates.io/crates/oar-ocr) wraps `ort` with OCR-specific utilities and supports formula models directly.
