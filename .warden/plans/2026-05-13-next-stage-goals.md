# Next Stage Goals And Reconsolidation Plan

**Status:** recorded
**Created:** 2026-05-13
**Branch:** `stage-6`
**Current HEAD:** `8c2ac4f docs: reconsolidate next pdf quality stages`

## Task Restatement

Confirm whether the current branch is in the right place, reconcile the stage numbering, and set concrete goals for the next stages before more implementation work starts.

## Situation

The current branch is structurally in the right place: it is clean, pushed to `origin/stage-6`, and contains two coherent commits on top of `origin/main`:

- `ceb37ca feat: use tagged pdf structure when available`
- `3df5949 feat(eval): add quality evaluation command`

The reconsolidation issue is stage naming. The original roadmap's Task 7 was image/vector handling, but the actual branch inserted evaluation infrastructure as Stage 7. That was the right product move because the project needed a measurement gate before further heuristic work, but the roadmap now needs explicit alignment.

This file is the detailed next-stage contract. The roadmap in `docs/plans/2026-04-24-pdf-quality-roadmap.md` keeps the shorter public alignment note and should point back here rather than duplicate every acceptance item.

## Stage Goals

### Stage 7.5 - Baseline Consolidation

**Goal:** Turn `pdfp eval` from infrastructure into a useful regression gate.

**Scope:**

- Merge or PR-review `origin/stage-6` so the current work lands on `main`.
- Create ignored local PDFs under `test-corpus/eval/` or another ignored corpus path.
- Add tracked fixture JSON under `tests/eval_fixtures/` pointing to those corpus PDFs.
- Install/verify optional runtime dependencies:
  - `libpdfium` for tagged-PDF structure role extraction.
  - RapidLaTeX-OCR `encoder.onnx`, `decoder.onnx`, and `vocab.txt` only if native formula OCR quality is being evaluated.
- Record baseline scores for representative PDF classes.

**Acceptance:**

- `pdfp eval tests/eval_fixtures/` evaluates at least one real local PDF and skips missing corpus files cleanly.
- `pdfp eval <local-fixtures>` records current formula recall, heading accuracy, and table recall for at least one engineering report and one calculation PDF.
- `pdfp doctor` and a feature-enabled conversion run make runtime gaps explicit.

### Stage 8 - Measured Heading And Formula Recovery

**Goal:** Improve the weakest measured signals before investing in less-measured image/vector work.

**Reason:** The local engineering-report eval run measured table recall as good on sampled pages, but heading accuracy and display-formula recall were poor. That is better evidence than the old stage ordering.

**Scope:**

- Improve heading classification on numbered engineering/report headings such as `1 Introduction`, `1.1 Scope`, and appendix headings.
- Reduce formula false positives in page headers/front matter when `--formulas local` is forced.
- Improve display-formula recall for centred equation layouts that currently remain plain text.
- Keep table recall stable.

**Acceptance:**

- `pdfp eval <local-engineering-fixtures>` shows heading accuracy and formula recall improvement against the Stage 7.5 baseline.
- `cargo test`
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings`
- No regression in `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture`

### Stage 9 - Image And Vector Handling

**Goal:** Complete the original roadmap Task 7 after the text/formula/table baseline is stable.

**Scope:**

- Add configurable thresholds for tiny decorative image suppression.
- Suppress repeated edge/furniture images.
- Keep meaningful embedded figures.
- Investigate whether vector-heavy figures can be represented by rendered page-region snapshots rather than raw vector extraction.
- Document vector-only limits if reliable vector bounds are not exposed by the available PDF stack.

**Acceptance:**

- Existing front-matter image tests stay green.
- `cargo test render::markdown::tests::scholarly_front_matter_drops_decorative_images_and_keeps_captioned_figure`
- `cargo test --test golden -- --ignored golden_corpus_sweep` passes or skips only for missing local corpus fixtures.
- Eval/golden notes distinguish "missing vector representation" from "image extraction regression".

### Stage 10 - Release Polish

**Goal:** Ship the improved CLI with honest expectations.

**Scope:**

- Document quality matrix:
  - good: born-digital prose PDFs and many academic papers.
  - improving: tables, formulas, figures, tagged PDFs.
  - requires OCR/hybrid: scanned or damaged text-layer PDFs.
  - limited: missing ToUnicode maps, complex vector-only diagrams, bad or absent tags.
- Add final CLI examples for conversion, no-image conversion, OCR/hybrid, and eval.
- Ensure install instructions match where `pdfp` is actually installed on this machine (`~/.local/bin` vs `~/.cargo/bin`).

**Acceptance:**

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings`
- `git ls-files ':(glob)**/*.png' ':(glob)**/*.pdf' | wc -l` remains `0`
- README and `docs/CLI.md` include the quality matrix and eval workflow.

## Reconsolidation Checklist

- [ ] Open or merge PR for `origin/stage-6`; do not continue piling feature work onto a branch named after an older stage.
- [ ] Create a new branch for baseline work, suggested: `stage-7.5-baseline-eval-corpus`.
- [ ] Keep `stage-6` as the provenance branch for tagged PDF + eval infrastructure.
- [ ] Rename mental model: original roadmap Task 7 is now live Stage 9.
- [ ] Use `pdfp eval` results, not only visual inspection, to choose whether Stage 8 or Stage 9 should run next.

## Non-Goals

- Do not add tracked PDF files to git.
- Do not treat ONNX formula OCR as accepted until real RapidLaTeX-OCR model files have been tested.
- Do not make `libpdfium` mandatory; the tagged-PDF path should remain an optional improvement with graceful fallback.
- Do not release before the quality matrix states the current limits plainly.
