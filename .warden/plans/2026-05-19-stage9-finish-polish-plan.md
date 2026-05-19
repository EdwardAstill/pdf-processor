# Stage 9 Finish And Polish Plan

**Status:** completed  
**Created:** 2026-05-19  
**Branch:** `stage9-finish-polish`  
**Worktree:** `.worktrees/stage9-finish-polish`

## Goal

Finish Stage 9 image/vector extraction so the generated hard-image pack reaches
full decorative suppression, meaningful figure retention, caption pairing, and
vector-only acknowledgement without regressing existing text, formula, table,
or hard image floors. Then polish docs and release-readiness notes so source,
tests, and installed CLI expectations are explicit.

## Assumptions

- `A1`
  - statement: Stage 9 completion can be measured with the tracked generated
    hard-image fixture pack.
  - type: repo-state
  - source: `tests/eval_fixtures/stage9-hard-images.json` and existing Stage 9
    records.
  - check: `scripts/generate-eval-fixtures.sh stage9-hard-images && cargo run --quiet -- eval tests/eval_fixtures/`
  - if false: add or repair fixture generation before heuristic tuning.
  - owner: task 1
- `A2`
  - statement: Remaining Stage 9 failures are localized to figure filtering and
    caption-only vector-region estimation.
  - type: design
  - source: current debug output under `target/stage9-debug/`.
  - check: `target/debug/pdfp convert test-corpus/eval/stage9-hard-images.pdf -o target/stage9-debug --figures snapshot --debug-figures --figure-dpi 96`
  - if false: inspect pipeline merge/render ordering before changing figure
    heuristics.
  - owner: task 2
- `A3`
  - statement: Stage 8 and Stage 8.5 floors must remain stable while image
    heuristics change.
  - type: policy
  - source: `.warden/NEXT.md` and Stage 8/8.5 plan records.
  - check: `cargo test` plus `cargo run --quiet -- eval tests/eval_fixtures/`
  - if false: revert or narrow the image heuristic change.
  - owner: task 5
- `A4`
  - statement: Current installed `pdfp` is release-tag `0.3.0`; Stage 9 source
    should identify itself as a later unreleased package version, while install
    remains a separate release action unless explicitly requested.
  - type: repo-state
  - source: `pdfp --version`, `git describe --tags --always`, and earlier CLI
    comparison.
  - check: `pdfp --version && git describe --tags --always`
  - if false: update release/install guidance to match actual installed binary.
  - owner: task 4

## Tasks

1. Regenerate and baseline the hard Stage 9 fixture pack.
   - block: research
   - skill: `warden:rust`
   - instruction: Run fixture generation, eval, and debug conversion; record
     which pages leak decorative output or miss vector acknowledgement.

2. Tune figure detection/filtering for Stage 9 failures.
   - block: execute
   - skill: `warden:rust`
   - instruction: Update only the figure/render heuristics needed to suppress
     uncaptioned decorative banner candidates and choose nonblank caption-only
     vector regions near figure captions.
   - acceptance: `cargo test figure::detect::tests --bin pdfp` -> exits 0.
   - acceptance: `target/debug/pdfp convert test-corpus/eval/stage9-hard-images.pdf -o target/stage9-debug --figures snapshot --debug-figures --figure-dpi 96` -> page 1 markdown has no image, page 4 markdown has a retained figure with the Figure 3 caption.

3. Add focused regression tests.
   - block: execute
   - skill: `warden:rust`
   - instruction: Add tests that lock the decorative suppression rule and the
     vector-caption text-region estimate without relying on external PDFs.
   - acceptance: `cargo test figure::detect::tests --bin pdfp` -> includes the new tests and exits 0.

4. Polish docs and next-work notes.
   - block: execute
   - skill: `warden:writing`
   - instruction: Update fixture docs and `.warden/NEXT.md` with final Stage 9
     numbers, installed-CLI caveat, and remaining release/version-bump action.
   - acceptance: `rg -n "Stage 9|decorative suppression|installed CLI|version" tests/eval_fixtures/README.md .warden/NEXT.md` -> exits 0.

5. Verify complete Stage 9 finish.
   - block: review
   - skill: `warden:verification-before-completion`
   - instruction: Run formatting, eval, unit/integration tests, and diff checks
     before reporting completion.
   - acceptance: `cargo fmt --check` -> exits 0.
   - acceptance: `cargo run --quiet -- eval tests/eval_fixtures/` -> generated hard pack reports `2/2`, `3/3`, `3/3`, and `1/1` for Stage 9 image metrics, with available hard fixtures still passing.
   - acceptance: `cargo test` -> exits 0.
   - acceptance: `git diff --check` -> exits 0.

## Non-Goals

- Do not create or publish a GitHub release.
- Do not overwrite the user's installed CLI unless explicitly requested.
- Do not change Stage 8 heading/formula/table fixture expectations to hide a
  regression.

## Completion Criteria

- Generated hard-image fixture pack reaches:
  - decorative suppression `2/2`
  - meaningful figure retention `3/3`
  - figure-caption pairing `3/3`
  - vector-only acknowledgement `1/1`
- Previously passing hard image fixtures remain passing when local example PDFs
  are available.
- `cargo test` and formatting pass.
- Docs state what is complete in source and what still requires release/install
  work.

## Result

- Generated hard-image fixture pack is fully green: decorative suppression
  `2/2`, meaningful figure retention `3/3`, figure-caption pairing `3/3`, and
  vector-only acknowledgement `1/1`.
- Combined hard image fixtures are fully green when worktree `example/pdf` is
  linked from the main checkout: decorative suppression `2/2`, meaningful
  figure retention `9/9`, figure-caption pairing `6/6`, and vector-only
  acknowledgement `2/2`.
- Source package version is bumped to `0.3.1`; installed `pdfp 0.3.0` remains
  unchanged until release/install.

## Verification

- `cargo fmt --check` -> pass.
- `cargo run --quiet -- eval tests/eval_fixtures/` -> pass; 4 documents
  evaluated, 3 skipped because ignored engineering/sample PDFs are absent.
- `cargo test` -> pass.
- `cargo clippy --all-targets -- -D warnings` -> pass.
- `git diff --check` -> pass.
