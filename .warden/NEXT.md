# Next Work

Updated: 2026-05-13 18:45 AWST
Branch: stage-6
Remote: origin/stage-6

## Current Goal
Reconsolidate staged PDF quality work after tagged-PDF support and Stage 7 evaluation infrastructure.

## Completed This Session
- Committed the completed tagged-PDF structure-tree path as `feat: use tagged pdf structure when available`.
- Added `pdfp eval <fixtures-dir>` for local fixture-based quality evaluation.
- Added fixture loading, page/document metrics, an eval runner that calls the full local pipeline, CLI dispatch, tests, sample fixture docs, and CLI/testing documentation.
- Verified default, `pdfium-metadata`, and `onnx-ocr` builds.

## Changed Files
- `src/eval/` - fixture schema, metrics, and full-pipeline eval runner.
- `src/pipeline.rs` - public `process_pdf_to_document()` entry point for in-process evaluation.
- `src/cli.rs`, `src/commands.rs`, `src/main.rs`, `src/lib.rs` - eval command wiring and library exports.
- `tests/eval_integration.rs`, `tests/eval_fixtures/`, `tests/cli_help.rs` - eval coverage and CLI smoke coverage.
- `README.md`, `docs/CLI.md`, `docs/TESTING.md` - documented the new eval command and fixture format.
- `.warden/plans/2026-05-11-stage7-evaluation.md` - post-implementation review.

## Verification
- `cargo fmt --check` -> pass.
- `cargo test` -> pass.
- `cargo test --features pdfium-metadata` -> pass.
- `cargo test --features onnx-ocr` -> pass.
- `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture` -> pass.
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings` -> pass.
- `target/debug/pdfp eval tests/eval_fixtures` -> gracefully skips missing sample PDF.

## Blockers / Open Questions
- Real ONNX formula recognition was not run because RapidLaTeX-OCR `encoder.onnx`, `decoder.onnx`, and `vocab.txt` are not present locally.
- The repository worktree has no local ignored PDF corpus, so live output comparison uses user-local PDFs outside the repo.

## Next Action
- Open or merge PR for `origin/stage-6`, then create a new branch for Stage 7.5 baseline consolidation.
- Use `.warden/plans/2026-05-13-next-stage-goals.md` as the next-stage contract.
