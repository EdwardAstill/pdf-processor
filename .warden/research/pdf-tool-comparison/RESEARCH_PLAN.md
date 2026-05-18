# PDF Tool Comparison Research Plan

**Created:** 2026-05-15
**Question:** Is `pdfp` on par with leading PDF extraction tools, and what
comparison table should the repo maintain?

## Scope

- Compare `pdfp` against leading current local/open-source and cloud/API PDF
  parsing tools.
- Separate sourced feature claims from measured benchmark results.
- Produce a repo-facing comparison table and a short parity verdict.

## Source Families

- Official project docs and GitHub READMEs.
- Official API docs for cloud services.
- Existing local benchmark records from `pdfp eval`.

## Tools

- `pdfp`
- PyMuPDF4LLM
- Poppler `pdftotext`
- pdfminer.six
- pdfplumber
- Camelot
- Tabula / tabula-java
- OCRmyPDF / Tesseract
- Docling
- Marker / Datalab
- MinerU
- Mathpix
- Adobe PDF Extract API
- LlamaParse
- Unstructured

## Output

- `.warden/research/pdf-tool-comparison/REPORT.md`
- `docs/TOOL_COMPARISON.md`
