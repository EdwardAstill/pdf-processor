---
title: "OCR Algorithms"
kind: "reference"
category: "wiki"
summary: "OCR approaches used in PDF conversion pipelines: general text OCR (Tesseract, PaddleOCR, TrOCR), formula-specific OCR, and the OCR-as-preprocessing strategy."
virtual_path: "wiki/algorithms/ocr"
entities: [Tesseract, PaddleOCR, TrOCR, UniMERNet, RapidLaTeX-OCR]
---

# OCR Algorithms

OCR in the PDF conversion context means one of two things:

- **Scan OCR** — the PDF has no usable text layer; OCR reconstructs it from rendered page images
- **Formula OCR** — the text layer exists but cannot express mathematical notation; OCR reconstructs LaTeX from equation crops

These are different problems requiring different models.

---

## Scan OCR

### Tesseract (rule-based + LSTM)

Classic open-source OCR engine. LSTM mode uses a sequence model trained on multilingual text. Separate equation-detector module uses symbol density to locate formula regions before OCR runs. Used by OCRmyPDF as the recognition backend.

**Best for**: Scan-heavy documents where the text layer is missing or garbage. Broad language support.

**Licence**: Apache 2.0

### PaddleOCR (Baidu)

Neural OCR stack: detection model finds text regions, recognition model reads them. 80+ languages. Production-quality. Default OCR engine in both PDF-Extract-Kit and MinerU.

**Best for**: High-accuracy OCR at scale.

**Licence**: Apache 2.0

### TrOCR (Microsoft)

Transformer OCR — ViT encoder + GPT-2-style decoder. General printed text; weaker than specialist models on formulas (TER 42% vs 36% for Pix2Tex on formula benchmarks). Not recommended for math-heavy documents.

**Licence**: Open source (Hugging Face Transformers)

### OCR-as-Preprocessing Pattern

The cleanest integration shape for a pipeline like `pdfp`:

1. Detect scan-heavy pages (see [page-triage.md](page-triage.md))
2. Run OCRmyPDF to produce a searchable derivative PDF
3. Feed derivative PDF back into the normal extraction pipeline
4. Record in metadata that OCR preprocessing was used

The original file stays untouched. The normal pipeline handles everything downstream.

---

## Formula OCR (Equation Image → LaTeX)

For choosing between formula OCR models see [tools/equation-ocr-models.md](../tools/equation-ocr-models.md). The short version:

| Tool | Size | CPU | Licence | Best for |
|------|------|-----|---------|---------|
| RapidLaTeX-OCR | ~100 MB | Fast | Apache/MIT | ONNX sidecar; Rust via `ort` |
| Texo | 20 MB | Very fast | AGPL-3.0 | Fastest CPU path |
| UniMERNet-Tiny | 441 MB | Slow | Apache 2.0 | Best accuracy |
| Pix2Tex | ~100 MB | Medium | MIT | Baseline |

The Rust integration path: export model to ONNX → call via `ort` crate → no Python at runtime.

For the full pipeline see [structures/equations.md](../structures/equations.md).
