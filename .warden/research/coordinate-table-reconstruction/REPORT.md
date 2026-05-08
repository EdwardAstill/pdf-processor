# Coordinate Table Reconstruction Research Report

date: 2026-05-05
status: complete

## Recommendation

Implement the next table improvement as a native coordinate-based table extractor inside `pdfp`. Do not start with OCR or a Python sidecar for the Crosby catalogue class of failures.

The source PDF has a usable text layer. `pdftotext -layout` preserves the G-213 Crosby table as readable aligned text, while the current `pdfp` Markdown path collapses the same table because it clusters only whole `RawTextBlock` bboxes. The fix is to keep word/character positions through extraction and reconstruct table regions from rows and column ranges.

## Local Findings

Current `pdfp` has the right raw ingredients but drops them too early:

- `src/pdf/extractor.rs` iterates MuPDF text blocks, lines, and chars.
- `TextChar::origin`, `TextChar::size`, and `TextChar::quad` are available through the local `mupdf` crate.
- `collect_block_text` uses character positions transiently to rebuild text rows, but `RawPage` stores only `RawTextBlock { bbox, text, font_size, font_name, ... }`.
- `src/layout/table.rs` detects tables by clustering whole block bboxes. That cannot recover tables where MuPDF returns a full product table as one or a few long text blocks.

The Crosby catalogue confirms this exact failure mode. `pdftotext -layout` on page 23 preserves columns like `Nominal Size`, `Working Load Limit`, `Stock No.`, `Weight Each`, dimensions, and replacement pin stock number. Plain text extraction and current Markdown lose the table shape.

## External Evidence

MuPDF supports richer structured text extraction. Its structured text options include `preserve-whitespace`, `accurate-bboxes`, `vectors`, `segment`, and `table-hunt` [MuPDF structured text options](https://mupdf.readthedocs.io/en/latest/reference/common/stext-options.html). Its StructuredText API exposes text blocks, lines, character callbacks with origin/font/size/quad, image blocks, vectors, and JSON output with positional data [MuPDF StructuredText](https://mupdf.readthedocs.io/en/latest/reference/javascript/types/StructuredText.html). The Rust `mupdf-sys` layer also exposes `fz_table_hunt` and `fz_table_hunt_within_bounds`, but those are low-level FFI functions rather than a safe table object API [mupdf-sys docs](https://docs.rs/mupdf-sys/latest/mupdf_sys/).

The algorithmic shape is well established. pdfplumber finds explicit or implied table lines, intersections, cells, and tables, and its `text` strategy infers imaginary lines from word alignment [pdfplumber docs](https://pypi.org/project/pdfplumber/). Camelot Stream does the same broad thing: group words into text rows, estimate table areas, infer column ranges, and assign words to cells by x/y coordinates [Camelot how it works](https://camelot-py.readthedocs.io/en/master/user/how-it-works.html). Camelot can export tables as Markdown, CSV, JSON, Excel, HTML, and SQLite [Camelot quickstart](https://camelot-py.readthedocs.io/en/master/user/quickstart.html).

## Design Implication

The best native design is:

1. Preserve page-level word geometry in `RawPage`.
2. Build table regions from word rows, not from already-merged blocks.
3. Infer column bands from recurring x alignments and numeric right edges.
4. Assign words into row/column cells.
5. Score confidence before rendering.
6. Emit real Markdown tables only when confidence is good.
7. For low-confidence table-like regions, emit fenced fixed-width layout text rather than collapsed paragraphs.

This keeps `pdfp` as a one-command local binary while matching the core strategy used by mature table extractors.

## Risks

- Multi-row headers need careful merging. Crosby tables often use stacked headers like `Working / Load / Limit / (t)`.
- Numeric columns can be right-aligned while text columns are left-aligned. Column inference must support both.
- Tables wider than 10 columns are common in engineering catalogues, so the current `x_clusters.len() > 10` rejection is too strict.
- A Markdown table is not always the most faithful representation. Confidence-based fallback to fenced layout text is required.

## Conclusion

This is viable and likely the right next change. The implementation should not depend on OCR for born-digital Crosby pages. OCR/table-recognition can remain a later fallback for scanned or broken-text PDFs, but the immediate quality win is preserving and using MuPDF character/word coordinates.

## Sources

- MuPDF structured text options: https://mupdf.readthedocs.io/en/latest/reference/common/stext-options.html
- MuPDF StructuredText API: https://mupdf.readthedocs.io/en/latest/reference/javascript/types/StructuredText.html
- Rust `mupdf-sys` docs: https://docs.rs/mupdf-sys/latest/mupdf_sys/
- pdfplumber docs: https://pypi.org/project/pdfplumber/
- Camelot algorithm docs: https://camelot-py.readthedocs.io/en/master/user/how-it-works.html
- Camelot quickstart/export docs: https://camelot-py.readthedocs.io/en/master/user/quickstart.html
