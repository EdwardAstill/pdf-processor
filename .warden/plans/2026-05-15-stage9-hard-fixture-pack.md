# Stage 9 - Hard Fixture Pack

**Status:** completed
**Created:** 2026-05-15
**Branch:** `stage-8-heading-formula`

## Goal

Add harder, locally reproducible fixtures so image/vector improvements and
regressions are visible in `pdfp eval` instead of hidden by the current small
hard-fixture set.

## Assumptions

- `A1`: Controlled synthetic PDFs are acceptable for fixture hardening when the
  source files are tracked and the generated PDF stays under ignored
  `test-corpus/`.
  - type: repo-state
  - source: existing ignored engineering eval PDFs.
  - check: `rg -n "test-corpus/eval" tests/eval_fixtures README.md`
  - if false: use only existing `example/pdf` corpus references.
  - owner: fixture generation
- `A2`: `typst` and `rsvg-convert` are available locally for reproducible
  fixture generation.
  - type: external
  - source: local command check.
  - check: `command -v typst && command -v rsvg-convert`
  - if false: keep the JSON fixture skipped until the PDF is supplied manually.
  - owner: fixture generation
- `A3`: Stage 9 should add measurement coverage before tuning figure heuristics.
  - type: design
  - source: user asked for harder examples to notice improvements/regressions.
  - check: `cargo run --quiet -- eval tests/eval_fixtures/`
  - if false: follow up with a heuristic tuning plan immediately.
  - owner: review

## Tasks

1. Add reproducible hard-image fixture sources.
   - block: execute
   - skill: `warden:typst`
   - instruction: Add Typst and SVG sources that generate a four-page hard
     image/vector PDF under `test-corpus/eval/`.
   - acceptance: `scripts/generate-eval-fixtures.sh stage9-hard-images` -> exits 0 and creates `test-corpus/eval/stage9-hard-images.pdf`.

2. Add eval expectations for the generated fixture.
   - block: execute
   - skill: `warden:pdf-processing`
   - instruction: Add a tracked JSON fixture pointing at the generated PDF with
     non-zero decorative, meaningful-figure, caption-pairing, and vector-only
     expectations.
   - acceptance: `cargo run --quiet -- eval tests/eval_fixtures/` -> includes `stage9-hard-images.pdf`.

3. Record the new Stage 9 benchmark interpretation.
   - block: execute
   - skill: `warden:writing`
   - instruction: Update fixture docs and next-stage notes so the hard pack is
     treated as the next improvement/regression signal.
   - acceptance: `rg -n "stage9-hard-images|decorative suppression" tests/eval_fixtures .warden/NEXT.md` -> exits 0.

4. Verify.
   - block: review
   - skill: `warden:verification-before-completion`
   - instruction: Run generation, eval, and diff checks before reporting the
     measured result.

## Result

- Added `scripts/generate-eval-fixtures.sh` with a `stage9-hard-images` target.
- Added tracked Typst/SVG sources under
  `tests/eval_fixtures/stage9_hard_images/`.
- Added `tests/eval_fixtures/stage9-hard-images.json`.
- Generated the ignored local PDF at
  `test-corpus/eval/stage9-hard-images.pdf`.
- The new fixture makes the current image/vector weaknesses visible: current
  decorative suppression is `1/2`, meaningful figure retention is `2/3`,
  figure-caption pairing is `2/3`, and vector-only acknowledgement is `0/1` on
  the generated hard pack.

## Verification

- `scripts/generate-eval-fixtures.sh stage9-hard-images` -> pass.
- `cargo run --quiet -- eval tests/eval_fixtures/` -> pass; evaluated 6
  documents and skipped the intentionally missing sample fixture.
- `bash -n scripts/generate-eval-fixtures.sh` -> pass.
- `git diff --check` -> pass.
