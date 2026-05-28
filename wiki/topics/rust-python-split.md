---
title: "Rust and Python Split"
kind: "knowledge"
category: "wiki"
summary: "Why pdfp uses both Rust and Python, how they are currently split, what each language is best for in PDF processing, and where the split should evolve."
entities: [Rust, Python, MuPDF, Docling, ONNX, OCRmyPDF, hybrid]
---

# Rust and Python Split

`pdfp` is a Rust project that calls Python for specific hard problems. This page explains why, how the split works today, and where it should move.

---

## Why Both Languages

PDF processing sits at the intersection of two very different domains:

| Domain | Best language | Why |
|---|---|---|
| File format parsing, text extraction, coordinate geometry | **Rust** | Fast, deterministic, zero-overhead FFI with MuPDF. Single binary distribution. |
| ML models for layout, tables, formulas, OCR | **Python** | Every model in the PDF ecosystem runs in PyTorch/ONNX with Python tooling. |

A pure-Rust project would need to reimplement or port every ML model. A pure-Python project would be slow on large documents and hard to distribute. The split is pragmatic: Rust for the deterministic pipeline, Python for model-based recovery.

---

## Current Split

### Rust (the engine)

Everything in the core pipeline runs in Rust:

```
Open PDF → Extract text blocks + images + coordinates → Recover layout (XY-Cut++)
→ Classify blocks (headings, paragraphs, lists, tables, captions)
→ Detect formulas (heuristic) → Detect figures → Render Markdown
```

Plus all non-conversion operations: page manipulation, metadata, inspection, search, evaluation.

**Key Rust dependencies:**
- `mupdf` 0.6 — PDF rendering and text extraction
- `lopdf` 0.40 — PDF object graph read/write
- `pdfium-render` (optional) — font metadata and struct-tree access

### Python (the recovery path)

Python is called in two places, both as external processes:

1. **`--hybrid docling`** — HTTP call to a Docling server. Routes entire pages judged "hard" by the triage logic (formula-heavy, scan-heavy, layout-complex). Docling returns Markdown that replaces the local pipeline's output for that page.

2. **`--ocr`** — Subprocess call to `ocrmypdf`. Creates a searchable derivative PDF from a scan, then feeds it back through the Rust pipeline.

### ONNX bridge (Rust → model, no Python)

The `onnx-ocr` feature gate runs ONNX formula OCR models directly in Rust via the `ort` crate. This is the ideal path — run models without a Python process. Currently experimental, model availability is limited.

---

## Assessment: Current Split

### What's right

- **Rust owns the file format.** Parsing PDF objects, extracting text blocks, manipulating pages — Rust is the right choice. Fast, no GC, single binary.
- **Layout analysis in Rust.** XY-Cut++, block classification, heading detection — these are geometric algorithms that benefit from Rust's speed and zero-cost abstractions.
- **Python for hard ML.** Formula OCR, complex table structure recognition, layout model inference — these need PyTorch/ONNX models that only exist in Python.
- **ONNX as the bridge.** Running models via `ort` in Rust avoids the process boundary. The feature gate exists and works.

### What's suboptimal

**1. Hybrid routing is too coarse.** The current `--hybrid docling` routes *entire pages*. A page with one hard table loses all its clean text blocks, headings, and figures — Rust could handle those fine. Should add a per-region mode: send only the formula crop or table region to Python, keep the rest local.

**2. Evaluation framework is Rust-only.** The Rust eval framework measures formula precision/recall and block accuracy. But the entire PDF evaluation ecosystem (OmniDocBench, RD-TableBench, GriTS, T-LAG) is Python. `pdfp` can't run standard benchmarks without a Python companion.

### What's already correct

**Furniture suppression is active.** `src/layout/furniture.rs` → `detect_furniture_bboxes()` runs in the pipeline, excluding headers/footers/watermarks from formula detection and suppressing text blocks inside furniture zones.

**Structure tree is wired (feature-gated).** `pdfium-metadata` feature → `FPDF_StructTree_GetForPage()` → authoritative heading detection in classifier. Tests cover struct-tree role mapping.

**ONNX is the preferred formula OCR path.** `onnx-ocr` feature → `OnnxFormulaSidecar` via `ort` crate. The formula sidecar architecture (trait with command and ONNX implementations) already prefers ONNX when available.

The old split-specific plan has been removed. Live follow-up work belongs in the current handoff rather than a separate plan file.

---

## Optimal Split (recommended)

### Rust: deterministic extraction core

```
Open PDF → Text + coordinate extraction → Layout recovery → Block classification
→ Furniture suppression → Table/heuristic detection → Formula/heuristic detection
→ Figure extraction → Internal block model → Markdown rendering
```

Plus: page operations, metadata, signatures, form filling (deterministic), eval framework.

### Python: model-based recovery sidecar

Called per-region, not per-page:

```
Formula crop → LaTeX reconstruction (UniMERNet, RapidLaTeX-OCR)
Table region → Structure recognition (StructEqTable)
Complex layout → Layout model (DocLayout-YOLO)
Scan page → OCR (OCRmyPDF derivative, then back to Rust)
Evaluation → Standard benchmarks (OmniDocBench, GriTS, T-LAG)
```

### ONNX: direct model inference (no Python)

For models that export to ONNX and are small enough to bundle:

```
Formula crop → ONNX model → LaTeX (via ort crate)
Layout page → ONNX model → regions → back to Rust classification
```

ONNX should be the first choice. Python is the fallback when a model isn't available in ONNX or is too large to bundle.

### Routing policy

| Signal | Action |
|---|---|
| Formula candidate, high confidence | ONNX formula OCR (Rust) |
| Formula candidate, ONNX fails | Python formula OCR |
| Table region, bordered | Rust heuristic |
| Table region, borderless/symbol-heavy | Python StructEqTable or ONNX table model |
| Scan-heavy page | Python OCRmyPDF → derivative PDF → Rust extraction |
| Layout-complex page (magazine, brochure) | Python DocLayout-YOLO → region boxes → Rust classification |
| Tagged PDF with struct-tree | Rust (no Python needed — struct-tree is authoritative) |
| Everything else | Rust (deterministic pipeline) |

---

## Related Pages

- [Pipeline Overview](pipeline-overview.md) — pipeline stages
- [Formula Detection](../algorithms/formula-detection.md) — hybrid formula OCR path
- [Scans and OCR](scans-and-ocr.md) — OCRmyPDF integration
- [Evaluation and Benchmarks](evaluation-and-benchmarks.md) — eval framework
- [ARCHITECTURE.md](../../ARCHITECTURE.md) — module map and design decisions
