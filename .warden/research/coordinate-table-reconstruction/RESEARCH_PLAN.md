# Coordinate Table Reconstruction Research Plan

date: 2026-05-05
status: complete

## Question

Can `pdfp` improve Markdown tables for born-digital catalogue PDFs by using local text positions instead of OCR or a Python sidecar?

## Subquestions

- What does the local extractor currently retain: blocks, lines, words, chars, or bboxes?
- Do primary libraries solve similar tables through text coordinates rather than OCR?
- Does MuPDF expose enough structured text data through the existing Rust dependency?
- What should `pdfp` do when a table is detected but confidence is too low for a real Markdown table?
- What tests should prove this works on the Crosby catalogue without making normal prose worse?

## Source Strategy

- Local code inspection for current extraction granularity and renderer behavior.
- Primary MuPDF docs and Rust binding docs for structured text and table-hunt viability.
- Primary pdfplumber and Camelot docs for proven coordinate-table algorithms.
- Local Crosby PDF checks with `pdftotext -layout` as a baseline for table region preservation.

## Deliverables

- `.warden/research/coordinate-table-reconstruction/REPORT.md`
- `.warden/research/coordinate-table-reconstruction/evidence.jsonl`
- `.warden/research/coordinate-table-reconstruction/contradictions.md`
- `.warden/plans/2026-05-05-coordinate-table-reconstruction-plan.md`
