# GitHub Candidate Notes

## Best Native-Port References

| Candidate | License signal | Why it matters | Fit for `pdfp` |
| --- | --- | --- | --- |
| `jsvine/pdfplumber` | MIT | Builds tables from detailed chars, words, lines, rectangles, inferred text edges, intersections, and cells. | High. Port algorithm ideas into `src/layout/table.rs`. |
| `camelot-dev/camelot` | MIT | Stream, Network, Lattice, and Hybrid give a practical taxonomy of table failures. | High for algorithm design; medium for direct integration because it is Python. |
| `tabulapdf/tabula-java` | MIT | Established table extraction behavior and UI expectations. | Medium. Useful reference, less direct than pdfplumber/Camelot. |
| `allenai/pdffigures2` | Apache-2.0 | Caption-driven figure proposal and scoring. | High for `src/figure/detect.rs` improvements. |

## Best Optional Sidecars

| Candidate | License signal | Why it matters | Fit for `pdfp` |
| --- | --- | --- | --- |
| `docling-project/docling` | MIT | Whole-document backend with table structure, OCR, formula enrichment, and JSON/Markdown outputs. | High. Already wired as `--hybrid docling`; live compatibility testing is the next step. |
| `conjuncts/gmft` | MIT | Lightweight deep table extraction with many export formats. | High for hard-table subprocess experiments. |
| `opendatalab/UniMERNet` | Apache-2.0 | Formula-image-to-LaTeX recognition. | High for crop-level formula sidecar. |
| `xavctn/img2table` | MIT | OpenCV table extraction for PDFs/images with pluggable OCR. | Medium-high for scanned/image table experiments. |
| `ocrmypdf/OCRmyPDF` | MPL-2.0 | Searchable-PDF preprocessing. | Already integrated; keep as first OCR pass. |

## Benchmark Or Opt-In Only

| Candidate | License signal | Why it matters | Fit for `pdfp` |
| --- | --- | --- | --- |
| `microsoft/table-transformer` | MIT | Strong pretrained table detection/structure models and PubTables-1M/GriTS tooling. | Benchmark or optional heavy sidecar, not a Rust-native import. |
| `opendatalab/PDF-Extract-Kit` | AGPL-3.0 | Modular layout, formula, OCR, and table model stack. | Benchmark/opt-in subprocess only. Do not import code into MIT repo. |
| `datalab-to/surya` | GPL-3.0 | OCR, layout, reading order, table recognition, LaTeX OCR. | Benchmark/opt-in subprocess only. Do not import code into MIT repo. |
| `datalab-to/marker` | GPL code, separate weight terms | Strong PDF-to-Markdown and table extraction path. | Benchmark oracle only unless licensing is accepted explicitly. |
| `opendatalab/MinerU` | verify before use | Very capable full-document parser with formulas, tables, OCR, cross-page table handling. | Benchmark first; verify license/runtime before integration. |

## Lower Immediate Priority

| Candidate | Reason |
| --- | --- |
| `grobid/grobid` | Excellent for scholarly metadata, references, and TEI full text, but not the fastest route for current table/formula/image issues. |
| `allenai/deepfigures-open` | Interesting figure ML work, but PDFFigures2 is easier to adapt to the current caption/snapshot implementation. |
| `PaddlePaddle/PaddleOCR` | Strong OCR stack, but OCRmyPDF is already the simpler preprocessing route. Use PaddleOCR indirectly through PDF-Extract-Kit/img2table unless scan OCR becomes the primary bottleneck. |
| `unstructured` | Useful broad document-processing ecosystem, but less focused than Docling/gmft/UniMERNet for this repo's immediate failures. |
