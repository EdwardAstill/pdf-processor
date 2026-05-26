# Renderer Refactor Plan

**Date:** 2026-05-26
**Goal:** Split `src/render/markdown.rs` (2,670 lines, 69 functions) into focused modules.
The architectural problem: the renderer does layout inference (table detection, form field
parsing, scholarly page analysis) inline during rendering. These belong in `src/layout/`.

**Implemented:** the renderer now delegates text helpers, media planning, scholarly front-matter
rendering, and table/form inference to focused modules. `src/layout/table_inference.rs` returns
structural data (`StructuredRegionKind`) rather than pre-rendered Markdown, so `layout` no longer
depends on renderer table serialization.

## Module split

### 1. `src/layout/table_inference.rs` (NEW — ~500 lines moved from renderer)

Layout inference that the renderer should not be doing:

| Function (renamed) | Lines | Purpose |
|---|---|---|
| `detect_structured_region` (was `try_render_structured_region`) | ~20 | Entry point: tries numeric tables then form fields |
| `detect_implicit_numeric_table` (was `try_render_implicit_numeric_table`) | ~100 | Detects table-like runs of paragraphs |
| `detect_form_field_blocks` (was `try_render_form_fields`) | ~55 | Detects form field label-value pairs |
| `collect_textish_run` | ~14 | Groups consecutive text blocks |
| `parse_numeric_row` | ~44 | Parses a line into numeric values |
| `normalize_numeric_row` | ~10 | Normalizes row to dominant column count |
| `looks_like_table_header` | ~9 | Heuristic for header rows |
| `derive_table_headers` | ~40 | Extracts column headers from rows |
| `looks_like_field_label` | ~9 | | 
| `looks_like_key_value_label` | ~11 | |
| `normalize_field_label` | ~8 | |
| `extract_inline_field_value` | ~13 | |
| `looks_like_choice_line` | ~11 | |
| `looks_like_field_value` | ~9 | |
| `normalize_field_value` | ~4 | |
| `numeric_value_re`, `invoice_header_re`, `key_value_label_keyword_re` | ~15 | Static regexes |
| `looks_like_invoice_header`, `extract_single_numeric_value`, `is_numeric_separator` | ~20 | |
| `normalize_structured_text`, `strip_leading_list_marker` | ~35 | Text normalization used by inference |
| Types: `StructuredRender`, `ParsedNumericRow` | | |

Return type changes: `StructuredRender.markdown` field removed — inference only produces
structural data. The renderer converts it to markdown.

### 2. `src/render/media.rs` (NEW — ~200 lines)

| Function | Purpose |
|---|---|
| `build_page_media_plan` | Determines which images/figures are unique |
| `normalized_repeated_text_key` | Normalizes text for dedup comparison |
| `should_suppress_repeated_text_block` | Dedup decision |
| `is_media_block`, `media_has_caption`, `media_fingerprint` | Media analysis |
| Types: `MediaPlan`, `MediaFingerprint` | |

### 3. `src/render/scholarly.rs` (NEW — ~450 lines)

| Function | Purpose |
|---|---|
| `render_scholarly_first_page` | Detects and renders scholarly front matter |
| `find_scholarly_title_candidate` | Finds title block |
| `find_front_matter_end` | Detects where front matter ends |
| `is_scholarly_metadata_line`, `is_scholarly_note_line`, `is_abstract_heading` | Classification |
| `split_abstract_block`, `is_main_section_heading` | Structure detection |
| `looks_like_author_block`, `looks_like_author_name_line`, `looks_like_affiliation_line` | Author detection |
| `has_title_stopword`, `collect_author_entries`, `table_cell_author_entries` | |
| `normalize_front_matter_text` | Text cleanup |
| Regexes: `scholarly_metadata_re`, `scholarly_note_re`, `abstract_heading_re`, `numbered_section_heading_re`, `affiliation_keyword_re` | |
| Type: `ScholarlyFrontMatter` | |

### 4. `src/render/text.rs` (NEW — ~200 lines)

| Function | Purpose |
|---|---|
| `escape_comment_attr` | HTML comment attribute escaping |
| `normalize_heading_text` | Heading text normalization |
| `normalize_paragraph_text` | Paragraph text normalization |
| `inline_wrap` | Bold/italic inline markup |
| `escape_table_cell` | Pipe escaping in table cells |
| `append_paragraph`, `append_plain_text` | Markdown string building |
| `normalize_structured_text` (from table_inference dependency) | |

### 5. `src/render/markdown.rs` (REMAINING — ~1,000 lines)

Pure rendering after the split:

| Function | Purpose |
|---|---|
| `render_page` | Page-level orchestrator (calls sub-modules) |
| `render_heading` | # heading output |
| `render_list` | Bullet/numbered list |
| `render_table` (cell blocks) | GFM table from TableCell blocks |
| `render_coordinate_table` | CoordinateTable rendering |
| `render_detected_markdown_table` | Detected table → GFM |
| `build_table_grid` | Cell grid assembly |
| `render_table_grid` | Grid → GFM output |
| `render_key_value_grid` | Key-value pairs as markdown |
| `append_rendered_block` | Block dispatch |
| `build_render_context` | Context construction |
| `split_into_sections`, `parse_page_marker`, `parse_heading_line` | Section splitting |
| Types: `Section`, `RenderContext`, `ExtractedImage` | |
| Tests | |

## Dependency graph

```
markdown.rs (rendering)
  ├── media.rs (dedup before rendering)
  ├── scholarly.rs (first-page special rendering)
  ├── text.rs (shared text helpers)
  └── layout/table_inference.rs (structured region detection)
        └── text.rs (normalize_structured_text)

No circular dependencies.
```

## Additional cleanups

### 6. Block construction centralization

Add `Block::new_text()` and `Block::new_special()` constructors to `src/document/types.rs`
to reduce the boilerplate across 16 construction sites.

### 7. Hybrid routing unification

Extract common Docling client setup and error handling from `apply_to_document` and
`apply_regions_to_document` into a shared helper.

### Estimated impact

| Module | Before | After |
|---|---|---|
| `src/render/markdown.rs` | 2,670 lines | ~1,000 lines |
| `src/layout/table_inference.rs` | — | ~500 lines |
| `src/render/media.rs` | — | ~200 lines |
| `src/render/scholarly.rs` | — | ~450 lines |
| `src/render/text.rs` | — | ~200 lines |

## Performance gate

1. `cargo clippy -- -D warnings` — clean
2. `cargo test` — all passing
3. `cargo build --release` — succeeds
4. Conversion benchmark — no regression

## Execution order

1. Extract `text.rs` first (no dependencies on other new modules)
2. Extract `media.rs` (depends on text.rs)
3. Extract `scholarly.rs` (depends on text.rs + media.rs)
4. Extract `table_inference.rs` — rename functions, remove markdown generation
5. Update `markdown.rs` — wire up imports, keep pure rendering
6. Centralize Block construction
7. Unify hybrid routing
8. Verify at each step
