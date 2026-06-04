# Markdown Rendering and Figure Extraction (`src/render/`, `src/figure/`)

Serialises assembled blocks into Markdown and detects/renders figure regions as PNG snapshots.

---

## Markdown Rendering (`src/render/`)

## Source files

| File | Purpose |
|---|---|
| `mod.rs` | Main `MarkdownRenderer` — iterates pages, dispatches block types to renderers |
| `markdown/mod.rs` | Renderer implementation: `render_document()`, per-page rendering, inline formatting |
| `markdown/clean.rs` | Clean-style post-processor: reflow paragraphs, normalise glyphs, suppress review markers |
| `markdown/tests.rs` | Renderer unit tests |
| `table.rs` | Table rendering: Markdown pipe tables, key-value lists, numeric tables, form fields, coordinate tables |
| `scholarly.rs` | Scholarly front-matter rendering: metadata suppression, abstract splitting, affiliation demotion |
| `media.rs` | Media deduplication: repeated edge-text suppression, repeated image fingerprinting |
| `text.rs` | Text helpers: `escape_table_cell()`, `append_paragraph()`, heading normalisation |

## Key types

| Type | File | Purpose |
|---|---|---|
| `MarkdownRenderer` | `markdown/mod.rs` | Main renderer struct. Configurable with `MarkdownStyle` |
| `MarkdownStyle` | `markdown/mod.rs` | `Clean`, `Faithful`, `Review` — controls post-processing behaviour |
| `RenderedDocument` | `mod.rs` | Output: `markdown` string and `extracted_images` list |
| `RenderContext` | `media.rs` | Tracks repeated edge text and media across pages for suppression |
| `PageMediaPlan` | `media.rs` | Per-page plan: which block IDs (images, figures) to keep |
| `ScholarlyFrontMatterRender` | `scholarly.rs` | Renders the first page specially when it looks like a paper |

## Key functions

### MarkdownRenderer (`markdown/mod.rs`)

| Function | Description |
|---|---|
| `MarkdownRenderer::clean(...)` | Create renderer with `Clean` style (default) |
| `MarkdownRenderer::faithful(...)` | Create renderer with `Faithful` style |
| `MarkdownRenderer::with_style(...)` | Create renderer with explicit style |
| `renderer.render_document(doc) -> RenderedDocument` | Render entire document page by page |

Block rendering dispatch (in `render_document`):
- `Heading` → `#` / `##` / ...
- `Paragraph` → plain text (clean style may reflow with next paragraph)
- `ListItem` → `- ` or `1. ` bullet
- `TableCell` → collect consecutive cells, render as Markdown table
- `CoordinateTable` → delegate to `render_coordinate_table()` in `table.rs`
- `Caption` → `*caption text*`
- `CodeBlock` → fenced ` ``` ` block
- `Image` / `Figure` → `![image](path)`
- `Formula` → `$$ latex $$` or `$ latex $`
- `FormulaReview` → `<!-- formula-review: ... -->` comment
- `PageNumber` / `RunningHeader` / `RunningFooter` / `Artifact` → suppressed

### Clean style (`markdown/clean.rs`)

| Function | Description |
|---|---|
| `clean_markdown(raw_markdown) -> String` | Post-processes rendered markdown: reflows hyphenated line-breaks, normalises ligatures (ﬀ→ff, ﬁ→fi), strips formula-review comments, preserves table and code-block structure |

### Table rendering (`table.rs`)

| Function | Description |
|---|---|
| `render_coordinate_table(table: &DetectedTable) -> String` | Renders a coordinate table as Markdown pipe table or fenced layout block |
| `render_table(table_blocks: &[&Block]) -> String` | Renders `TableCell` blocks as a Markdown pipe table |
| `render_structured_region(markdown, blocks, start, region)` | Renders inferred numeric tables and form-field lists |
| `render_inferred_numeric_table(headers, rows, total) -> String` | Invoice-style numeric table with optional total row |

### Scholarly front matter (`scholarly.rs`)

| Function | Description |
|---|---|
| `ScholarlyFrontMatterRender::render(page) -> ScholarlyFrontMatterRender` | Special first-page rendering for academic papers: detects abstract heading, separates metadata (author lines, affiliations, arXiv IDs), demotes author contact tables to plain text |

### Media dedup (`media.rs`)

| Function | Description |
|---|---|
| `build_render_context(doc) -> RenderContext` | Scans all pages for repeated edge text (headers/footers) and repeated images |
| `plan_page_media(page, ctx) -> PageMediaPlan` | Decides which image/figure blocks to keep for a page |

## CLI flags

| Flag | Effect |
|---|---|
| `--markdown-style clean\|faithful\|review` | Controls the rendering style (default `clean`) |
| `--conservative` | Forces review-safe rendering (no speculative tables/formulas) |
| `--no-images` | Disables image output |
| `--figures embedded\|snapshot\|both\|none` | Controls image source mode |

## Cross-references

- [pipeline.md](pipeline.md) — what feeds into the renderer
- [pdf-extraction.md](pdf-extraction.md) — embedded image extraction
- `wiki/topics/markdown-rendering.md` — rendering pipeline details
- `wiki/topics/figure-snapshot-extraction.md` — figure detection and rendering
- `wiki/structures/tables.md` — table output modes
- `wiki/structures/equations.md` — formula output modes
- `docs/CLI.md` — full CLI reference with style examples

---

## Figure Extraction (`src/figure/`)

Detects visual figure regions on PDF pages and renders them as PNG snapshots. Separate from embedded image extraction (which happens in `src/pdf/extractor.rs`).

### Source files

| File | Purpose |
|---|---|
| `mod.rs` | Module root |
| `detect.rs` | Figure candidate detection: groups images with captions, estimates caption-only regions |
| `render.rs` | Render figure regions to PNG snapshots using MuPDF page rendering |

### Key types

| Type | Purpose |
|---|---|
| `FigureDetectionOptions` | Configuration: `padding`, `min_width_ratio`, `min_height_ratio` (defaults: 8pt, 0.10, 0.06) |
| `FigureCandidate` | A detected figure region: `bbox`, `caption_bbox`, `caption_text`, `seed_image_indices`, `confidence`, `reason` |
| `RenderedFigure` | Output of rendering: the candidate plus a resulting `Block` |

### Key functions

| Function | Description |
|---|---|
| `detect_figure_candidates(raw_page, blocks, options)` | Groups embedded images with nearby captions. Falls back to estimated caption-only regions |
| `render_figure_snapshots(pdf_path, page_num, candidates, dir, dpi)` | Renders each candidate region at the specified DPI, saves as `pageN_figM.png` |

### CLI flags

| Flag | Effect |
|---|---|
| `--figures embedded\|snapshot\|both\|none` | Image output mode. Embedded = saved images (fast). Snapshot = rendered visual regions. Both = both |
| `--figure-dpi <N>` | Snapshot resolution (default 200) |
| `--figure-padding <PTS>` | Padding around detected regions (default 8.0) |
| `--debug-figures` | Write figure candidate JSON under `debug/figures/` |
| `--conservative` | Disables figure snapshots (embedded only) |
