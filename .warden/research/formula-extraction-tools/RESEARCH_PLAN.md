# Research Plan - Formula Extraction Tools

## Question

What proven external tools can `pdfp` leverage to improve extraction of formulas from engineering standards PDFs, especially DNV-ST-N001-style documents where normal PDF text extraction leaves formula gaps?

## Subquestions

- Which tools can detect formula regions in PDFs?
- Which tools can recognize formula images as LaTeX or structured math?
- Which tools can export whole-document Markdown/JSON with formula content?
- Which options are local/offline versus hosted/API?
- Which integration shape is most realistic for `pdfp`: sidecar binary, Python service, Docling hybrid, or manual audit workflow?

## Source Families

- Official project documentation and model docs
- GitHub repositories for install/runtime shape
- Papers/technical reports for maturity claims
- Existing `pdfp` docs for current integration constraints

## Deliverables

- `evidence.jsonl`
- `contradictions.md`
- `REPORT.md`
- Inline recommendation to the user
