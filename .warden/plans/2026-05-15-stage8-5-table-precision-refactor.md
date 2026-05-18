# Stage 8.5 - Table Precision And Candidate Refactor

**Status:** completed
**Created:** 2026-05-15
**Branch:** `stage-8-heading-formula`
**Source review:** `.warden/research/table-system-review/REPORT.md`

## Goal

Preserve Stage 8 table recall while removing broad false-positive table regions
and giving future image/vector work a clean region model.

## Why This Exists

Stage 8 recovered headings and formulas strongly, but table precision remains
weak despite perfect recall:

| fixture | recall | precision |
|---|---:|---:|
| engineering-calc | 1/1 | 1/2 |
| engineering-report | 3/3 | 3/4 |
| combined | 4/4 | 4/6 |

The false positives are broad `Layout` table regions, not tight table
detections. This is significant because Stage 9 image/vector work will also
depend on region boundaries.

## Scope

- Add table candidate provenance and evidence fields.
- Centralize table candidate scoring.
- Replace first-seen overlap suppression with best-candidate arbitration.
- Quarantine or drop broad page-like layout candidates unless supported by
  independent table evidence.
- Extend eval fixtures to optionally record expected table regions.
- Keep page-level table recall as a legacy floor while adding region precision.

## Acceptance

- Existing Stage 8 floors still hold:
  - headings combined >= 11/21, current target floor 17/21
  - formulas combined >= 4/13, current target floor 13/13
  - table recall 4/4
- Table page precision improves from current combined `4/6` to at least `4/4`
  on the tracked evaluated pages.
- Eval can report table region precision when expected boxes are present.
- Broad page-like layout candidates are not emitted as normal table blocks on
  calc page 2 or report page 4.
- `cargo fmt --check`
- `target/debug/pdfp eval tests/eval_fixtures/`
- `cargo test`
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings`

## Measured Result

Verified on 2026-05-15:

- heading accuracy `17/21`, formula recall `13/13`
- table page recall `4/4`, table page precision `4/4`
- table region recall `4/4`, table region precision `4/5`
- engineering-calc page 2 debug table count `0`
- engineering-report page 4 debug table count `0`

The remaining table-region false positive is engineering-calc page 1, where an
extra definitions/layout band is emitted on a true-table page. This is recorded
as the Stage 8.5 table-region precision floor for Stage 9.

## Non-Goals

- Do not make Docling, Camelot, Tabula, or Table Transformer mandatory.
- Do not attempt full cell-structure GriTS evaluation in this stage.
- Do not lower the Stage 8 heading/formula floors to make room for table work.

## Follow-On

After this stage, Stage 9 image/vector metrics can start with stable text,
formula, and table floors, including table precision rather than recall only.
