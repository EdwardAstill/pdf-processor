# Next Work

Updated: 2026-05-19 AWST
Branch: stage9-finish-polish
Remote: origin/main at `07c3b26`

## Current Goal
Stage 9 image/vector handling is finished in source. The generated hard-image
fixture pack now reaches full decorative suppression, captioned figure
retention, figure-caption pairing, and vector-only acknowledgement while the
hard local image fixtures stay green. The package version is bumped to
`0.3.1`; the installed `pdfp` remains an older `0.3.0` binary until a release
or local install step updates it.

External-tool parity is now partially measured for deterministic peers.
`docs/TOOL_COMPARISON.md` puts `pdfp` first against PyMuPDF4LLM, Poppler
`pdftotext`, pdfminer.six, pdfplumber, Camelot, Tabula, and OCRmyPDF. The
current measured read is that PyMuPDF4LLM is the closest full Markdown peer,
table-only tools are useful but noisy, and OCRmyPDF still needs local `qpdf`
and `tesseract` before scan preprocessing can be measured. Broader cloud/ML
parity still requires same-fixture sidecar/API runs.

## Completed This Session
- Extended `pdfp eval` with precision and false-positive counts for formulas,
  headings, and table pages.
- Recovered engineering numbered headings by detecting undotted engineering
  numbering, splitting heading-first blocks, and recognizing the calculation
  sheet's canonical heading labels.
- Recovered formulas by ignoring broad layout-table regions as formula blockers,
  boosting compact centered display equations, and suppressing obvious inline
  prose formula false positives.
- Preserved table recall and did not increase heading/formula false positives.
- Recorded precision baseline and Stage 8 measured results in
  `.warden/research/stage7-5-baseline/BASELINE.md`.
- Reviewed the table pipeline against primary-source table extraction systems
  and inserted Stage 8.5 table precision/candidate refactor before Stage 9.
- Implemented Stage 8.5 table evidence scoring, caption-anchored table runs,
  best-candidate table overlap arbitration, weak layout-table quarantine, and
  IoU-based table-region eval metrics.
- Added Stage 9 image/figure metrics to `pdfp eval`, enabled snapshot figure
  extraction during eval in a temp output directory, and added harder fixture
  JSON files for `attention.pdf`, the PDF/UA brochure, and vector-heavy
  `resnet.pdf` from the local `example/pdf/` corpus.
- Added a sourced tool-comparison table focused on deterministic peers first,
  then broader Docling/Marker/MinerU/cloud comparators.
- Extended `scripts/sidecar-audit.sh` with deterministic comparator backends:
  Poppler `pdftotext -layout`, PyMuPDF4LLM, pdfplumber, pdfminer.six, Camelot,
  Tabula, and OCRmyPDF.
- Ran the deterministic peer comparison on both the engineering eval fixtures
  and the harder `example/pdf` sidecar corpus. Results are recorded in
  `docs/TOOL_COMPARISON.md`; raw outputs are under
  `target/sidecar-audit-eval-fixtures/` and
  `target/sidecar-audit-deterministic/`.
- Added a generated Stage 9 hard-image fixture pack from tracked Typst/SVG
  sources. It creates ignored `test-corpus/eval/stage9-hard-images.pdf` and
  measures non-zero decorative suppression, captioned figure retention, mixed
  decorative/meaningful pages, and vector-only acknowledgement.
- Finished Stage 9 tuning by suppressing uncaptioned decorative top banners,
  choosing nonblank caption-only vector regions from nearby diagram-label text,
  and suppressing duplicate caption/inner-label Markdown after a figure
  snapshot claims that content.
- Added focused regression tests for decorative banner suppression, vector-only
  caption text-region estimation, and duplicate figure-caption/inner-label
  Markdown suppression.
- Bumped source package version to `0.3.1` so Stage 9 builds are distinguishable
  from installed/released `0.3.0`.

## Stage 8 Benchmark Result
- engineering-calc: headings `8/8`, formulas `12/12`, tables `1/1`.
- engineering-report: headings `9/13`, formulas `1/1`, tables `3/3`.
- combined: headings `17/21`, formulas `13/13`, tables `4/4`.
- precision: heading FP `0`, formula FP `0`, table precision unchanged
  (calc `50%`, report `75%`).

## Stage 8.5 Benchmark Result
- engineering-calc: headings `8/8`, formulas `12/12`, table pages `1/1`,
  table page precision `1/1`, table region precision `1/2`.
- engineering-report: headings `9/13`, formulas `1/1`, table pages `3/3`,
  table page precision `3/3`, table region precision `3/3`.
- combined: headings `17/21`, formulas `13/13`, table recall `4/4`, table page
  precision `4/4`, table region precision `4/5`.
- calc page 2 and report page 4 emit no normal table blocks.

## Stage 9 Kickoff Benchmark Result
- hard image fixtures: `attention.pdf` pages 3/4, PDF/UA brochure pages 1/2,
  and vector-heavy `resnet.pdf` page 2.
- meaningful figure retention: `6/6` across hard image pages.
- figure-caption pairing: `3/3` where captions are expected.
- vector-only acknowledgement: `1/1`.
- engineering floors still hold: headings `17/21`, formulas `13/13`, table page
  recall/precision `4/4`, table region precision `4/5`.

## Stage 9 Hard Fixture Pack Result
- generated hard pack: `stage9-hard-images.pdf` from
  `scripts/generate-eval-fixtures.sh stage9-hard-images`.
- generated pack alone: decorative suppression `2/2`, meaningful figure
  retention `3/3`, figure-caption pairing `3/3`, vector-only acknowledgement
  `1/1`.
- combined hard image fixtures with the generated pack: decorative suppression
  `2/2`, meaningful figure retention `9/9`, figure-caption pairing `6/6`,
  vector-only acknowledgement `2/2`.
- Rendered Markdown polish: page 1 no longer emits the decorative banner as a
  figure; captioned pages render one caption copy; vector-only page 4 renders a
  nonblank figure snapshot and suppresses duplicated inner diagram labels.

## Version / Install State
- Source package version is `0.3.1`.
- Installed `/home/eastill/.local/bin/pdfp` was previously confirmed as
  `pdfp 0.3.0`; it does not expose current source `pdfp eval` behavior until
  rebuilt/installed.
- Release/install remains the next operational step if this branch should
  replace the user's CLI.

## Changed Files
- `.gitignore`
- `src/eval/fixtures.rs`
- `src/eval/metrics.rs`
- `src/eval/runner.rs`
- `tests/eval_integration.rs`
- `src/figure/detect.rs`
- `src/figure/render.rs`
- `src/render/markdown.rs`
- `Cargo.toml`
- `src/layout/classifier.rs`
- `src/formula/detect.rs`
- `src/pipeline/mod.rs`
- `src/pipeline/merge.rs`
- `src/layout/table.rs`
- `src/layout/table_detector.rs`
- `docs/CLI.md`
- `docs/TESTING.md`
- `tests/eval_fixtures/README.md`
- `tests/eval_fixtures/engineering-calc.json`
- `tests/eval_fixtures/engineering-report.json`
- `tests/eval_fixtures/hard-attention.json`
- `tests/eval_fixtures/hard-brochure.json`
- `tests/eval_fixtures/hard-resnet-vector.json`
- `tests/eval_fixtures/stage9-hard-images.json`
- `tests/eval_fixtures/stage9_hard_images/`
- `scripts/generate-eval-fixtures.sh`
- `.warden/research/stage7-5-baseline/BASELINE.md`
- `.warden/plans/2026-05-15-stage8-heading-formula-recovery.md`
- `.warden/research/table-system-review/REPORT.md`
- `.warden/plans/2026-05-15-stage8-5-table-precision-refactor.md`
- `.warden/plans/2026-05-15-stage9-image-benchmark-kickoff.md`
- `.warden/plans/2026-05-15-stage9-hard-fixture-pack.md`
- `.warden/plans/2026-05-19-stage9-finish-polish-plan.md`
- `.warden/plans/2026-05-15-pdf-tool-comparison-plan.md`
- `.warden/research/pdf-tool-comparison/`
- `docs/TOOL_COMPARISON.md`
- `scripts/sidecar-audit.sh`
- `.warden/plans/2026-05-13-next-stage-goals.md`

## Verification
- `cargo fmt --check` -> pass.
- `cargo run --quiet -- eval tests/eval_fixtures/` -> Stage 8.5 floors plus
  Stage 9 image kickoff baseline met; 5 documents evaluated, 1 intentionally
  missing sample skipped.
- `cargo test` -> pass.
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings` -> pass.
- `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture` -> pass.
- calc page 2 and report page 4 `--debug-tables` counts -> `0`.
- `PATH="$PWD/target/sidecar-tools/venv/bin:$PATH" PDFP_SIDECAR_CORPUS="$PWD/test-corpus/eval" PDFP_SIDECAR_OUT="$PWD/target/sidecar-audit-eval-fixtures" PDFP_SIDECAR_BACKENDS="native pdftotext-layout pymupdf4llm pdfplumber pdfminer camelot tabula ocrmypdf" PDFP_SIDECAR_FIXTURES="engineering-calc-example.pdf engineering-report-example.pdf" bash scripts/sidecar-audit.sh` -> pass; OCRmyPDF skipped because `qpdf` is missing.
- `PATH="$PWD/target/sidecar-tools/venv/bin:$PATH" PDFP_SIDECAR_OUT="$PWD/target/sidecar-audit-deterministic" PDFP_SIDECAR_BACKENDS="native pdftotext-layout pymupdf4llm pdfplumber pdfminer camelot tabula ocrmypdf" bash scripts/sidecar-audit.sh` -> pass; OCRmyPDF skipped because `qpdf` is missing.
- `scripts/generate-eval-fixtures.sh stage9-hard-images` -> pass.
- `target/debug/pdfp inspect test-corpus/eval/stage9-hard-images.pdf --json` -> pass; generated pack has 4 pages.
- `cargo test figure::detect::tests --bin pdfp` -> pass.
- `cargo test figure_snapshots_claim_duplicate_caption_and_inner_labels --bin pdfp` -> pass.
- `cargo run --quiet -- eval tests/eval_fixtures/` with worktree `example/pdf`
  linked -> hard image fixtures pass; engineering fixtures skip when ignored
  local PDFs are absent.
- `cargo test` -> pass.
- `cargo clippy --all-targets -- -D warnings` -> pass.
- `git diff --check` -> pass.

## Known Followups
- Table region precision still has one recorded false positive: calc page 1
  emits an extra definition/layout region on a true-table page. Page-level table
  precision is fixed at `4/4`, but Stage 9 should preserve the region baseline
  `4/5`.
- Table-system review recorded at
  `.warden/research/table-system-review/REPORT.md`, with detailed Stage 8.5
  plan at `.warden/plans/2026-05-15-stage8-5-table-precision-refactor.md`.
- Stage 9 source is complete, but installed CLI parity still needs an explicit
  install or release step.
- Tool parity still needs OCRmyPDF after OS dependencies are installed, plus
  same-fixture runs for Docling, Marker, MinerU, Mathpix, Adobe PDF Extract,
  LlamaParse, and Unstructured. The deterministic comparison is now measured,
  but the ML/cloud rows are still sourced capability mapping, not a leaderboard.
- Report heading stretch is not reached (`9/13` vs stretch `10/13`), but the
  Stage 8 minimum and combined stretch are met.
- Real ONNX formula recognition remains untested because RapidLaTeX-OCR model
  files are not installed locally.
