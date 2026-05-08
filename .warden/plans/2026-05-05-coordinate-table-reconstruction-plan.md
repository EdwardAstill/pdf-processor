# Coordinate Table Reconstruction Plan

status: completed
date: 2026-05-05
shape: research-plan-execute-review

## Task

Improve `pdfp` table conversion by using local text coordinates to render real Markdown tables for born-digital PDFs like the Crosby catalogue, with fixed-width layout fallback for low-confidence cases.

## Research Inputs

- Research report: `.warden/research/coordinate-table-reconstruction/REPORT.md`
- Evidence ledger: `.warden/research/coordinate-table-reconstruction/evidence.jsonl`
- Contradictions: `.warden/research/coordinate-table-reconstruction/contradictions.md`
- Crosby source PDF: `/home/eastill/projects/literature/specs/crosby/Crosby & Gunnebo Industries Catalog - Metric 2025-2026.pdf`

## Assumptions

- id: A1
  statement: The local MuPDF wrapper exposes enough character geometry to build word boxes without adding a Python sidecar.
  type: architectural
  source: local crate inspection and MuPDF docs.
  check: `rg -n "pub fn quad|pub fn origin|pub fn chars|pub fn lines" ~/.cargo/registry/src/*/mupdf-0.6.0/src/text_page.rs` -> finds exposed APIs.
  if false: fall back to `TextPage::to_json()` parsing or a narrow `mupdf-sys` call.
  owner: Task 1

- id: A2
  statement: Crosby page 23 is a valid target fixture for table reconstruction because the PDF text layer preserves row/column positions.
  type: repo-state
  source: local `pdftotext -layout` run.
  check: `pdftotext -f 23 -l 23 -layout "$CROSBY_PDF" - | rg "G-213 Round Pin Anchor Shackles|1018017|1082330"` -> exits 0.
  if false: pick another Crosby table page from the same catalogue.
  owner: Task 6

- id: A3
  statement: Real Markdown should be emitted only for high-confidence regions; low-confidence regions should not be collapsed into prose.
  type: design
  source: research report and current bad Crosby output.
  check: test fixtures include both a successful Markdown table and a low-confidence fallback.
  if false: disable native table rendering by default and expose it behind an opt-in flag.
  owner: Task 5

- id: A4
  statement: The default converter can change bad collapsed table output into better table/layout output without breaking normal prose tests.
  type: design
  source: user request and existing golden tests.
  check: `cargo test --test golden` -> passes after snapshots/fixtures are intentionally updated.
  if false: add `--tables auto|native|layout|off` and keep legacy behavior available.
  owner: Task 7

## Behavior Contract

Default behavior:

```bash
pdfp convert input.pdf -o out
```

The converter should automatically improve table-like regions:

- high-confidence coordinate table -> GitHub-flavored Markdown table,
- low-confidence table-like region -> fenced `text` layout block,
- non-table prose -> existing paragraph/list/heading behavior.

Add table debugging controls:

```bash
pdfp convert input.pdf -o out --debug-tables
pdfp convert input.pdf -o out --tables auto
pdfp convert input.pdf -o out --tables native
pdfp convert input.pdf -o out --tables layout
pdfp convert input.pdf -o out --tables off
```

`--tables auto` should be the default once tests show no broad regressions. `--tables layout` is the explicit safety mode for catalogue-style PDFs when real Markdown confidence is not enough.

## Task 1: Preserve Word Geometry

Block: execute
Skill: rust
Status: completed

Instruction:

Extend the document data model so `RawPage` retains page-level text geometry below `RawTextBlock`:

- `RawChar` or internal char geometry from MuPDF `TextChar::quad`.
- `RawWord { text, bbox, baseline_y, font_size, source_block_id, source_line_id }`.
- Optional `RawTextLine` if it simplifies row grouping.

Keep existing `RawTextBlock` behavior for headings/prose.

Acceptance:

- `cargo test table_geometry_extracts_words_from_chars` -> passes with word bboxes for a synthetic char sequence.
- `cargo test` -> existing extractor/render tests still compile.
- `rg -n "words: Vec<RawWord>|struct RawWord" src/document src/pdf` -> exits 0.

## Task 2: Build Native Table Regions From Words

Block: execute
Skill: rust
Status: completed

Instruction:

Replace or supplement `src/layout/table.rs` with a word-level detector inspired by Camelot Stream/pdfplumber text strategy:

- group words into visual rows by y-overlap/baseline tolerance,
- find contiguous row runs with repeated column structure,
- support wide tables with more than 10 columns,
- use left edges for text columns and right edges for numeric columns,
- infer column bands from recurring x positions,
- assign words into cells,
- merge adjacent words in each cell by x position.

Acceptance:

- `cargo test detects_wide_coordinate_table_from_words` -> emits at least 12 columns.
- `cargo test handles_right_aligned_numeric_columns` -> numeric values stay in correct columns.
- `cargo test does_not_classify_normal_paragraphs_as_tables` -> prose remains paragraphs.

## Task 3: Render Multi-Row Headers And Cells

Block: execute
Skill: rust
Status: completed

Instruction:

Add a table model independent from `BlockKind::TableCell`, for example:

- `DetectedTable { bbox, rows, confidence, fallback_layout }`
- `DetectedCell { row, col, text, bbox }`

Render high-confidence tables as Markdown:

- combine stacked header rows where obvious (`Working` + `Load` + `Limit` + `(t)`),
- preserve units in headers,
- escape Markdown table pipes,
- keep row count and column count stable.

Acceptance:

- `cargo test renders_stacked_catalogue_headers` -> output includes `Working Load Limit (t)`.
- `cargo test renders_markdown_table_with_stable_column_count` -> every row has the same cell count.
- `cargo test table_cells_escape_markdown_pipes` -> passes.

## Task 4: Add Fixed-Width Layout Fallback

Block: execute
Skill: rust
Status: completed

Instruction:

When a region is table-like but native reconstruction confidence is low, render a fenced layout block instead of collapsed prose. Use the preserved words and page coordinates to create a monospaced row layout. If needed, use Poppler `pdftotext -layout` only as a dev comparison, not as a runtime dependency.

Acceptance:

- `cargo test low_confidence_table_renders_fenced_layout` -> output contains ```` ```text ```` and aligned rows.
- `cargo test low_confidence_table_does_not_emit_collapsed_long_line` -> no table-like line exceeds a configured length threshold.

## Task 5: Integrate With Page Rendering

Block: execute
Skill: rust
Status: completed

Instruction:

Thread detected table regions into the page/render pipeline without duplicating the same text as both paragraphs and table cells:

- detect table regions before classification or immediately after raw extraction,
- suppress source blocks/words covered by rendered table regions,
- preserve surrounding headings, diagrams, captions, and warnings,
- add `--tables` and `--debug-tables` CLI flags with help text.

Acceptance:

- `cargo test --test cli_help` -> table flags appear in help.
- `cargo test table_region_suppresses_source_blocks` -> no duplicate table text.
- `cargo test surrounding_text_survives_table_replacement` -> heading/prose around the table remains.

## Task 6: Crosby Baseline And Regression Fixture

Block: execute
Skill: rust
Status: completed

Instruction:

Add a focused integration check against the local Crosby PDF when present. The test should be skipped when the fixture is absent unless the quality harness is running.

Use page 23 / catalogue printed page 22 as the primary fixture:

- expected title: `G-213 Round Pin Anchor Shackles`,
- expected values: `1018017`, `1018295`, `1082330`,
- expected headers: `Nominal Size (in)`, `Working Load Limit (t)`, `Stock No.`, `Weight Each (kg)`.

Acceptance:

- `cargo test crosby_g213_table_reconstructs_when_fixture_present` -> passes or reports skipped fixture.
- `pdfp convert "$CROSBY_PDF" -o /tmp/pdfp-crosby-table-test --tables native` -> output page contains a Markdown table with the expected values.
- `pdfp convert "$CROSBY_PDF" -o /tmp/pdfp-crosby-layout-test --tables layout` -> output contains a readable fenced layout table.

## Task 7: Corpus Verification And Performance

Block: review
Skill: performance-profiling
Status: completed

Instruction:

Run the converter against representative PDFs:

- Crosby catalogue page/table checks,
- existing small golden fixtures,
- `attention.pdf`, `clip.pdf`, `resnet.pdf` if present.

Measure runtime, Markdown size, count of Markdown tables, count of fenced layout fallbacks, and number of very long lines.

Acceptance:

- `cargo test` -> passes.
- `scripts/quality-report.sh` or equivalent quality command -> completes.
- Report written under `.warden/research/coordinate-table-reconstruction/baseline/`.
- No normal prose fixture regresses into large false-positive tables.

## Task 8: Documentation

Block: execute
Skill: writing
Status: completed

Instruction:

Update user and internals docs:

- `README.md`
- `docs/CLI.md`
- `docs/pdf-internals.md`
- Warden `pdf-processing` skill if CLI flags are user-facing.

Explain:

- born-digital coordinate tables,
- layout fallback,
- when OCR/table image recognition is still needed,
- how to debug bad table extraction with `--debug-tables`.

Acceptance:

- `rg -n -- "--tables|--debug-tables|coordinate.*table|layout fallback" README.md docs` -> exits 0.
- `pdfp convert --help` -> includes table flags after rebuild.
