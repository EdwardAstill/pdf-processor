# PDF Tool Comparison

Last checked: 2026-05-15.

## Verdict

`pdfp` should be compared first against deterministic/local PDF pipelines, not
against broad cloud document-AI services. Against that comparable set, it is now
credible but not proven best-in-class.

It is closest to **PyMuPDF4LLM** as a full PDF-to-Markdown peer. It should also
be measured against **Poppler `pdftotext -layout`** and **pdfminer.six** for raw
text/layout, **pdfplumber**, **Camelot**, and **Tabula** for deterministic table
extraction, and **OCRmyPDF/Tesseract** for scanned-page preprocessing.

The useful comparison is row-by-row:

- **Competitive now:** local conversion, deterministic debug artifacts, page
  operations, search/inspect workflows, and the current measured fixtures.
- **Improving:** headings, formulas, tables, figure snapshots, and vector-region
  acknowledgement.
- **Not yet best-in-class:** scanned PDFs, semantic formula OCR, complex
  multi-page tables, image descriptions, and broad mixed-layout documents.

## Current `pdfp` Baseline

Measured by `cargo run --quiet -- eval tests/eval_fixtures/` on this machine:

| Signal | Current result |
| --- | ---: |
| Heading accuracy | `17/21` |
| Formula recall | `13/13` |
| Table page recall | `4/4` |
| Table page precision | `4/4` |
| Table-region precision | `4/5` |
| Meaningful figure retention | `6/6` |
| Figure-caption pairing | `3/3` |
| Vector-only acknowledgement | `1/1` |
| Decorative suppression | metric wired, needs non-zero fixture |

These numbers are local fixture floors, not a public benchmark against other
tools.

## Measured Deterministic Run

Run on 2026-05-15 with an isolated Python environment at
`target/sidecar-tools/venv`.

```bash
PATH="$PWD/target/sidecar-tools/venv/bin:$PATH" \
PDFP_SIDECAR_OUT="$PWD/target/sidecar-audit-eval-fixtures" \
PDFP_SIDECAR_CORPUS="$PWD/test-corpus/eval" \
PDFP_SIDECAR_FIXTURES="engineering-calc-example.pdf engineering-report-example.pdf" \
PDFP_SIDECAR_BACKENDS="native pdftotext-layout pymupdf4llm pdfplumber pdfminer camelot tabula ocrmypdf" \
  bash scripts/sidecar-audit.sh
```

The same backend set was also run against the harder sidecar corpus at
`example/pdf`.

| Tool | Measured status | Result on current fixtures |
| --- | --- | --- |
| `pdfp` native | ok | Official eval remains the only fully scored row: headings `17/21`, formulas `13/13`, table page recall/precision `4/4`, table-region precision `4/5`. |
| PyMuPDF4LLM | ok | Strongest full Markdown peer. It emits Markdown headings and tables for the engineering fixtures. It does not provide the same formula-review/debug semantics, and produced empty Markdown for the scanned Chinese fixture. |
| Poppler `pdftotext -layout` | ok | Good raw layout text on selectable-text PDFs. It emits no Markdown headings, tables, figures, formula blocks, or debug artifacts. It produced effectively empty text for the scanned Chinese fixture. |
| pdfminer.six | ok | Similar raw text baseline to Poppler. It emits no Markdown structure, table objects, figures, or formula blocks. It produced effectively empty text for the scanned Chinese fixture. |
| pdfplumber | ok | Useful inspection/table baseline. It extracted report table JSON, but the calc fixture table JSON was empty in this run. It is not a full Markdown/formula/figure pipeline. |
| Camelot | ok | Strong table specialist on expected table pages, but noisy outside them: eval fixtures produced calc `1` lattice / `4` stream tables and report `3` lattice / `6` stream tables; the math sidecar document produced `29` stream table candidates. |
| Tabula / tabula-py | ok | Extracted tables, but over-detected more aggressively: eval fixtures produced calc `8` lattice / `0` stream tables and report `7` lattice / `3` stream tables; the math sidecar document produced `60` lattice / `7` stream table candidates. |
| OCRmyPDF | skipped | Python entrypoint installed, but OS dependencies are incomplete here: `qpdf` and `tesseract` are missing, so scan preprocessing is not yet measured. |

Current interpretation: `pdfp` is ahead of raw text tools for structured
Markdown and ahead of table-only tools as an end-to-end pipeline. PyMuPDF4LLM is
the closest full-pipeline deterministic peer and should be treated as the main
near-term comparator. Camelot and Tabula are still valuable as table oracles,
but their measured false positives mean they should inform candidate scoring,
not replace the native table arbitration.

## Deterministic Peer Set

These are the best comparable tools for the current local-first `pdfp` design.

| Tool | Best comparable role | Where it may beat `pdfp` | Where `pdfp` is stronger |
| --- | --- | --- | --- |
| PyMuPDF4LLM | Full deterministic PDF-to-Markdown/RAG export. | Mature MuPDF-backed Markdown, JSON/text outputs, table/image/vector references, page chunks, and optional OCR. | Integrated CLI, eval floors, debug crops/JSON, page ops/search/inspect, formula audit routing. |
| Poppler `pdftotext -layout` | Raw text/layout baseline. | Very mature, fast, simple physical-layout text output; available here now. | No Markdown structure, no table/figure/formula eval, no debug artifacts beyond raw text/bbox modes. |
| pdfminer.six | Python text/layout extraction baseline. | Detailed layout-analysis object model and command-line text extraction. | `pdfp` has higher-level Markdown, figures, tables, formulas, eval, and CLI workflows. |
| pdfplumber | Deterministic PDF inspection and table extraction. | Strong visual debugging and table extraction from PDF lines/word alignment. | Broader document pipeline and measured markdown-oriented quality gates. |
| Camelot | Deterministic table extraction. | Specialized Stream/Lattice/Network/Hybrid table parsers. | Full document conversion plus formulas/figures/eval; Camelot is table-focused. |
| Tabula / tabula-java | Deterministic table extraction. | Mature selectable-text table extraction and CLI CSV/TSV/JSON output. | Full document conversion and regression metrics; Tabula is table-focused. |
| OCRmyPDF + Tesseract | Deterministic scan preprocessing. | Adds searchable OCR text layers to scanned PDFs. | `pdfp` should call it; it is not a Markdown/table/figure pipeline by itself. |

## Deterministic Audit Backends

`scripts/sidecar-audit.sh` now includes these comparable deterministic backends:

```bash
PDFP_SIDECAR_BACKENDS="native pdftotext-layout pymupdf4llm pdfplumber pdfminer camelot tabula ocrmypdf" \
  bash scripts/sidecar-audit.sh
```

On this machine, `pdftotext` is installed; the Python/table/OCR tools currently
skip cleanly unless their modules or commands are installed. For the measured
run above, the Python tools were installed into an isolated `target/` venv
rather than added as project dependencies.

## Broader Capability Matrix

| Tool | Deployment | Best documented strengths | Main cautions | How to use in our benchmark |
| --- | --- | --- | --- | --- |
| `pdfp` | Local Rust CLI | Measured local fixtures, debug JSON/crops, page ops, search, inspect, OCR/hybrid hooks. | Fixture set is still small; no external parity score. | Baseline row; always run first. |
| PyMuPDF4LLM | Local Python/MuPDF | Markdown export with OCR, images, headers/footers, page chunks, and table strategy options. | External Python dependency; needs local install for direct measurement. | Closest deterministic full-pipeline peer. |
| Poppler `pdftotext -layout` | Local CLI | Plain-text extraction with physical-layout preservation, bbox, bbox-layout, and TSV modes. | Text only; no Markdown semantics. | Raw text/layout floor. |
| pdfminer.six | Local Python | Layout analysis and command-line/programmatic text extraction. | Text/layout library, not full Markdown pipeline. | Python layout baseline. |
| pdfplumber | Local Python | PDF object inspection, visual debugging, table extraction. | Library/tooling layer, not full Markdown pipeline. | Table/debug comparator. |
| Camelot | Local Python | Stream/Lattice/Network/Hybrid deterministic table extraction. | Table-focused and input-sensitive. | Table-specific comparator. |
| Tabula / tabula-java | Local Java/CLI | Selectable-text table extraction to CSV/TSV/JSON. | Table-focused and not OCR. | Table-specific comparator. |
| OCRmyPDF | Local CLI | Adds OCR text layer to scanned PDFs using Tesseract. | Preprocessor, not a Markdown extractor. | Scan preprocessing comparator. |
| Docling | Local Python/service | MIT package with layout analysis and table-structure models; OCR, table, and formula options. | Heavier runtime; enabling OCR/table/formulas increases processing time. | First local ML sidecar. |
| Marker | Local or hosted | Markdown/JSON/chunks/HTML; tables, forms, equations, inline math, images, OCR and optional LLM assist. | GPL/commercial licensing; vendor benchmark claims need local confirmation. | Benchmark oracle only; do not import code. |
| MinerU | Local/API | Markdown/JSON; formulas to LaTeX; tables to HTML; scanned/garbled PDFs; 109-language OCR; complex layouts. | Large model/runtime footprint; custom license. | Hard-document comparator. |
| Mathpix | Cloud API | STEM-oriented PDF OCR to Mathpix Markdown, Markdown, HTML, LaTeX, DOCX/PPTX; advanced table fallback. | Paid credentials and document upload. | Formula/STEM quality oracle. |
| Adobe PDF Extract API | Cloud API | Structured JSON and Markdown; contextual text blocks, complex tables, figures, native/scanned PDFs. | Paid enterprise API and document upload. | Enterprise structure oracle. |
| LlamaParse | Cloud API | Agentic OCR/parsing for 130+ formats; PDFs/scans to LLM-ready text. | Paid/API behavior; output can be prompt/config dependent. | RAG parser oracle. |
| Unstructured | API/library ecosystem | Partitioning, image/table block extraction, production on-demand jobs. | Broad ingestion system rather than a narrow PDF quality target. | Ingestion pipeline comparator. |

## Parity Criteria

Do not claim `pdfp` is on par with a tool unless the same fixture set has been
run through both tools and the comparison records:

- heading/section accuracy;
- formula detection and, separately, formula recognition quality;
- table page recall, page precision, region recall, and region precision;
- figure retention, figure-caption pairing, decorative suppression, and
  vector-region acknowledgement;
- OCR/scanned-page behavior;
- runtime and memory;
- dependency, license, credential, and upload constraints.

## Next Benchmark Work

The existing sidecar audit now provides the deterministic starting point:

```bash
bash scripts/sidecar-audit.sh
```

Next, install/run the deterministic peers first and inspect
`target/sidecar-audit/summary.md`. After that, add wrappers for Marker, MinerU,
Mathpix, Adobe PDF Extract, LlamaParse, and Unstructured. Until those wrappers
produce measured outputs under `target/sidecar-audit/`, the non-deterministic
rows remain a capability comparison, not a leaderboard.

## Sources

- Docling technical report: https://arxiv.org/abs/2408.09869
- Docling pipeline options: https://docling-project.github.io/docling/reference/pipeline_options/
- PyMuPDF4LLM API: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/api.html
- Poppler `pdftotext` manpage: https://manpages.debian.org/bookworm/poppler-utils/pdftotext.1.en.html
- pdfminer.six command line docs: https://pdfminersix.readthedocs.io/en/latest/tutorial/commandline.html
- pdfplumber README: https://github.com/jsvine/pdfplumber/blob/stable/README.md
- Camelot parser docs: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html
- Tabula Java README: https://github.com/tabulapdf/tabula-java
- OCRmyPDF README: https://github.com/ocrmypdf/OCRmyPDF
- Marker README: https://github.com/datalab-to/marker
- Marker project metadata: https://github.com/datalab-to/marker/blob/master/pyproject.toml
- MinerU README: https://github.com/opendatalab/MinerU/blob/master/README.md
- Adobe PDF Extract API: https://developer.adobe.com/document-services/docs/overview/pdf-extract-api/
- Mathpix PDF API: https://docs.mathpix.com/reference/post-v3-pdf
- LlamaParse docs: https://developers.llamaindex.ai/llamaparse/
- Unstructured image/table extraction docs: https://docs.unstructured.io/api-reference/legacy-api/partition/extract-image-block-types
