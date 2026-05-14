# Next Work

Updated: 2026-05-14 09:08 AWST
Branch: stage-7.5-baseline-eval-corpus
Remote: none

## Current Goal
Finish Stage 7.5 baseline consolidation so Stage 8 has measured heading/formula targets.

## Completed This Session
- Created branch/worktree `stage-7.5-baseline-eval-corpus` from `stage-6`.
- Copied two local Typst-generated PDFs into ignored `test-corpus/eval/`.
- Added tracked eval fixtures for the engineering report and engineering calculation PDFs.
- Recorded Stage 7.5 baseline scores in `.warden/research/stage7-5-baseline/BASELINE.md`.
- Documented the local baseline workflow in `docs/TESTING.md` and `tests/eval_fixtures/README.md`.

## Changed Files
- `tests/eval_fixtures/engineering-report.json` - expected headings/formulas/tables for the local engineering report fixture.
- `tests/eval_fixtures/engineering-calc.json` - expected headings/formulas/tables for the local calculation fixture.
- `tests/eval_fixtures/README.md` - local corpus setup notes.
- `tests/eval_integration.rs` - validates all tracked eval fixture JSON files.
- `docs/TESTING.md` - Stage 7.5 baseline workflow and current scores.
- `.gitignore` - ignores project-local `.worktrees/`.
- `.warden/plans/2026-05-14-stage7-5-baseline-consolidation.md` - implementation contract.
- `.warden/research/stage7-5-baseline/BASELINE.md` - baseline results and runtime gaps.
- `.warden/NEXT.md` - updated handoff.

## Verification
- `cargo fmt --check` -> pass.
- `cargo test --test eval_integration tracked_fixture_json_files_are_valid` -> pass.
- `cargo test` -> pass.
- `target/debug/pdfp eval tests/eval_fixtures/` -> evaluates 2 local documents, skips 1 missing sample.
- `git check-ignore -v .worktrees/example test-corpus/eval/engineering-report-example.pdf test-corpus/eval/engineering-calc-example.pdf` -> both local artifact paths ignored.
- `target/debug/pdfp doctor --json` -> reports OCR unavailable with actionable install hint.
- `cargo run --features pdfium-metadata -- convert test-corpus/eval/engineering-report-example.pdf -o target/stage7-5-pdfium-smoke --no-images --verbose` -> pass with explicit `libpdfium.so` missing fallback warnings.
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings` -> pass.

## Blockers / Open Questions
- Real ONNX formula recognition was not run because RapidLaTeX-OCR model files are not present locally.
- `libpdfium` is not visible in `ldconfig -p`, so tagged-PDF runtime extraction still needs a local dependency install before accepting `pdfium-metadata` quality claims.
- Table recall is currently 100% on the baseline fixtures, but debug output shows broad whole-page table regions; table precision is not yet measured by `pdfp eval`.

## Next Action
- Commit Stage 7.5.
- Start Stage 8: improve numbered engineering headings and display/calc formula recall against this baseline.
