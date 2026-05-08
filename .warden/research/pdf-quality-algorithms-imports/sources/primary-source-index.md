# Primary Source Index

Checked on 2026-05-08.

## Local Sources

- `src/layout/table.rs` - current coordinate table detector and confidence fallback.
- `src/formula/detect.rs` - current formula candidate detector and crop metadata.
- `src/figure/detect.rs` - current caption/image-based figure candidate detector.
- `src/ocr/mod.rs` - OCRmyPDF preparation, caching, command resolution, and provenance.
- `src/hybrid/client.rs` - current `docling-serve` multipart client and options.
- `docs/QUALITY_LOOP.md` - repeatable research/change/test/observe loop.
- `.warden/research/coordinate-table-reconstruction/REPORT.md`
- `.warden/research/formula-extraction-tools/REPORT.md`
- `.warden/research/figure-snapshot-extraction/REPORT.md`
- `.warden/research/local-ocr-and-quality-plan/REPORT.md`

## External Sources

- pdfplumber: https://github.com/jsvine/pdfplumber
- Camelot docs: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html
- Camelot source: https://github.com/camelot-dev/camelot
- Tabula Java: https://github.com/tabulapdf/tabula-java
- Table Transformer: https://github.com/microsoft/table-transformer
- GriTS paper: https://arxiv.org/abs/2203.12555
- gmft: https://github.com/conjuncts/gmft
- Docling: https://github.com/docling-project/docling
- Docling pipeline options: https://docling-project.github.io/docling/reference/pipeline_options/
- Docling enrichments: https://docling-project.github.io/docling/usage/enrichments/
- UniMERNet paper: https://arxiv.org/abs/2404.15254
- UniMERNet repo: https://github.com/opendatalab/UniMERNet
- PDF-Extract-Kit: https://github.com/opendatalab/PDF-Extract-Kit
- Surya: https://github.com/datalab-to/surya
- Marker: https://github.com/datalab-to/marker
- MinerU: https://github.com/opendatalab/MinerU
- OCRmyPDF docs: https://ocrmypdf.readthedocs.io/en/latest/
- img2table: https://github.com/xavctn/img2table
- PDFFigures2: https://github.com/allenai/pdffigures2
- GROBID: https://github.com/grobidOrg/grobid
- PyMuPDF4LLM: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/
- PaddleOCR: https://github.com/PaddlePaddle/PaddleOCR
- Unstructured docs: https://unstructured-io.github.io/unstructured/
