# Pipeline merge/suppress extraction

**Status:** implemented
**Created:** 2026-05-15
**Branch:** `main`
**Base:** `5cff09d` (Merge stage-7.5 baseline corpus into main)

## Goal

Carve the pure geometry-based merge and suppression helpers out of
`src/pipeline.rs` (1250 lines) into a dedicated, unit-tested submodule
`src/pipeline/merge.rs`. This is purely a refactor — no behaviour change.
It exists to give Stage 8 a regression net before any heuristic tuning.

## Why this refactor is worth doing now

- Stage 8's planned work touches heading classification (size + struct-tree
  + bold-promotion) and formula extraction. Both consumers feed the
  formula/table merge and suppression pipeline. Extracting the merge layer
  first means Stage 8 can change scoring thresholds without re-reading a
  1250-line file to find the geometry helpers.
- All twelve helpers were previously private and untested. Three of them
  had ad-hoc tests at the bottom of `pipeline.rs`. Promoting them to
  `pub(super)` items in their own file makes them straightforward to test
  in isolation.
- The scope is tight: only pure functions over `Block`, `Bbox`,
  `TableCandidate`, and `FormulaCandidate` were moved. The remaining
  formula-text helpers (`build_formula_latex`, `unicode_to_latex`,
  `should_emit_formula_candidate`, `is_unresolved_formula_review`,
  `formula_candidates_to_blocks`, `table_candidates_to_blocks`) stay in
  `pipeline/mod.rs` because they are pipeline-stage glue, not geometry.

## Scope (moved into `src/pipeline/merge.rs`)

- `bbox_overlap_ratio`
- `bbox_overlap_smaller`
- `suppress_overlapping_table_candidates`
- `formula_excluded_regions`
- `suppress_text_covered_by_tables`
- `suppress_text_covered_by_furniture`
- `suppress_text_covered_by_formulas`
- `suppress_formula_candidates_overlapping_tables`
- `merge_text_and_formulas`
- `merge_text_and_tables`
- `merge_text_and_images`
- `merge_media_blocks`

All twelve are now `pub(super)` items reachable from `pipeline/mod.rs` via
an explicit `use merge::{...}` import.

## Out of scope (explicitly deferred)

- Restructuring `table.rs` / `table_detector.rs` — Stage 8 will be tuning
  table precision; refactoring those modules now guarantees merge pain.
- Splitting `src/render/markdown.rs` — long but cohesive; no Stage 8 work
  routes through a structural change here.
- Introducing a `BlockSource` trait for extractors — premature; only paid
  off when adding a sixth extractor.
- Pulling heading classification into a strategy pattern — premature until
  Stage 8 introduces a fourth heading signal.

## Tasks

- [x] Task 1: Inventory the merge/suppress helpers in `pipeline.rs`,
  confirm none are referenced from outside the file, and identify their
  tests.
  - Acceptance: `rg -n "merge_text_and_|suppress_|bbox_overlap_|formula_excluded_regions" src` shows usages only in `src/pipeline/`.
- [x] Task 2: Convert `src/pipeline.rs` -> `src/pipeline/mod.rs` and add
  `src/pipeline/merge.rs` containing the twelve extracted helpers.
  - Acceptance: `cargo build` passes; `pipeline/mod.rs` imports the helpers
    via an explicit `use merge::{...}`.
- [x] Task 3: Add unit tests for the extracted helpers in
  `pipeline/merge.rs`, moving the three existing relevant tests
  (`suppresses_formula_candidates_inside_strong_tables`,
  `formula_exclusions_include_tables_and_furniture`,
  `furniture_bboxes_suppress_text_blocks`,
  `formula_blocks_suppress_overlapping_text_blocks`) and adding fresh
  coverage for the rest.
  - Acceptance: `cargo test --lib pipeline::merge` -> 16 tests pass.
- [x] Task 4: `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` all clean.
- [x] Task 5: Update `CLAUDE.md` module map, `.warden/NEXT.md`, and record
  the deferred refactor candidates so they are not forgotten.

## Verification

- `cargo build` -> pass.
- `cargo test` -> 179 + 189 + everything else green; 16 new tests under `pipeline::merge::tests`.
- `cargo clippy --all-targets -- -D warnings` -> pass.
- No public API changed; the helpers are `pub(super)` inside the
  `pipeline` module.

## Followups (NOT done — captured for later)

- If Stage 8 adds a fourth heading signal that the current `if/else`
  cannot host cleanly, revisit the `HeadingClassifier` strategy idea.
- After Stage 8's table precision work lands, reconsider whether
  `layout/table.rs` + `table_detector.rs` should be reorganised into a
  staged `detect -> score -> suppress -> emit` module.
- If `render/markdown.rs` keeps growing, split out media planning and
  scholarly front matter into their own files; right now the file is long
  but cohesive, so this is purely cosmetic.
