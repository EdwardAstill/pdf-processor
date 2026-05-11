---
title: "Layout Detection Models"
kind: "reference"
category: "wiki"
summary: "Model-based layout detection tools: DocLayout-YOLO, Table Transformer (TATR), Surya, LayoutLMv3."
virtual_path: "wiki/tools/layout-models"
entities: [DocLayout-YOLO, TATR, Surya, LayoutLMv3]
---

# Layout Detection Models

These models predict bounding boxes and class labels for document regions from page images. They complement (not replace) text extraction — they tell you *where* content is; extraction tells you *what* it contains.

---

## DocLayout-YOLO

- **Architecture**: YOLOv10 + Global-to-Local Controllable Receptive Module
- **Training data**: DocSynth-300K (synthetic, general documents)
- **Output categories** (10): Text, Title, Abandoned Text (decorative), Figure, Figure Caption, Table, Table Caption, Table Footnote, Isolated Formula, Formula Caption
- **Performance**: mAP 78.8% on tables; 85.5 FPS; CPU-capable
- **Key benefit**: "Isolated Formula" and "Table" as separate classes trained jointly — reduces formula/table confusion at the routing stage
- **Domain match**: General documents; engineering standards partially out-of-distribution
- **Licence**: AGPL-3.0
- **Links**: https://github.com/opendatalab/DocLayout-YOLO; arXiv:2410.12628

---

## Table Transformer (TATR, Microsoft)

- **Architecture**: DETR + ResNet-18; two separate models (table detection + table structure recognition)
- **Training data**: PubTables-1M (947K tables from academic papers)
- **CPU feasible**: No (GPU recommended)
- **Domain match**: Academic papers only — engineering standards strongly out-of-distribution
- **Key benefit**: Clean decomposition of detection vs structure recognition; useful architecture reference
- **Licence**: MIT
- **Links**: https://github.com/microsoft/table-transformer; arXiv:2110.00061

---

## Surya (Datalab / VikParuchuri)

- **Architecture**: Multi-task model for layout detection, text line detection, reading order, OCR
- **Used by**: Marker as its core model
- **CPU feasible**: Slow; GPU recommended
- **Licence**: CC-BY-NC-SA 4.0 (non-commercial)
- **Links**: https://github.com/VikParuchuri/surya

---

## LayoutLMv3 (Microsoft)

- **Architecture**: Multi-modal (text + image + layout) transformer; fine-tuned variant LayoutLMv3_ft used in PDF-Extract-Kit
- **CPU feasible**: No
- **Licence**: CC-BY-NC-SA 4.0
- **Links**: https://github.com/microsoft/unilm/tree/master/layoutlmv3
