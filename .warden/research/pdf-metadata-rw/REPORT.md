# PDF Metadata Read/Write Research

Date: 2026-05-19
Status: standard-depth research for a planned `pdfp metadata` feature

## Question

Can `pdfp` grow from basic metadata inspection to explicit PDF metadata read,
write, and documentation support without weakening existing PDF safety
contracts?

## Current Repo State

- `pdfp inspect` reads only `title`, `author`, and `subject` from the PDF Info
  dictionary, plus page count and page scan signals.
- The internal `DocumentMetadata` type currently contains only `title`,
  `author`, `subject`, and `page_count`.
- `README.md` and `docs/CLI.md` explicitly say PDF-producing page operations do
  not yet guarantee document-level metadata preservation.
- Existing PDF writers use MuPDF:
  - `src/processor/pages.rs` uses `mupdf::pdf::PdfDocument` and full saves for
    extract/delete/reorder/merge.
  - `src/processor/impose.rs` and `src/processor/resize.rs` render into fresh
    PDFs with `DocumentWriter`, so metadata preservation there is naturally a
    separate follow-up.

Relevant local code:

- `src/processor/inspect.rs`
- `src/pdf/extractor.rs`
- `src/document/types.rs`
- `src/processor/pages.rs`
- `src/processor/impose.rs`
- `src/processor/resize.rs`

## PDF Metadata Surface

The practical user-visible metadata surface is:

- Info dictionary fields:
  - `Title`
  - `Author`
  - `Subject`
  - `Keywords`
  - `Creator`
  - `Producer`
  - `CreationDate`
  - `ModDate`
- Catalog-level XML metadata stream:
  - `/Root/Metadata`, normally XMP

ExifTool's PDF tag reference confirms the standard Info fields and notes that
PDFs can also carry XMP metadata under the document catalog's `Metadata` entry.
It also highlights an important safety point: incremental metadata edits are
fast and reversible, but old values may remain recoverable until a full rewrite
or linearization pass removes them.

Source:

- https://exiftool.org/TagNames/PDF.html

## Library Findings

### MuPDF

The Rust `mupdf` crate already exposes metadata reads through
`Document::metadata(MetadataName)`. Its `MetadataName` enum includes:

- `Author`
- `Title`
- `Producer`
- `Creator`
- `CreationDate`
- `ModDate`
- `Subject`
- `Keywords`

The bundled MuPDF C source also has `fz_set_metadata()` and PDF-specific
`pdf_set_metadata()` support for those `info:*` keys. The current Rust wrapper
does not expose a safe `set_metadata()` method, but it does expose lower-level
`PdfDocument::trailer()`, `PdfObject::dict_put()`, `PdfDocument::new_dict()`,
and `PdfDocument::save_with_options()`.

Implication: MuPDF can support Info writes, but using it directly in this repo
requires either:

- low-level trailer/Info dictionary mutation with careful PDF text-string
  encoding, or
- adding a wrapper-level metadata setter around MuPDF internals, which is more
  invasive because the crate's raw document pointer is private.

Local source inspected:

- `~/.cargo/registry/src/.../mupdf-0.6.0/src/document.rs`
- `~/.cargo/registry/src/.../mupdf-0.6.0/src/pdf/document.rs`
- `~/.cargo/registry/src/.../mupdf-0.6.0/src/pdf/object.rs`
- `~/.cargo/registry/src/.../mupdf-sys-0.6.0/mupdf/source/pdf/pdf-xref.c`
- `~/.cargo/registry/src/.../mupdf-sys-0.6.0/mupdf/include/mupdf/fitz/document.h`

### lopdf

`lopdf` 0.40.0 is a pure-Rust PDF object manipulation library.

Useful APIs and properties:

- `Document::load()` loads existing PDFs.
- `Document::save()` writes PDFs.
- `Document::load_metadata()` reads title/page-count-style metadata without
  loading full document objects.
- `PdfMetadata` includes `title`, `author`, `subject`, `keywords`, `creator`,
  `producer`, `creation_date`, `modification_date`, `page_count`, and
  `version`.
- `text_string()` writes PDF text strings as PDFDocEncoding for ASCII or
  UTF-16BE otherwise.
- `decode_text_string()` handles PDFDocEncoding, UTF-16BE, and UTF-8 BOMs.
- Rust requirement is 1.85; local toolchain is `rustc 1.94.0`.

Sources:

- https://docs.rs/lopdf/latest/lopdf/
- https://docs.rs/lopdf/latest/i686-pc-windows-msvc/lopdf/struct.Document.html
- https://docs.rs/lopdf/latest/lopdf/struct.PdfMetadata.html

Implication: `lopdf` is a strong fit for the first metadata feature because it
can mutate PDF dictionaries directly, has proper text-string helpers, and avoids
unsafe bindings. It rewrites the whole file rather than incremental-appending,
which is acceptable for a safe default and avoids leaving old metadata
recoverable in the file body.

### qpdf / ExifTool

ExifTool already supports native PDF and XMP metadata writes, but its default
incremental-update model leaves old values recoverable unless a qpdf cleanup is
run afterwards. Depending on external CLIs would also weaken `pdfp`'s
local-first, single-binary model.

Recommendation: do not shell out to ExifTool/qpdf for the core feature. Use
them only as optional parity/manual validation tools in tests or documentation.

## Recommended Direction

Build a first-class `pdfp metadata` command family using `lopdf` for Info
dictionary reads/writes and MuPDF only as an optional validation/compatibility
reader.

MVP scope:

- Read full Info metadata:
  - `pdfp metadata show input.pdf`
  - `pdfp metadata show input.pdf --json`
- Write full Info metadata to a new output PDF:
  - `pdfp metadata set input.pdf -o output.pdf --title ... --author ...`
- Clear selected Info fields:
  - `pdfp metadata clear input.pdf -o output.pdf --fields title,author`
- Preserve existing page contents through metadata-only rewrite.
- Refuse in-place writes, matching existing page operation safety.
- Detect and warn when XMP metadata is present but not updated.
- Detect and warn when signatures/permissions are present and metadata writes
  may invalidate or conflict with them.

Deferred scope:

- XMP edit/sync support.
- In-place updates.
- Incremental-save mode.
- Metadata preservation guarantees for `pages`, `impose`, and `page resize`.
- PDF/A conformance validation.
- Digital-signature-preserving edits.

## Key Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Info and XMP divergence | PDF viewers may show different values | Warn when XMP exists; document MVP is Info-only |
| Metadata removal privacy | Old values must not remain recoverable | Prefer full rewrite over incremental update; document limits |
| Signed PDFs | Metadata writes may break signatures | Detect likely signature dictionaries; warn or require `--force` |
| Encrypted PDFs | Writes may fail or strip security incorrectly | Start with unencrypted PDFs; plan password support only after tests |
| Non-ASCII metadata | Garbled viewer display | Use PDF text-string encoding via `lopdf::text_string()` |
| Dates | Bad date strings confuse consumers | Validate `D:YYYYMMDDHHmmSSOHH'mm'`, `now`, and RFC3339 inputs |
| Large PDFs | Full rewrite can be expensive | Document full-rewrite behavior; add smoke test on fixture size only |

## Open Decisions for Implementation

1. Whether `metadata set` should update `ModDate` automatically by default.
   Recommendation: yes, unless `--no-touch-mod-date` is passed.
2. Whether `Producer` should be writable.
   Recommendation: yes, but require explicit `--producer`; do not overwrite it
   automatically.
3. Whether `clear` should allow `--all`.
   Recommendation: yes, but leave `Producer` and `CreationDate` out of `--all`
   unless explicitly named, to reduce accidental provenance loss.
4. Whether XMP should block writes.
   Recommendation: warn only for MVP; add `--strict-xmp` later if needed.
