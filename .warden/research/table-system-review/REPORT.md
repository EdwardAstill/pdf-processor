# Table System Review

Status: recorded
Date: 2026-05-15 AWST
Branch: stage-8-heading-formula

## Executive Conclusion

The local table system is good enough to preserve the Stage 8 recall floor, but
it is not the best architecture yet. Stage 8 currently measures table recall at
`4/4`, while table precision remains weak: engineering-calc is `1/2` table pages
(`50.0%`, one false positive) and engineering-report is `3/4` table pages
(`75.0%`, one false positive).

The root problem is not that the project needs to immediately swap in Camelot,
Tabula, Docling, or Table Transformer. The problem is that the local detector
does not yet have a real candidate provenance and scoring model. Broad page
layout regions can become table blocks with fixed confidence, and current eval
cannot tell whether a detected region tightly matches the expected table.

Stage 9 image/vector work should not start on top of that ambiguity. Add a
Stage 8.5 table-precision refactor first, then let Stage 9 inherit those floors.

## Local Architecture Observed

- `src/layout/table.rs` contains text/coordinate table detection, row grouping,
  alignment fallback, table confidence, and overlap suppression.
- `src/layout/table_detector.rs` adds geometry candidates from horizontal lines,
  line grids, and whitespace regions.
- `src/pipeline/mod.rs` converts coordinate and geometry candidates into
  `CoordinateTable` blocks, merges them with text, and uses tables as formula
  exclusion regions.
- `src/render/markdown.rs` renders both detected coordinate tables and an
  additional implicit numeric-table path during rendering.
- `src/eval/metrics.rs` scores tables as page-level presence/absence only.

## Findings

### High - Broad layout regions are emitted as table blocks

`src/layout/table_detector.rs:48-56` combines line-pair, grid, and whitespace
regions, then `src/layout/table_detector.rs:315-361` converts any accepted
region into rows with fixed confidence `0.78`. The whitespace path at
`src/layout/table_detector.rs:159-184` can create a region from aligned words
across a large part of a page.

Debug output confirms the failure mode. The calc fixture page 1 produced a
`Layout` table region from approximately `x=42..553`, `y=17..784`, containing
the title, metadata, bullets, prose, formulas, and the real input table. Calc
page 2 also emitted a `Layout` table despite `expected_tables: 0`.

Impact: table recall looks perfect while output quality and precision are not.
It also creates risk for Stage 9 because broad table boxes can interfere with
figure/vector region handling.

### Medium - Strategy names do not correspond to real strategies

`src/layout/table.rs:83-96` routes `LineGrid`, `ExplicitRegion`, and
`TextAlignment` to the same text-alignment function. This makes the code harder
to reason about and blocks strategy-specific gates.

External systems keep these distinctions explicit: pdfplumber exposes line,
text, and explicit strategies; Camelot and Tabula separate lattice/line and
stream/text modes.

### Medium - Candidate scoring lacks evidence and negative features

`src/layout/table.rs:932-961` scores coordinate tables from row consistency,
numeric density, and width ratio. `src/layout/table_detector.rs:360` assigns a
fixed confidence to geometry regions. `src/pipeline/mod.rs:403-408` then chooses
Markdown vs layout mostly from row consistency.

Missing evidence:

- source/provenance: line grid, ruling band, text network, numeric rows,
  explicit region, external model
- positive signals: ruling intersections, caption proximity, stable row/column
  graph, header/body split
- negative signals: broad-page coverage, prose stopword density, heading/list
  density, paragraph-like row lengths

### Medium - Overlap suppression keeps first, not best

`src/layout/table.rs:994-1004` suppresses overlapping candidates by retaining the
first seen candidate. The test in `src/pipeline/merge.rs:469-480` encodes the
same policy. This is risky once broad candidates and narrow candidates overlap:
order, not quality, decides which table survives.

### Medium - Eval cannot measure table region or structure quality

`src/eval/fixtures.rs:11-19` only stores `expected_tables` as a count per page.
`src/eval/metrics.rs:107-113` therefore treats any table block on an expected
table page as a true positive. A page can contain one real table and one broad
false table and still score as a table true positive.

This is below the bar used by modern table research and tooling. PubTables-1M
stores table, row, column, and cell boxes; GriTS evaluates table matrices rather
than only page presence.

### Low - Table reconstruction logic is split between extraction and render

`src/render/markdown.rs:765-860` contains an implicit numeric-table renderer
that runs over paragraph/list blocks, separate from the main table detectors.
That path is useful as a fallback, but it means table behavior is partly in the
renderer instead of in a single candidate/scoring pipeline.

## External System Lessons

- pdfplumber: keep vertical/horizontal strategy selection explicit and expose
  tolerances for lines, text, intersections, and explicit boundaries.
- Camelot: separate Stream, Lattice, Network, and Hybrid paths, and make debug
  evidence visible.
- Tabula: keep the ruled-table vs unruled-table distinction visible to users.
- Table Transformer / PubTables-1M: separate detection from structure
  recognition, and evaluate bounding boxes and cells, not just page presence.
- Docling: model-based layout/table/formula stages are useful sidecars or
  benchmark comparators, but they should not become mandatory for the local fast
  path.

## Recommended Refactor

Stage 8.5 should add a single table candidate contract before more heuristics:

```rust
enum TableEvidenceSource {
    RulingGrid,
    RulingBand,
    TextNetwork,
    NumericRows,
    ExplicitRegion,
    ExternalModel,
}

struct TableEvidence {
    source: TableEvidenceSource,
    row_consistency: f32,
    column_alignment: f32,
    numeric_density: f32,
    ruling_intersections: usize,
    caption_score: f32,
    broad_page_penalty: f32,
    prose_penalty: f32,
    debug_reasons: Vec<String>,
}
```

Then:

1. Return candidates with provenance and evidence from every detector.
2. Score candidates once, in one place.
3. Prefer narrow/high-evidence candidates over broad candidates during overlap
   suppression.
4. Drop or quarantine broad layout candidates unless they have independent table
   evidence.
5. Move implicit numeric-table rendering behind the same candidate path.
6. Extend eval with optional expected table boxes and IoU-based precision before
   changing table heuristics.

## Roadmap Adjustment

Insert Stage 8.5 before Stage 9:

- Keep Stage 8 heading/formula/table recall floors.
- Add table region precision metrics.
- Suppress the two tracked broad table false positives while preserving `4/4`
  table recall.
- Refactor candidate provenance/scoring enough that Stage 9 image/vector
  regions do not inherit broad table boxes.

Stage 9 should still be image/vector, but its first gate should include Stage
8.5 table precision floors as well as the Stage 8 heading/formula floors.
