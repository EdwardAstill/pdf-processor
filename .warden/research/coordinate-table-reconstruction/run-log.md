# Coordinate Table Reconstruction Run Log

date: 2026-05-05

- Inspected `src/pdf/extractor.rs`, `src/document/types.rs`, `src/layout/table.rs`, `src/layout/classifier.rs`, and `src/render/markdown.rs`.
- Confirmed current `RawTextBlock` stores block bbox/text/font size only; character positions are used transiently in `collect_block_text` and then discarded.
- Confirmed `mupdf` crate exposes `TextLine::bounds`, `TextChar::origin`, `TextChar::size`, and `TextChar::quad`.
- Checked `pdfp convert --help`; no table-specific flags exist today.
- Ran `pdftotext -layout` on page 23 of the Crosby catalogue; the G-213 table remained visually aligned.
- Ran plain `pdftotext` on the same page; the table context degraded, confirming the need for layout coordinates.
- Reviewed MuPDF structured text options, MuPDF StructuredText API, mupdf-sys functions, pdfplumber table extraction docs, and Camelot Stream/Lattice docs.
