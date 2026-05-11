---
title: "Table Structure Recognition Models"
kind: "reference"
category: "wiki"
summary: "Model-based table structure recognition: StructEqTable, CascadeTabNet, LGPMA, TableStructureFormer."
virtual_path: "wiki/tools/table-models"
entities: [StructEqTable, CascadeTabNet, LGPMA, TableStructureFormer]
---

# Table Structure Recognition Models

These models take a table region image and recover row, column, and cell structure. Distinct from layout detection models (which find where the table is) and extraction libraries (which work from the text layer).

All require GPU for practical use. No pre-trained model covers engineering standards (ISO/DNV/IEC) — training data is academic papers or financial documents. Custom fine-tuning is required for high accuracy on standards-style tables.

---

## StructEqTable (InternScience)

- **What it does**: Table image → LaTeX/HTML/Markdown via a model trained on DocGenome/TableX (2M+ arXiv table crops, 156 disciplines)
- **Key strength**: Handles symbol-heavy cells (mathematical content in cells) because the training data includes arXiv papers with math
- **CPU feasible**: No (~1 s/image on A100 GPU)
- **Domain match**: arXiv scientific papers; may transfer to engineering standards better than PubTables-1M-trained models given the math content
- **Licence**: MIT
- **Links**: https://github.com/InternScience/StructEqTable-Deploy; arXiv:2406.11633

---

## CascadeTabNet

- **Architecture**: Cascade Mask R-CNN + HRNet backbone; end-to-end detection + structure recognition
- **Benchmarks**: 3rd rank ICDAR 2019 detection; best on ICDAR 2013/2019 structure recognition
- **Key strength**: Handles merged cells
- **CPU feasible**: No
- **Licence**: MIT
- **Links**: https://github.com/DevashishPrasad/CascadeTabNet; arXiv:2004.12629

---

## LGPMA (Local + Global Pyramid Mask Alignment)

- **Architecture**: Mask R-CNN variant with local (text region) + global (cell relationship) feature paths, combined with pyramid re-scoring
- **Key strength**: Explicitly handles merged/spanning cells; ICDAR 2021 Best Industry Paper
- **CPU feasible**: No
- **Availability**: Published paper; check repo for code/weights
- **Links**: arXiv:2105.06224

---

## TableStructureFormer

- **Architecture**: Masked-attention mask transformer with SAResNet encoder; annotates rows, columns, and cells with row-column spans
- **Key strength**: Span annotation — designed for tables with complex merged headers
- **CPU feasible**: No
- **Source**: CVPR 2022 (Nassar et al., IBM)
- **Links**: arXiv (check paper for code availability)
