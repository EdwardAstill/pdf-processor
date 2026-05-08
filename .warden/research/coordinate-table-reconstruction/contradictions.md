# Coordinate Table Reconstruction Contradictions

date: 2026-05-05
status: complete

## Findings

No direct contradiction was found. The evidence points to a tiered approach:

- `pdftotext -layout` is better than current `pdfp` Markdown for Crosby tables, but it is fixed-width text, not structured Markdown.
- pdfplumber and Camelot show that coordinate-derived Markdown tables are viable for born-digital PDFs when word/character positions are available.
- MuPDF exposes enough structured text primitives and even a table-hunt option, but the current local `pdfp` data model drops the finer positions before table reconstruction.

## Tensions

- **Native implementation vs sidecar quality:** Camelot/pdfplumber are mature, but adding Python dependencies conflicts with the single-binary/local-tool direction. The native MuPDF path should be tried first.
- **Real Markdown tables vs correctness:** Some complex multi-row headers may be better represented as fenced layout text until header merging is reliable. The implementation should use confidence scoring rather than forcing every detected region into a table.
- **MuPDF table-hunt vs custom algorithm:** MuPDF exposes table-hunt at the C/sys layer, but the safe Rust wrapper does not expose high-level table results. Use structured text flags and word geometry first; only call `mupdf-sys` directly if the safe wrapper cannot supply enough geometry.
