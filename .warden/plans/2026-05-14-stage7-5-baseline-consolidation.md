# Stage 7.5: Baseline Consolidation

**Status:** implemented
**Created:** 2026-05-14
**Branch:** `stage-7.5-baseline-eval-corpus`
**Base:** `stage-6` at `2eeb1c0`

## Goal

Turn `pdfp eval` into a useful local regression gate by wiring it to real ignored PDFs and recording baseline scores before Stage 8 heuristic work.

## Assumptions

- `A1`
  - statement: representative local engineering-report and calculation PDFs exist outside this repo.
  - type: repo-state
  - source: `/home/eastill/projects/typst-template/*/example.pdf`
  - check: `pdfinfo test-corpus/eval/engineering-report-example.pdf` and `pdfinfo test-corpus/eval/engineering-calc-example.pdf`
  - if false: use another ignored local corpus path and update fixture JSON paths.
  - owner: Task 1
- `A2`
  - statement: `test-corpus/` is ignored and PDFs will not be tracked.
  - type: policy
  - source: `.gitignore`
  - check: `git check-ignore -v test-corpus/eval/engineering-report-example.pdf`
  - if false: fix ignore rules before copying PDFs.
  - owner: Task 1
- `A3`
  - statement: expected headings/formulas/tables are derived from the Typst source and PDF text, not current converter output.
  - type: design
  - source: `/home/eastill/projects/typst-template/engineering-report/example.typ` and `/home/eastill/projects/typst-template/engineering-calc/example.typ`
  - check: compare fixture JSON with source headings and display/calc math blocks.
  - if false: regenerate fixtures from source material before using scores as a baseline.
  - owner: Task 2

## Tasks

- [x] Task 1: Create ignored local corpus PDFs under `test-corpus/eval/`.
  - Acceptance: `git check-ignore -v test-corpus/eval/engineering-report-example.pdf` -> prints `.gitignore:15:/test-corpus/`.
- [x] Task 2: Add tracked eval fixture JSON for engineering report and calculation PDFs.
  - Acceptance: `cargo test --test eval_integration example_fixture_is_valid_json` -> pass.
- [x] Task 3: Record baseline scores and runtime dependency gaps.
  - Acceptance: `target/debug/pdfp eval tests/eval_fixtures/` -> evaluates local PDFs and skips missing sample.
- [x] Task 4: Update documentation and continuation artifact.
  - Acceptance: `rg "Stage 7.5" docs/TESTING.md tests/eval_fixtures/README.md .warden/NEXT.md` -> exits 0.

## Notes

- The baseline reveals current `pdfp eval` limits: table recall can be high while geometry detection marks whole pages as table regions, because Stage 7 metrics do not yet score table precision.
- Native ONNX formula quality remains unmeasured until RapidLaTeX-OCR model files are installed locally.
