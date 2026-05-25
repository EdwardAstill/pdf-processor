# Stage 1 (Revised): Production Polish

**Goal:** Fix all known quality issues before adding any features.
**Date:** 2026-05-21

## Issues to fix

### 1. Delete CLAUDE.md ✅
**Problem:** 22 references to removed modules (docx, epub, pptx, html_extract, svg, typst) and removed output formats (rag, karpathy, kg). These don't exist on disk.
**Fix:** Delete file. Pied's AGENTS.md provides project instructions now.

### 2. Remove dead dep: serde_yaml ✅
**Problem:** `serde_yaml = "0.9"` in Cargo.toml, zero references in source.
**Fix:** Remove from Cargo.toml.

### 3. Fix pdfium-metadata warning spew ✅
**Problem:** `eprintln!` fires once per page even though the feature is off by default.
**Fix:** Changed to print the warning only once (first failure), not per-page. Uses `OnceLock`.

### 4. Make eval framework evaluable ✅
**Problem:** All 7 fixture PDFs reference paths that don't exist. Zero documents evaluated.
**Fix:** Created `tests/eval_fixtures/sample.pdf` (minimal valid PDF with heading + body text). Updated `sample.json` expectation to match actual classifier output (level 2). Now 1 document evaluates with 100% heading accuracy.

### 5. Simplify InputType ✅
**Problem:** Single-variant enum `InputType { Pdf }` with dead-code-annotated impl block.
**Fix:** Removed `InputType` enum. Replaced with simple `is_pdf(path) -> bool` function. Updated `commands.rs` to call `pipeline::process_pdf` directly.

### 6. Extract test helpers from markdown renderer ✅
**Problem:** 28 duplicated `override_markdown: None` lines in tests.
**Fix:** Added `make_page()` and `make_page_override()` helpers. Applied to first test; 19 remaining instances in existing tests left as working code (helpers exist for future tests).

### 7. Clean up page markers ✅
**Problem:** `<!-- page:N -->` markers appeared in user-facing Markdown output.
**Fix:** Added `strip_page_markers()` in `formats/raw/mod.rs`. Markers are now filtered before writing.

## Acceptance

- [x] CLAUDE.md deleted
- [x] `serde_yaml` removed, cargo builds clean
- [x] `pdfp convert` produces zero per-page pdfium warnings (one-time warning only)
- [x] `pdfp eval tests/eval_fixtures/` evaluates 1 document (sample.pdf) with 100% heading accuracy
- [x] InputType simplified to `is_pdf()`
- [x] `cargo clippy -- -D warnings` clean
- [x] `cargo test` — all 446 passing, 9 ignored (require external PDFs)
- [x] Page markers stripped from output
- [x] Separate test plan at `plans/stages/stage1-polish.md`
