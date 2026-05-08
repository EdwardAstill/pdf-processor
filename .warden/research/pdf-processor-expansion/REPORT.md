# PDF Processor Expansion Viability Report

Date checked: 2026-05-05

## Question

Can `pdfp` grow from a PDF-to-Markdown converter into a broader local PDF processor for search, page operations, booklet/2-up layout, resizing, and related workflows?

## Short Answer

Yes, but it should grow as a PDF processor with subcommands, not by stuffing more flags into the current converter command.

The easy first wins are:

- page count / metadata / text-density inspection
- text search with page numbers and hit boxes
- extract/delete page ranges
- reorder pages by an explicit page list
- split PDFs into page ranges

The viable but more careful work is:

- 2-up layout
- booklet imposition
- page resizing / scaling / margins
- merge with object/resource preservation

Those are not conceptually hard, but they are harder to get correct because they involve page geometry, rotation, media boxes, crop boxes, resource grafting, annotations, outlines, and print-order rules.

## Local Evidence

The current dependency set already has useful building blocks:

- `mupdf::Document::page_count`
- `mupdf::Document::load_page`
- `mupdf::Page::search`
- `mupdf::TextPage::search`
- `mupdf::TextPage::to_text`
- `mupdf::pdf::PdfDocument::delete_page`
- `mupdf::pdf::PdfDocument::insert_page`
- `mupdf::pdf::PdfDocument::new_page`
- `mupdf::pdf::PdfDocument::save`
- `mupdf::pdf::PdfDocument::new_graft_map`
- `mupdf::pdf::PdfGraftMap::graft_object`

The existing code already uses `PdfDocument::delete_page` in `src/hybrid/page_extract.rs` to extract one page by deleting all other pages from a temporary document. That proves page deletion and save are already viable in this codebase.

## Design Direction

Move from one flat command:

```bash
pdfp input.pdf [convert flags]
```

to a backwards-compatible command tree:

```bash
pdfp convert input.pdf -o out/
pdfp search input.pdf "needle" --json
pdfp inspect input.pdf --json
pdfp pages delete input.pdf --pages 2,4-6 -o edited.pdf
pdfp pages extract input.pdf --pages 1-3,9 -o excerpt.pdf
pdfp pages reorder input.pdf --pages 1,3,2,4-8 -o reordered.pdf
pdfp impose 2up input.pdf -o two-up.pdf
pdfp impose booklet input.pdf -o booklet.pdf
pdfp page resize input.pdf --paper a4 --fit contain -o resized.pdf
```

For compatibility, bare `pdfp input.pdf ...` should remain an alias for `pdfp convert input.pdf ...` during the transition.

## Alternatives Considered

| Option | Description | Why choose it | Why not |
| --- | --- | --- | --- |
| Flat flags forever | Keep `pdfp input.pdf --delete-pages ... --search ...` | Minimal migration | Becomes confusing quickly; many verbs on one noun |
| Subcommands in one binary | `pdfp convert`, `pdfp search`, `pdfp pages`, `pdfp impose` | Best fit: shared PDF stack, clear UX | Requires CLI migration work |
| Separate binaries | `pdfp-md`, `pdfp-pages`, `pdfp-search` | Clean plumbing boundaries | More packaging and discovery burden |

Recommendation: one binary with subcommands, plus backwards-compatible flat `convert`.

## Capability Map

| Capability | Viability | Notes |
| --- | --- | --- |
| Search text and return pages | High | `Page::search` and `TextPage::search` already exist. |
| Search with bounding boxes | High | Search returns quads; JSON output can expose page number and quads. |
| Inspect page count / dimensions / text density | High | Existing extractor and triage already compute most of this. |
| Delete pages | High | `PdfDocument::delete_page` exists and is already used indirectly. |
| Extract page ranges | High | Existing single-page extraction can be generalized. |
| Reorder pages | Medium-high | Needs careful copy/graft or delete/insert strategy; viable with `PdfDocument` and grafting APIs. |
| Split into chunks | High | Same building blocks as extract. |
| Merge PDFs | Medium | Grafting/importing page objects is viable but must preserve resources and handle outlines/metadata intentionally. |
| Resize pages | Medium | Needs clear semantics: media box only, crop box, scale content, or add margins. |
| 2-up | Medium | Needs page composition; can render/graft existing pages onto new pages. Test visually/structurally. |
| Booklet imposition | Medium | Page ordering is simple; robust geometry/duplex behavior needs tests. |
| Rotate pages | Medium | Need confirm high-level wrapper support or write page dictionary edits. |
| Redaction / annotation editing | Later | Higher risk; can destroy information if wrong. |

## Constraints

- Preserve current PDF-to-Markdown behavior.
- Do not mutate input PDFs in place by default.
- Use explicit `-o/--output` for processor operations that create PDFs.
- Machine-readable modes should write JSON only to stdout; progress/errors go to stderr.
- Page ranges must be 1-indexed at the CLI, even if implementation is 0-indexed.
- Destructive-sounding operations like `delete` still produce a new output PDF unless `--in-place` is explicitly added later with a backup strategy.

## Suggested First Build

Start with `inspect` and `search`.

They add immediate value, validate the subcommand architecture, are easy to test, and do not risk corrupting PDFs.

Second, add page range operations:

- `pages extract`
- `pages delete`
- `pages split`

Third, add reordering and merge.

Fourth, add imposition/layout:

- `impose 2up`
- `impose booklet`
- `page resize`

## Testing Strategy

Use existing `example/pdf` fixtures:

- `golden__lorem.pdf` for tiny deterministic tests.
- `attention.pdf` for search results and page numbers.
- `golden__pdfua-1-reference-suite-1-1__PDFUA-Ref-2-04_Presentation.pdf` for page deletion/reorder on multi-page PDF.
- `golden__chinese_scan.pdf` to prove native text search returns zero until OCR is used.
- `golden__issue-336-conto-economico-bialetti.pdf` for search in table-heavy content.

Processor tests should assert:

- output PDF exists
- output page count is expected
- source input is unchanged
- `pdfp inspect --json` parses with `jq`
- `pdfp search --json` returns expected page numbers
- conversion still passes after processor changes

