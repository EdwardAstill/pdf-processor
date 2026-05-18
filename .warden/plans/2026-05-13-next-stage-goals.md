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

**Benchmark Targets (vs Stage 7.5 baseline):**

Stage 7.5 baseline, anchored to the two tracked fixtures:

| metric | engineering-calc | engineering-report | combined |
|---|---|---|---|
| heading accuracy | 0/8 (0%) | 0/13 (0%) | 0/21 (0%) |
| formula recall | 0/12 (0%) | 0/1 (0%) | 0/13 (0%) |
| table recall | 1/1 (100%) | 3/3 (100%) | 4/4 (100%) |

Stage 8 acceptance numbers — must hit the minimum to claim Stage 8 complete; stretch is the bar before declaring Stage 8 "done well":

| metric | minimum | stretch | rationale |
|---|---|---|---|
| heading accuracy (combined) | >=11/21 (>=50%) | >=15/21 (>=70%) | Recover at least the numbered H1+H2 layer; stretch reaches into H3 clause-numbered headings. |
| heading accuracy (engineering-report) | >=7/13 | >=10/13 | The report has cleaner heading hierarchy than the calc sheet — easier wins live here. |
| heading accuracy (engineering-calc) | >=4/8 (H1+H2 only) | >=6/8 | The H3 "(AS 4100 §8.3.1)" parenthetical clause refs are hard; do not block stretch on them. |
| formula recall (combined) | >=4/13 (>=30%) | >=7/13 (>=53%) | Pick off display/centered equations first; inline calc terms can wait for Stage 9. |
| formula recall (engineering-report) | 1/1 | 1/1 | The single display equation must be findable; this is a high-confidence gate. |
| formula recall (engineering-calc) | >=3/12 | >=6/12 | Each `=` line in the clause checks is a candidate; aim for the obvious ones. |
| table recall (combined) | 4/4 (FLOOR — no regression) | 4/4 | Stage 8 must not drop any currently-detected table. |

**Required eval extension (Stage 8 sub-deliverable):**

The existing `pdfp eval` only measures recall. Before claiming Stage 8 acceptance:

- Extend `pdfp eval` to report precision alongside recall for headings, formulas, and tables.
- Baseline current precision on the tracked fixtures; record in `.warden/research/stage7-5-baseline/BASELINE.md` (append-only, do not rewrite history).
- Stage 8 must not regress precision below that newly-recorded baseline — specifically: heading false-positive count must not grow, formula false-positive count is allowed to grow by at most +1 across both fixtures (the existing front-matter false positive observed in baseline is the ceiling), table region precision must not drop.

Without the precision extension, heading and formula tuning can hit the recall targets above while quietly destroying output quality, so the precision metric ships first.

**Acceptance:**

- `pdfp eval <local-engineering-fixtures>` reports both recall and precision, and meets all minimum-column targets in the table above.
- The precision floor (heading FP <= baseline, formula FP <= baseline+1, table precision >= baseline) is preserved.
- `cargo test`
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings`
- No regression in `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture`

### Stage 8.5 - Table Precision And Candidate Refactor

**Goal:** Preserve the Stage 8 table recall floor while removing broad false-positive table regions before image/vector work depends on region boundaries.

**Reason:** Stage 8 kept table recall at `4/4`, but the new precision metric exposed weak table precision: engineering-calc is `1/2` table pages and engineering-report is `3/4` table pages. Debug output shows the false positives are broad `Layout` table candidates, not tight table regions. That should be fixed before Stage 9 starts figure/vector region handling.

**Benchmark Targets:**

Stage 8.5 keeps all Stage 8 heading/formula floors and adds a table precision recovery floor:

| metric | minimum | stretch | rationale |
|---|---|---|---|
| table recall (combined) | 4/4 | 4/4 | No regression from Stage 8. |
| table page precision (combined) | 4/4 on tracked evaluated pages | 4/4 plus no broad debug-only table regions on fixture non-table pages | Remove the two tracked false-positive table pages before downstream region work. |
| table region precision | metric ships first; baseline this stage | >=75% once expected boxes are recorded | Page-level precision is not enough to catch broad regions on true-table pages. |
| heading/formula floors | Stage 8 minimums hold | Stage 8 measured numbers hold | Table tuning must not undo Stage 8 gains. |

**Scope:**

- Add table candidate provenance/evidence fields for ruling grid, ruling band, text alignment/network, numeric rows, explicit regions, and optional external-model candidates.
- Centralize table scoring instead of mixing fixed geometry confidence with coordinate-table confidence.
- Replace first-seen overlap suppression with best-candidate arbitration.
- Drop or quarantine broad page-like layout candidates unless they have independent table evidence.
- Extend fixtures to optionally record expected table boxes and evaluate IoU-based table region precision.

**Acceptance:**

- `pdfp eval tests/eval_fixtures/` reports current Stage 8 heading/formula floors and table recall `4/4`.
- Table page precision on tracked evaluated pages improves from combined `4/6` to at least `4/4`.
- Eval can report table region precision when expected boxes are present.
- Broad page-like layout candidates are not emitted as normal table blocks on engineering-calc page 2 or engineering-report page 4.
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings`

**Measured Result (2026-05-15):**

- heading accuracy `17/21`, formula recall `13/13`, table page recall `4/4`
- table page precision `4/4`, improved from Stage 8 `4/6`
- table region recall `4/4` and table region precision `4/5` on recorded boxes
- engineering-calc page 2 and engineering-report page 4 emit no normal table blocks

### Stage 9 - Image And Vector Handling

**Goal:** Complete the original roadmap Task 7 after the text/formula/table baseline and table precision refactor are stable.

**Benchmark Targets:**

Stage 9 introduces image/vector metrics that were not captured by the original
`pdfp eval` command. The Stage 8 heading/formula numbers and Stage 8.5 table
recall/precision numbers become the FLOOR — Stage 9 must not regress any of
them.

New metrics to add to `pdfp eval` (Stage 9 sub-deliverable, ships first):

| metric | what it measures | initial target |
|---|---|---|
| decorative image suppression rate | suppressed decorative images / total decorative images present | baseline this stage, then >=80% on the tracked fixtures |
| meaningful figure retention rate | kept captioned/content figures / total meaningful figures | >=95% — almost no false suppression |
| figure-caption pairing rate | figures with caption attached / figures total | baseline this stage, then >=70% |
| vector-only region acknowledgement | vector-heavy regions tagged as "vector-only, snapshot-or-skip" / total such regions | baseline-only this stage; numeric target deferred |

Stage 9 kickoff measurement on 2026-05-15:

- `pdfp eval` now enables snapshot figure extraction into a temp output
  directory and reports decorative suppression, meaningful figure retention,
  figure-caption pairing, and vector-only acknowledgement when fixture
  expectations are present.
- Three harder local fixture JSON files were added for the ignored
  `example/pdf/` corpus:
  `attention.pdf`, `PDFUA-Ref-2-06_Brochure.pdf`, and vector-heavy
  `resnet.pdf`.
- Current non-zero image baseline is meaningful figure retention `6/6`,
  figure-caption pairing `3/3`, and vector-only acknowledgement `1/1` on the
  sampled hard pages.
- Decorative suppression is implemented in the metric layer but still has `0/0`
  tracked fixture expectations. Do not claim the decorative target until at
  least one deliberately labeled non-zero decorative page is added.

Stage 9 hard-fixture update on 2026-05-15:

- Added a reproducible generated fixture pack:
  `scripts/generate-eval-fixtures.sh stage9-hard-images`.
- The generated PDF is ignored at `test-corpus/eval/stage9-hard-images.pdf`;
  tracked Typst/SVG sources live under
  `tests/eval_fixtures/stage9_hard_images/`.
- Generated pack result: decorative suppression `1/2`, meaningful figure
  retention `2/3`, figure-caption pairing `2/3`, vector-only acknowledgement
  `0/1`.
- Combined hard image result with the kickoff fixtures: decorative suppression
  `1/2`, meaningful figure retention `8/9`, figure-caption pairing `5/6`,
  vector-only acknowledgement `1/2`.
- These are now the Stage 9 tuning signals. Do not use the kickoff-only `6/6`,
  `3/3`, `1/1` image numbers as release floors without the generated hard pack.

Targets above remain proposed for the full Stage 9 tuning pass. Re-anchor the
decorative target after the non-zero decorative fixture is added, then lock the
measured floors in `.warden/research/stage7-5-baseline/BASELINE.md` before
heuristic tuning.

**Scope:**

- Suppress uncaptioned decorative raster regions without losing captioned
  meaningful figures.
- Suppress repeated edge/furniture images.
- Keep meaningful embedded figures above the current hard-pack `8/9` combined
  baseline.
- Investigate why the generated vector-only candidate appears in debug JSON but
  is not acknowledged as a rendered figure block.
- Document vector-only limits if reliable vector bounds are not exposed by the
  available PDF stack.

**Acceptance:**

- Stage 8 and Stage 8.5 floors hold (heading/formula/table recall and precision do not regress).
- The new Stage 9 metrics ship and produce numbers on the tracked engineering
  fixtures plus harder local image fixtures.
- Meaningful figure retention stays at or above the kickoff baseline `6/6`,
  figure-caption pairing stays at or above `3/3`, and vector-only
  acknowledgement stays at or above `1/1` where expected.
- At least one non-zero decorative-image fixture is added before claiming
  decorative target performance.
- Stage 9 hits the targets above for decorative-image suppression and meaningful-figure retention.
- Existing front-matter image tests stay green.
- `cargo test render::markdown::tests::scholarly_front_matter_drops_decorative_images_and_keeps_captioned_figure`
- `cargo test --test golden -- --ignored golden_corpus_sweep` passes or skips only for missing local corpus fixtures.
- Eval/golden notes distinguish "missing vector representation" from "image extraction regression".

### Stage 10 - Release Polish

**Goal:** Ship the improved CLI with honest expectations.

**Benchmark Targets:**

Stage 10 sets no new numeric targets — it freezes the Stage 9 end-state and forces the documentation to match. Specifically:

- Every Stage 8, Stage 8.5, and Stage 9 metric is at or above its recorded acceptance number — this is a hard floor.
- The README quality matrix quotes the actual numbers from `pdfp eval` on the tracked fixtures, not aspirational labels. The "good / improving / requires OCR / limited" bands each carry the most recent measured numbers (heading accuracy %, formula recall %, table recall %).
- The public tool-comparison table stays honest: `docs/TOOL_COMPARISON.md`
  separates measured `pdfp` fixture numbers from unmeasured third-party
  capability claims, and does not claim broad parity until sidecar/API runs are
  measured on the same fixtures.
- `pdfp eval` exit code is non-zero when any fixture falls below its recorded floor — so CI / pre-release runs gate on the numbers rather than on a human eyeballing the output.

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
- README and `docs/CLI.md` include the quality matrix and eval workflow, with measured numbers from `pdfp eval`.
- `docs/TOOL_COMPARISON.md` includes current measured `pdfp` numbers, sourced
  external tool capabilities, and a list of still-unmeasured comparison runs.
- `pdfp eval tests/eval_fixtures/` returns non-zero on any regression below the Stage 9 floor.

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
