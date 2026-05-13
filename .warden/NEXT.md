# Next Work

Updated: 2026-05-13 16:00
Branch: main
Remote: origin/main

## Current Goal
Continue staged PDF quality improvements after Stage 6 native ONNX formula OCR scaffold.

## Completed This Session
- Implemented Stage 6 `onnx-ocr` feature flag with native RapidLaTeX-OCR ONNX sidecar scaffold.
- Added image preprocessing, vocabulary loading, token decode helpers, ONNX session loading, and greedy decode loop.
- Wired `--formula-sidecar onnx:<model-dir>` and `cmd:<command>` while preserving bare command compatibility.
- Documented ONNX setup and added feature-gated integration tests.

## Changed Files
- `Cargo.toml` — optional `onnx-ocr` dependencies and `tempfile` dev dependency.
- `src/formula/ocr_onnx.rs` — native ONNX sidecar implementation.
- `src/formula/mod.rs`, `src/cli.rs`, `src/lib.rs`, `src/pipeline.rs` — feature gate, parser, exports, and dispatch.
- `tests/formula_onnx.rs` — feature-gated parser/preprocess/vocab/model-dir tests.
- `README.md`, `docs/CLI.md`, `docs/TESTING.md` — ONNX usage and test docs.
- `.warden/plans/2026-05-11-stage6-onnx-formula-ocr.md` — post-implementation review.

## Verification
- `cargo test` -> pass.
- `cargo test --features onnx-ocr` -> pass.
- `cargo clippy --all-targets -- -D warnings` -> pass.
- `cargo clippy --features onnx-ocr --all-targets -- -D warnings` -> pass.

## Blockers / Open Questions
- Real ONNX recognition was not run because RapidLaTeX-OCR model files are not present locally.
- `ort` `load-dynamic` failed on 2.0.0-rc.12 provider bindings; implementation uses `download-binaries`/`copy-dylibs`/`tls-rustls`.

## Next Action
- Start Stage 7 evaluation, including an ignored or local model-backed ONNX smoke once model files are available.
