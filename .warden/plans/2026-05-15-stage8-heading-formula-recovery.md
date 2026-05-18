# Stage 8 Heading And Formula Recovery

**Status:** implemented
**Created:** 2026-05-15
**Branch:** `stage-8-heading-formula`
**Base:** `89528d6` (`main`)

## Goal

Complete Stage 8 against the tracked engineering eval fixtures: add precision
metrics to `pdfp eval`, then recover measured numbered headings and display/calc
formula candidates without regressing table recall.

## Assumptions

- `A1`
  - statement: ignored local PDFs are available under `test-corpus/eval/`.
  - type: repo-state
  - source: user request and Stage 7.5 baseline.
  - check: `target/debug/pdfp eval tests/eval_fixtures/` evaluates two documents and skips only `sample.pdf`.
  - if false: copy from `/home/eastill/projects/typst-templates/.../example.pdf` before tuning.
  - owner: Task 1
- `A2`
  - statement: Stage 8 can improve local heuristics without requiring `libpdfium` or ONNX formula OCR.
  - type: architectural
  - source: Stage 8 acceptance and README runtime notes.
  - check: `cargo test` and `target/debug/pdfp eval tests/eval_fixtures/` pass on default features.
  - if false: limit Stage 8 to eval precision and document the dependency blocker.
  - owner: Tasks 2-4
- `A3`
  - statement: table recall must remain 4/4 while heading/formula thresholds change.
  - type: design
  - source: `.warden/plans/2026-05-13-next-stage-goals.md`.
  - check: eval report shows `table recall: 100.0%` for both engineering fixtures.
  - if false: revert the specific table-impacting heuristic and retune.
  - owner: Tasks 3-5

## Tasks

- [x] Task 1: Run current eval and inspect failed page content.
  - Skill: `warden:rust`
  - Acceptance: `target/debug/pdfp eval tests/eval_fixtures/` -> reports current heading/formula/table numbers.
- [x] Task 2: Extend `pdfp eval` with precision and false-positive counts.
  - Skill: `warden:rust`
  - Acceptance: `cargo test --test eval_integration` -> precision tests pass.
- [x] Task 3: Recover numbered engineering headings.
  - Skill: `warden:rust`
  - Acceptance: `target/debug/pdfp eval tests/eval_fixtures/` -> heading combined >= 11/21, report >= 7/13, calc >= 4/8.
- [x] Task 4: Recover display/calc formulas.
  - Skill: `warden:rust`
  - Acceptance: `target/debug/pdfp eval tests/eval_fixtures/` -> formula combined >= 4/13, report 1/1, calc >= 3/12.
- [x] Task 5: Update baseline notes and docs with measured precision.
  - Skill: `warden:writing`
  - Acceptance: `rg "precision" docs/TESTING.md tests/eval_fixtures/README.md .warden/research/stage7-5-baseline/BASELINE.md` -> exits 0.
- [x] Task 6: Verify Stage 8 floor.
  - Skill: `warden:verification-before-completion`
  - Acceptance: `cargo test` -> pass; feature clippy/golden attempted and result recorded.

## Notes

- `sample.json` intentionally points at a missing sample PDF and should remain a skip-path fixture.
- Precision is count-based for headings/formulas and page-region based for tables unless a stronger table-region annotation is added later.

## Verification

- `cargo fmt --check` -> pass.
- `target/debug/pdfp eval tests/eval_fixtures/` -> Stage 8 floor met:
  headings `17/21`, formulas `13/13`, tables `4/4`, heading FP `0`,
  formula FP `0`, table precision unchanged from precision baseline.
- `cargo test` -> pass.
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings` -> pass.
- `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture` -> pass.
- `git diff --check` -> pass.
