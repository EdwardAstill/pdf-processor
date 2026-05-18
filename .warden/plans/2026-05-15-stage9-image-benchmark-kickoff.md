# Stage 9 - Image Benchmark Kickoff

**Status:** completed
**Created:** 2026-05-15
**Branch:** `stage-8-heading-formula`

## Goal

Start testing harder PDFs and add image/figure extraction metrics to `pdfp eval`
before tuning image/vector heuristics.

## Assumptions

- `A1`: Eval can enable snapshot figure extraction without writing assets into
  the fixture corpus.
  - type: repo-state
  - source: `src/eval/runner.rs`
  - check: `cargo test --test eval_integration`
  - if false: keep image metrics unit-only until eval side effects are isolated.
  - owner: implementation
- `A2`: Harder local fixture PDFs can be referenced by JSON paths without
  committing binaries.
  - type: repo-state
  - source: local `example/pdf/` corpus
  - check: `target/debug/pdfp eval tests/eval_fixtures/`
  - if false: tracked JSON will skip missing PDFs, same as existing fixtures.
  - owner: fixture pass

## Tasks

1. Extend fixture schema and eval metrics for image/figure expectations.
   - skill: `warden:test-driven-development`
   - acceptance: `cargo test --test eval_integration image_metrics_count_decorative_suppression_and_caption_pairing` -> exits 0.

2. Enable media extraction during eval using a temp output directory.
   - skill: `warden:rust`
   - acceptance: `target/debug/pdfp eval tests/eval_fixtures/` -> reports image metrics and does not write images under `tests/eval_fixtures/`.

3. Add harder local fixture JSON files for academic/magazine PDFs.
   - skill: `warden:pdf-processing`
   - acceptance: `target/debug/pdfp eval tests/eval_fixtures/` -> evaluates available local hard fixtures or skips cleanly if absent.

4. Update docs and stage records with Stage 9 baseline numbers.
   - skill: `warden:writing`
   - acceptance: `rg -n "image|figure|decorative" docs tests/eval_fixtures .warden/plans/2026-05-13-next-stage-goals.md` -> exits 0.

5. Verify.
   - skill: `warden:verification-before-completion`
   - acceptance: `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings`, and `git diff --check` all exit 0.

## Kickoff Result

- Eval fixture schema now covers decorative images, meaningful figures,
  figure-caption pairs, vector-only regions, and text/table skip flags for
  image-only fixture pages.
- Eval conversion now writes snapshot figure assets under `/tmp/pdfp-eval-media`
  instead of the fixture directory.
- Hard local fixtures now cover:
  - `attention.pdf` pages 3 and 4: `2/2` meaningful figures and `2/2` caption
    pairs.
  - PDF/UA brochure pages 1 and 2: `3/3` meaningful figures.
  - `resnet.pdf` page 2: `1/1` meaningful figure, `1/1` caption pair, and
    `1/1` vector-only acknowledgement.
- Combined Stage 9 kickoff floor: `6/6` meaningful figure retention, `3/3`
  figure-caption pairing, and `1/1` vector-only acknowledgement. Decorative
  suppression is wired but still needs a non-zero labeled fixture page.

## Verification

- `cargo fmt --check` -> pass.
- `cargo run --quiet -- eval tests/eval_fixtures/` -> pass; evaluated 5
  documents and skipped the intentionally missing sample fixture.
- `cargo test` -> pass.
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings` -> pass.
- `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture` -> pass.
- `git diff --check` -> pass.
