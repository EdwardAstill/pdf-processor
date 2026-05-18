# PDF Tool Comparison Report

**Checked:** 2026-05-15

## Verdict

Against deterministic/local tools, `pdfp` is now close enough to deserve
head-to-head measurement. It is closest to PyMuPDF4LLM as a full
PDF-to-Markdown peer, with Poppler `pdftotext`, pdfminer.six, pdfplumber,
Camelot, Tabula, and OCRmyPDF/Tesseract forming the most useful deterministic
baselines.

It is still not broadly on par with the best document-understanding systems.
The ML/cloud systems remain likely stronger on broad OCR, semantic formula
recognition, complex layout, and picture understanding.

The right claim is narrower:

- Competitive: local-first conversion, deterministic debug artifacts, page
  operations, search/inspect/quality gates, and the current measured fixtures.
- Not yet competitive: broad OCR/layout semantics, exact formula OCR, complex
  cross-page tables, image descriptions, and hosted/API-grade robustness.

## Current `pdfp` Baseline

From local eval records:

- headings: `17/21`
- formulas: `13/13`
- table page recall/precision: `4/4`
- table-region precision: `4/5`
- meaningful figure retention: `6/6`
- figure-caption pairing: `3/3`
- vector-only acknowledgement: `1/1`
- decorative suppression: wired, but still needs a non-zero labeled fixture

## Tool Classes

| Class | Tools | What they are best for | `pdfp` posture |
| --- | --- | --- | --- |
| Fast local extraction | PyMuPDF4LLM, Poppler, pdfminer.six, `pdfp` | Born-digital text, Markdown, local workflows | `pdfp` can compete here if table/figure quality holds. |
| Deterministic table extraction | pdfplumber, Camelot, Tabula | Tables from lines, text alignment, and selectable text | `pdfp` should be benchmarked against these before claiming table strength. |
| Deterministic scan preprocessing | OCRmyPDF, Tesseract | Searchable text layers for scanned PDFs | `pdfp` should call these rather than replace them. |
| Local ML document parsing | Docling, MinerU, Marker | Layout semantics, tables, formulas, OCR/scans | `pdfp` should benchmark against these, not claim parity yet. |
| Cloud/API document intelligence | Adobe PDF Extract, Mathpix, LlamaParse, Unstructured | Hard OCR, STEM math, enterprise APIs, multimodal parsing | Use as external oracles where upload/credentials are acceptable. |

## Comparison Summary

| Tool | Deployment | Sourced capabilities | Main caution | Benchmark role |
| --- | --- | --- | --- | --- |
| `pdfp` | Local Rust CLI | Measured headings/formulas/tables/figures on repo fixtures; debug outputs; page ops/search/inspect. | Small fixture set; no broad external scores yet. | Baseline under test. |
| PyMuPDF4LLM | Local Python/MuPDF | Markdown API with OCR, images, headers/footers, page chunks, and table strategy options. | Less model-heavy semantic recovery. | Fast local baseline. |
| Poppler `pdftotext -layout` | Local CLI | Mature raw text/layout output plus bbox/bbox-layout/TSV modes. | Text only, no Markdown semantics. | Raw layout floor. |
| pdfminer.six | Local Python | Layout-analysis based text extraction and PDF internals debugging. | Text/layout library, not full Markdown. | Python text/layout baseline. |
| pdfplumber | Local Python | Visual debugging and table extraction from PDF object geometry. | Library/tooling layer, not full converter. | Table/debug comparator. |
| Camelot | Local Python | Stream/Lattice/Network/Hybrid table parsers. | Table-focused and input-sensitive. | Table comparator. |
| Tabula / tabula-java | Local Java/CLI | Table extraction to CSV/TSV/JSON from selectable-text PDFs. | Table-focused and not OCR. | Table comparator. |
| OCRmyPDF | Local CLI | Adds searchable OCR text layer to scanned PDFs. | Preprocessor, not Markdown extraction. | Scan comparator. |
| Docling | Local Python/service | MIT package with layout and table-structure models; OCR/table/formula options. | Heavier runtime; options increase processing time. | First local ML sidecar. |
| Marker | Local/hosted | Markdown/JSON/chunks/HTML; tables, forms, equations, inline math, images, OCR and LLM assist. | GPL/commercial licensing; vendor benchmarks. | Strong benchmark oracle, avoid code import. |
| MinerU | Local/API | Markdown/JSON; formulas to LaTeX; tables to HTML; scanned/garbled PDFs; 109-language OCR; complex layouts. | Large model/runtime footprint; custom license. | Strong hard-document comparator. |
| Mathpix | Cloud API | Async PDF processing to MMD/MD/HTML/LaTeX/DOCX/PPTX; advanced table fallback. | Paid credentials and upload. | STEM/formula quality oracle. |
| Adobe PDF Extract | Cloud API | JSON/Markdown; contextual text blocks, complex tables, figures, native/scanned PDFs. | Paid enterprise API and upload. | Enterprise structure oracle. |
| LlamaParse | Cloud API | Agentic OCR/parsing for 130+ formats; PDFs/scans to LLM-ready text. | Paid/API; model behavior may vary. | RAG parser oracle. |
| Unstructured | API/library ecosystem | Document partitioning, image/table block extraction, production on-demand jobs. | Broad ingestion tool; not specifically tuned to this repo's PDF metrics. | Ingestion pipeline comparator. |

## Recommendation

Start the comparison table now, but label it as **capability and benchmark
readiness**, not proof of parity. Next engineering step:

1. Run the deterministic `scripts/sidecar-audit.sh` backends:
   `pdftotext-layout`, `pymupdf4llm`, `pdfplumber`, `pdfminer`, `camelot`,
   `tabula`, and `ocrmypdf`.
2. Add wrapper commands for Marker, MinerU, Mathpix, Adobe, LlamaParse, and
   Unstructured.
3. Run the same fixtures through each available tool.
4. Add measured columns: headings, formulas, table pages/regions, figure
   retention/captioning, decorative suppression, OCR/scans, runtime, and
   operational constraints.
5. Only claim parity on a row-by-row basis after those measured runs.

## Sources

- Docling technical report: https://arxiv.org/abs/2408.09869
- Docling pipeline options: https://docling-project.github.io/docling/reference/pipeline_options/
- PyMuPDF4LLM API: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/api.html
- Poppler pdftotext manpage: https://manpages.debian.org/bookworm/poppler-utils/pdftotext.1.en.html
- pdfminer.six command line docs: https://pdfminersix.readthedocs.io/en/latest/tutorial/commandline.html
- pdfplumber README: https://github.com/jsvine/pdfplumber/blob/stable/README.md
- Camelot parser docs: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html
- tabula-java README: https://github.com/tabulapdf/tabula-java
- OCRmyPDF README: https://github.com/ocrmypdf/OCRmyPDF
- Marker README: https://github.com/datalab-to/marker
- Marker project metadata: https://github.com/datalab-to/marker/blob/master/pyproject.toml
- MinerU README: https://github.com/opendatalab/MinerU/blob/master/README.md
- Adobe PDF Extract API: https://developer.adobe.com/document-services/docs/overview/pdf-extract-api/
- Mathpix PDF API: https://docs.mathpix.com/reference/post-v3-pdf
- LlamaParse docs: https://developers.llamaindex.ai/llamaparse/
- Unstructured image/table extraction docs: https://docs.unstructured.io/api-reference/legacy-api/partition/extract-image-block-types
