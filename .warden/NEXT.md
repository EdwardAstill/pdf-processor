# Next Work

Updated: 2026-05-15 AWST
Branch: main
Remote: origin/main (in sync at session start)

## Current Goal
Get into Stage 8 (numbered engineering headings + display/calc formula recall).
Pre-Stage-8 refactor of the pipeline merge/suppress layer is complete and
unit-tested; Stage 8 heuristic work can start without re-reading 1250 lines
of `pipeline.rs`.

## Completed This Session
- Pipeline merge/suppress refactor: converted `src/pipeline.rs` ->
  `src/pipeline/mod.rs` and extracted twelve pure geometry helpers into
  `src/pipeline/merge.rs` (merge text+formulas/tables/images/media,
  suppress text covered by tables/furniture/formulas, suppress overlapping
  table candidates, suppress formula candidates overlapping tables,
  formula_excluded_regions, bbox_overlap_ratio, bbox_overlap_smaller).
- 16 unit tests now cover the extracted helpers in
  `pipeline::merge::tests` (4 lifted from `pipeline::tests`, 12 new).
- Plan recorded at `.warden/plans/2026-05-15-pipeline-merge-extraction.md`.
- Stage 7.5 plan annotated with a pointer to the new refactor plan.

## Changed Files
- `src/pipeline.rs` -> `src/pipeline/mod.rs` (git move).
- `src/pipeline/merge.rs` - new geometry-only submodule + 16 tests.
- `src/pipeline/mod.rs` - removed twelve extracted fn bodies; added
  `mod merge;` and an explicit `use merge::{...}`; dropped four tests now
  living in `merge::tests`.
- `.warden/plans/2026-05-15-pipeline-merge-extraction.md` - new plan.
- `.warden/plans/2026-05-14-stage7-5-baseline-consolidation.md` -
  cross-link added in Notes section.
- `.warden/NEXT.md` - this update.
- `CLAUDE.md` - module map updated for the new `pipeline/` directory.

## Verification
- `cargo build` -> pass.
- `cargo test` -> all suites green; `pipeline::merge::tests` = 16 ok,
  `pipeline::tests` = 4 ok, no failures elsewhere.
- `cargo clippy --all-targets -- -D warnings` -> pass.
- No public API changed (helpers are `pub(super)` inside the `pipeline` module).

## Blockers / Open Questions
- Real ONNX formula recognition was not run because RapidLaTeX-OCR model
  files are not present locally (carried over from Stage 7.5).
- `libpdfium` is not visible in `ldconfig -p`, so tagged-PDF runtime
  extraction still needs a local dependency install (carried over).
- Table recall is currently 100% on the baseline fixtures, but debug
  output shows broad whole-page table regions; table precision is not yet
  measured by `pdfp eval` (carried over).

## Deferred refactors (intentionally not done)
- Splitting `src/render/markdown.rs` (2635 lines) - long but cohesive;
  nothing in Stage 8 routes through structural change here.
- Pulling heading classification into a strategy pattern - premature
  until Stage 8 actually introduces a fourth heading signal that the
  current `if/else` chain cannot host.
- Reorganising `layout/table.rs` + `table_detector.rs` - Stage 8 will be
  tuning table precision; refactoring those modules now guarantees merge
  pain. Revisit after the precision work lands.
- Introducing a `BlockSource` trait for extractors - pure aesthetics; pay
  the cost only when adding a sixth extractor.

## Next Action
- Commit the refactor.
- Start Stage 8 against the numeric targets in
  `.warden/plans/2026-05-13-next-stage-goals.md` (Stage 8 section). Headline
  minimums: heading accuracy >=11/21 combined, formula recall >=4/13
  combined, table recall floor 4/4. Precision metric extension to
  `pdfp eval` is a Stage 8 sub-deliverable and ships before any heading or
  formula tuning, otherwise recall gains can quietly destroy precision.
