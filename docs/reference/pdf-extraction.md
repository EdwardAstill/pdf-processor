# PDF Extraction (`src/pdf/`)

Low-level PDF reading via MuPDF. Extracts text, word positions, images, and optional font metadata from a PDF file.

## Source files

| File | Purpose |
|---|---|
| `mod.rs` | Module root, re-exports `PdfExtractor` |
| `extractor.rs` | `PdfExtractor` — opens PDF, iterates pages, extracts blocks, words, images |
| `metadata.rs` | Optional `pdfium-metadata` font name and struct-tree reader (feature-gated) |
| `text_cleanup.rs` | Text normalisation: strip soft hyphens, combining overlines, control chars; insert spacing |

## Key types

| Type | Definition | Purpose |
|---|---|---|
| `PdfExtractor` | struct | Main entry point. Created from a PDF path, iterates pages |
| `RawPage` | struct (in `pipeline`/`document`) | Output of extraction: blocks, words (positioned), image refs, page geometry |
| `RawTextBlock` | struct | A single text block from MuPDF: `bbox`, `text`, `font_size`, `font_name`, `block_id`, `reading_order` |
| `RawWord` | struct | A positioned word: `bbox`, `text`, `font_size`, `baseline_y`, `block_id`, `line_id` |
| `ImageRef` | struct | An embedded image: `page_num`, `bbox`, `image_index`, `bytes`, `format` |

## Key functions (`PdfExtractor`)

| Function | Description |
|---|---|
| `PdfExtractor::open(path)` | Open a PDF, return `PdfExtractor` or error. Prepares the MuPDF document |
| `extractor.extract_page(page_num)` | Extract one page's blocks, words, and images. Returns `RawPage` |
| `extractor.page_count()` | Number of pages in the PDF |
| `extractor.load_page(page_num)` | Low-level MuPDF page handle for rule-line extraction (used by table detector) |

### Text cleanup (`text_cleanup.rs`)

| Function | Description |
|---|---|
| `cleanup_extracted_text(text)` | Strip soft hyphens, combining overline U+0305, control characters, zero-width spaces, bidi marks |
| `insert_missing_space_after_numbered_label(text)` | Insert space after "1.Introduction" → "1. Introduction" |

### Metadata (`metadata.rs`, feature `pdfium-metadata`)

| Type | Purpose |
|---|---|
| `PageMetadata` | Per-page font metadata and struct-tree roles from pdfium |
| `load_page_metadata(path, page_num)` | Load font info for one page (optional, returns `None` when feature disabled) |

## CLI flags

| Flag | Effect |
|---|---|
| (none — always runs) | Extraction is the first step of every conversion. No CLI flags disable it. |

## Dependencies

- **mupdf 0.6** — C library compiled from source by `cc`. Provides `mupdf::Document`, page rendering, text page extraction.
- **lopdf 0.40** — Low-level PDF object manipulation for metadata read/write. Used by `processor/metadata.rs`, not by the extractor.
- **pdfium-render 0.9** (optional) — Second-opinion font metadata reader. Gated behind `--features pdfium-metadata`. Dynamically loads `libpdfium.so` at runtime.

## Cross-references

- [`docs/reference/pdf-format.md`](reference/pdf-format.md) — PDF file format primer (content streams, fonts, encodings)
- `wiki/topics/text-extraction.md` — text extraction details and encoding gotchas
- `wiki/tools/pdf-engines.md` — MuPDF vs other PDF engines
- [layout-analysis.md](layout-analysis.md) — what happens to the extracted blocks and words
- [pipeline.md](pipeline.md) — how extraction feeds into the pipeline
