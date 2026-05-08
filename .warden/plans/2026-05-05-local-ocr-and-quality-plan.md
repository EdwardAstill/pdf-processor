# Local OCR and PDF Quality Plan

status: in-progress
date: 2026-05-05
shape: research-plan-execute-review

## Task

Add a research-backed plan for local OCR, quality improvements, and simplification of overgrown PDF-to-Markdown logic, using current in-repo PDFs as a baseline.

## Research Inputs

- Viability report: `.warden/research/local-ocr-and-quality-plan/REPORT.md`
- Evidence ledger: `.warden/research/local-ocr-and-quality-plan/evidence.jsonl`
- Baseline report: `.warden/research/local-ocr-and-quality-plan/baseline/current-quality/report.json`
- Top-level baseline summary: `.warden/research/local-ocr-and-quality-plan/baseline/top-level-summary.json`
- Prior pipeline refactor report: `.warden/research/convert2-pdf-pipeline-refactor/REPORT.md`

## Assumptions

- id: A1
  statement: OCR should be optional and selective, not part of the default born-digital PDF path.
  type: architectural
  source: OCRmyPDF/PyMuPDF4LLM/Docling docs and local wiki.
  check: `rg -- '--ocr|ocr' README.md src/cli.rs src/pipeline.rs` after implementation shows OCR is flag-gated.
  if false: Rework CLI defaults and quality expectations before implementation.
  owner: Task 2

- id: A2
  statement: OCRmyPDF is the first provider because it produces searchable derivative PDFs and avoids direct OCR/PDF assembly in Rust.
  type: design
  source: OCRmyPDF docs and Tesseract docs.
  check: `rg 'OcrMyPdf|ocrmypdf' src/ocr src/cli.rs docs README.md` after implementation.
  if false: Write a separate design for direct Tesseract rasterization and PDF assembly before coding.
  owner: Task 2

- id: A3
  statement: OCR dependencies may be missing on developer machines and CI.
  type: external
  source: local probe: `ocrmypdf`, `tesseract`, and `qpdf` are absent here.
  check: `command -v ocrmypdf || true` in tests/docs.
  if false: Still keep detection; real OCR acceptance tests can run when tools are available.
  owner: Task 2

- id: A4
  statement: Existing example PDFs are enough for a first regression baseline.
  type: repo-state
  source: baseline run over `example/pdf`.
  check: `jq '.summary' .warden/research/local-ocr-and-quality-plan/baseline/top-level-summary.json`.
  if false: Add or fetch fixtures before implementation.
  owner: Task 1

## Behavior Contract

The default command remains deterministic and local:

```bash
pdfp input.pdf -o out/
```

Local OCR is opt-in:

```bash
pdfp input.pdf --ocr auto --ocr-lang eng -o out/
pdfp input.pdf --ocr force --ocr-lang eng+deu -o out/
```

Hybrid Docling remains separate:

```bash
pdfp input.pdf --hybrid docling --hybrid-policy auto -o out/
```

OCR preprocessing never mutates the input PDF. It writes a derived searchable PDF into a temp/cache directory, then runs the normal pipeline against that derived file.

## Task 1: Make the Baseline Reproducible

Block: execute
Skill: rust / writing
Status: completed

Notes:

- `scripts/quality-report.sh` now supports explicit top-level vs recursive corpus traversal through `PDFP_QUALITY_RECURSIVE`.
- `scripts/quality-diff.sh` compares a stored baseline with a fresh report.
- `docs/TESTING.md` documents the quality report schema and baseline workflow.

Instruction:

Update `scripts/quality-report.sh` and `docs/TESTING.md` so baseline runs can explicitly choose recursive vs top-level-only corpus traversal and write both a full report and a top-level/non-duplicate summary. Keep the existing skip-cleanly behavior when a corpus is missing.

Acceptance:

- `PDFP_QUALITY_CORPUS=example/pdf PDFP_QUALITY_RECURSIVE=0 PDFP_QUALITY_OUT=target/quality-top bash scripts/quality-report.sh` -> exits 0 and reports 22 total top-level PDFs.
- `PDFP_QUALITY_CORPUS=example/pdf PDFP_QUALITY_RECURSIVE=1 PDFP_QUALITY_OUT=target/quality-recursive bash scripts/quality-report.sh` -> exits 0 and reports 44 total PDFs.
- `cargo test --test quality` -> 0 failures.

## Task 2: Add Local OCR Sidecar

Block: execute
Skill: rust
Status: completed for missing-tool and fake-provider paths; live OCR still depends on local OCRmyPDF/Tesseract installation.

Notes:

- Added opt-in `--ocr off|auto|force`, language, cache, timeout, and command flags.
- OCR preprocessing writes a derived searchable PDF and never mutates the input.
- `--ocr auto` skips clean born-digital PDFs before checking for the OCR command.

Instruction:

Add `src/ocr/` with an OCR provider abstraction and an OCRmyPDF provider. Add CLI flags:

- `--ocr <off|auto|force>` default `off`
- `--ocr-lang <LANGS>` default `eng`
- `--ocr-cache-dir <DIR>`
- `--ocr-timeout-secs <N>` default `600`
- `--ocr-command <PATH>` default `ocrmypdf`

Integrate it as preprocessing in `pipeline::process_pdf`: run normal scan triage first, decide whether OCR is needed, produce a derived searchable PDF, then run the existing extraction/render path on the derived PDF while preserving output naming based on the original input.

Acceptance:

- `cargo test ocr` -> 0 failures, including command construction and missing-tool behavior.
- `pdfp example/pdf/golden__lorem.pdf --ocr auto -o target/ocr-lorem` with no OCR tool installed -> exits 0 and does not attempt OCR because the page is text-readable.
- `pdfp example/pdf/golden__chinese_scan.pdf --ocr auto -o target/ocr-scan` with no OCR tool installed -> exits non-zero with an actionable missing `ocrmypdf` message.
- If `ocrmypdf` and `tesseract` are installed: `pdfp example/pdf/golden__chinese_scan.pdf --ocr auto -o target/ocr-scan` -> output Markdown contains text beyond only an image reference.

## Task 3: Add OCR Cache and Provenance

Block: execute
Skill: rust
Status: completed

Notes:

- OCR cache keys include source metadata, mode, language, command, and timeout.
- Verbose output reports cache hits.
- `inspect --json` and `search --json` include OCR decision/provenance fields.

Instruction:

Cache derived OCR PDFs by source path, file metadata, OCR mode, language, provider command, and OCR options. Emit verbose messages for cache hits/misses. Store minimal provenance in stderr and, if a metadata field is added, in the rendered document metadata.

Acceptance:

- Running the same scan fixture twice with `--ocr-cache-dir target/ocr-cache --verbose` -> second run prints an OCR cache hit.
- Changing `--ocr-lang` changes the cache key.
- `cargo test ocr_cache` -> 0 failures.

## Task 4: Strengthen Scan and Encoding Triage

Block: execute
Skill: rust
Status: completed

Notes:

- Added OCR triage fields for readable pages, image-only pages, low-density pages, suspicious replacement characters, and pages needing OCR.
- Clean born-digital PDFs are not routed to OCR in `auto` mode.

Instruction:

Keep current block-density scan detection, then add explicit triage fields that can later support per-page OCR: text-readable page count, image-only page count, suspicious replacement-character count, low text-area fraction, and pages needing OCR. Do not route clean born-digital PDFs to OCR.

Acceptance:

- `cargo test hybrid::triage ocr::triage` -> 0 failures.
- `pdfp example/pdf/golden__chinese_scan.pdf -o target/no-ocr-scan 2>target/no-ocr-scan.stderr` -> stderr contains `scan-heavy`.
- `pdfp example/pdf/attention.pdf --ocr auto -o target/attention-ocr-auto --verbose 2>target/attention-ocr-auto.stderr` -> stderr does not contain `ocrmypdf`.

## Task 5: Move Table/Form Recovery Upstream

Block: execute
Skill: refactoring + rust
Status: pending; next deep refactor target.

Instruction:

Reduce `render/markdown.rs` responsibility by introducing a structure pass for numeric-heavy table rows and form/key-value regions before rendering. Start with the financial statement and invoice fixtures. Do not add more financial-document hacks directly in the renderer.

Acceptance:

- `cargo test layout::table render::markdown` -> 0 failures.
- `pdfp example/pdf/golden__issue-336-conto-economico-bialetti.pdf -o target/financial-after` -> no glued numeric row matching `[[:alpha:]][0-9][0-9][0-9]\\.` in the known financial table region.
- Existing ignored financial golden test still passes: `cargo test --test golden -- --ignored golden_snapshot_financial_statement_structure`.

## Task 6: Add Tagged-PDF / Metadata Pass

Block: execute
Skill: rust
Status: pending

Instruction:

Extend the optional `pdfium-metadata` path so tagged-PDF structure roles can influence headings, artifacts, lists, and tables before visual heuristics. Keep it feature-gated and skip cleanly when `libpdfium` is unavailable.

Acceptance:

- `cargo test --features pdfium-metadata` -> 0 failures or clear skip for missing runtime PDFium where applicable.
- `cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture` -> passes or skips only for missing fixture/runtime dependency.

## Task 7: Add Debug Artifacts

Block: execute
Skill: rust
Status: pending; should be considered during the deep refactor so the new structure pass is observable.

Instruction:

Add a debug output mode that emits JSON artifacts for page classification, reading order, table candidates, suppressed furniture/images, OCR decision, and final render block order. Do not require image overlays in the first version.

Acceptance:

- `pdfp example/pdf/golden__issue-336-conto-economico-bialetti.pdf --debug-artifacts target/debug-financial -o target/debug-financial-out` -> writes JSON debug artifacts.
- `jq empty target/debug-financial/*.json` -> exits 0.
- `cargo test debug_artifacts` -> 0 failures.

## Task 8: Documentation and Quality Matrix

Block: execute
Skill: writing
Status: completed in the OCR implementation batch.

Instruction:

Update `README.md` and `docs/TESTING.md` with honest capability boundaries, OCR dependency setup, baseline workflow, and examples for local OCR and hybrid Docling.

Acceptance:

- `rg '--ocr|OCRmyPDF|quality-report|scan-heavy|hybrid docling' README.md docs/TESTING.md` -> exits 0.
- `cargo fmt --check` -> exits 0.
- `cargo test` -> 0 failures.
- `cargo clippy --all-targets --all-features -- -D warnings` -> exits 0.

## Task 9: Integrate OCR With Inspect and Search

Block: execute
Skill: rust
Status: completed for missing-tool and fake-provider paths; live scan search requires installed OCR dependencies.

Notes:

- `inspect` can report OCR decision/provenance in JSON.
- `search` runs against the derived OCR PDF when OCR is requested and needed.
- Born-digital search does not invoke OCR in `auto` mode.

Instruction:

After the OCR sidecar lands, allow processor commands to use the same OCR preprocessing decision path for scan-heavy PDFs. `inspect` should be able to report OCR-assisted readability, and `search` should be able to search the derived searchable PDF when OCR is requested.

CLI:

```bash
pdfp search input.pdf "needle" --ocr auto
pdfp inspect input.pdf --ocr auto --json
```

Acceptance:

- Without OCR tools: `pdfp search example/pdf/golden__chinese_scan.pdf "text" --ocr auto` -> actionable missing-tool error only if OCR is actually needed.
- With OCR tools installed: `pdfp search example/pdf/golden__chinese_scan.pdf "<known OCR text>" --ocr auto --json` -> returns at least one match.
- Born-digital search does not invoke OCR in verbose stderr.
- `pdfp inspect input.pdf --ocr auto --json` includes enough OCR decision/provenance fields to explain whether OCR was skipped, attempted, or used.

## Review Gate

Before completion of any implementation batch:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
bash scripts/quality-report.sh
```

Expected:

- Formatting clean.
- Tests pass.
- Clippy clean.
- Quality report exits 0 and writes a report.

If OCR dependencies are not installed, OCR-specific live tests must be recorded as skipped unless the test explicitly checks missing-tool behavior.

## Implementation Batch Review: Baseline + OCR Sidecar

Date: 2026-05-05

Completed tasks:

- Task 1: reproducible quality baseline.
- Task 2: local OCR sidecar for convert.
- Task 3: OCR cache and JSON/stderr provenance.
- Task 4: OCR triage.
- Task 8: README/CLI/testing documentation for OCR and quality.
- Task 9: OCR-aware inspect/search.

Deferred tasks:

- Task 5: table/form recovery upstream refactor. This is the next deep refactor step.
- Task 6: tagged-PDF / metadata pass.
- Task 7: debug artifacts. This should be designed with Task 5 so table/form recovery is observable.

Verification evidence:

```bash
cargo fmt --check
# exit 0

cargo test --test ocr
# 5 passed, 0 failed

cargo test ocr
# OCR unit/integration filters passed

cargo test hybrid::triage
# 11 passed, 0 failed

cargo test ocr::triage
# 3 passed, 0 failed

cargo test
# 126 unit tests passed, CLI/help/golden/hybrid/OCR/quality integration tests passed, 0 failed

cargo clippy --all-targets --all-features -- -D warnings
# exit 0

PDFP_QUALITY_CORPUS=example/pdf PDFP_QUALITY_RECURSIVE=0 PDFP_QUALITY_OUT=target/quality-top bash scripts/quality-report.sh
# quality report: 22 total, 22 passed, 0 failed (top-level)

bash scripts/quality-report.sh
# SKIP missing corpus: /home/eastill/projects/pdf-processor/test-corpus
```

Acceptance evidence:

```bash
target/debug/pdfp example/pdf/golden__chinese_scan.pdf -o target/no-ocr-scan 2>target/no-ocr-scan.stderr
rg 'scan-heavy|--ocr auto' target/no-ocr-scan.stderr
# scan-heavy warning now suggests --ocr auto or --hybrid docling

target/debug/pdfp example/pdf/attention.pdf --ocr auto -o target/attention-ocr-auto --verbose 2>target/attention-ocr-auto.stderr
! rg 'ocrmypdf|OCRmyPDF command' target/attention-ocr-auto.stderr
# born-digital OCR auto path did not invoke OCRmyPDF

target/debug/pdfp inspect example/pdf/golden__lorem.pdf --ocr auto --ocr-command definitely-missing-pdfp-ocr-command --json | jq '.ocr.status == "skipped" and .ocr.mode == "auto"'
# true

target/debug/pdfp search example/pdf/golden__chinese_scan.pdf text --ocr auto --ocr-command definitely-missing-pdfp-ocr-command --json
# exits non-zero with actionable missing OCRmyPDF command message

rg -- '--ocr|OCRmyPDF|quality-report|scan-heavy|hybrid docling' README.md docs/TESTING.md
# exit 0
```

Known live-test gap:

- `command -v ocrmypdf` and `command -v tesseract` returned no path in this environment, so live OCR text recovery on `example/pdf/golden__chinese_scan.pdf` was not run. Missing-tool behavior and cache behavior were verified with tests/fake provider.
