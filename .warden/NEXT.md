# Next Work

Updated: 2026-05-11 15:14
Branch: main
Remote: origin/main

## Current Goal
Continue staged PDF quality improvements after Stage 4 standards table detection.

## Completed This Session
- Implemented Stage 4 geometry-backed standards table detection.
- Integrated detected table regions into debug output, text suppression, and formula exclusion.
- Refactored geometry table row/layout shaping into `layout::table_detector`.
- Updated Stage 5-7 plans for current APIs and Stage 4 behavior.

## Changed Files
- `src/layout/drawing_ops.rs` — raster-backed horizontal/vertical line extraction.
- `src/layout/table_detector.rs` — line/grid/whitespace table region detection and region shaping.
- `src/pipeline.rs` — geometry table integration and debug schema update.
- `tests/table_detection.rs` — unit and DNV smoke coverage for Stage 4.
- `README.md`, `docs/CLI.md` — document rule-line geometry table detection.
- `.warden/plans/2026-05-11-stage5-inline-formatting.md` — note current API and block constructor adjustments.
- `.warden/plans/2026-05-11-stage6-onnx-formula-ocr.md` — note `image` dependency and sidecar compatibility.
- `.warden/plans/2026-05-11-stage7-evaluation.md` — require full-pipeline eval.

## Verification
- `cargo test` -> pass.
- `cargo clippy --all-targets -- -D warnings` -> pass.
- DNV page 69 table smoke -> pass in prior Stage 4 verification.

## Blockers / Open Questions
- `cargo fmt --check` still reports pre-existing rustfmt drift outside Stage 4 files.

## Next Action
- Start Stage 5 only after using the adjusted plan and current struct APIs.
